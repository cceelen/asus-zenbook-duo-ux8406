# syntax=docker/dockerfile:1
# openSUSE Tumbleweed: build the rpms from the same spec as Fedora, then
# install on a clean system.
#
# The packages alone, into dist/opensuse/:
#   docker build -f dev/containers/opensuse.Dockerfile --target packages \
#     --output dist/opensuse .

# The base image, pinned by digest. Renovate updates the tag and the digest.
FROM registry.opensuse.org/opensuse/tumbleweed:latest@sha256:3906bb6ea95dca6992ec37c360c75403f79331b11670c1972645390f031327a0 AS base

FROM base AS build
RUN zypper -n install rpm-build git cargo clang libbpf-devel meson ninja python3 \
        systemd-devel systemd-rpm-macros linux-glibc-devel gzip tar glib2-tools \
        nodejs-default rpmlint \
    && zypper clean -a
# The tests refuse to run as root, so everything from here on is the
# unprivileged user's.
RUN useradd -m builder \
    && mkdir -p /build /rpm /out \
    && chown -R builder:builder /build /rpm /out
COPY --chown=builder:builder . /build
USER builder
WORKDIR /build
# The source archive, as a release makes it (meson dist, from a git
# checkout; it carries the crates in vendor/). This step needs the network.
RUN git init -q . && git add -A \
    && git -c user.name=dev -c user.email=dev@localhost commit -qm dev \
    && meson setup /tmp/dist -Dscreen=false -Dguard=false -Dkeyboard=false -Dgnome=false -Drotation=false \
    && meson dist -C /tmp/dist --no-tests --formats gztar
# The packages, from that archive, as a user without root and without network.
RUN --network=none rpmbuild --define "_topdir /rpm" \
        --define "_sourcedir /tmp/dist/meson-dist" \
        -ba packaging/rpm/asus-zenbook-duo-ux8406.spec \
    && cp /rpm/RPMS/*/*.rpm /rpm/SRPMS/*.rpm /out/
# rpmlint on the spec and on every rpm built above, binary and source. Its
# full output is printed; the build fails on any error (rpmlint's own exit
# status depends on a badness threshold that differs per distribution) and
# on any other failure of rpmlint.
COPY --chown=builder:builder packaging/rpm/ /build/packaging/rpm/
RUN rc=0; \
    rpmlint -r /build/packaging/rpm/asus-zenbook-duo-ux8406.rpmlintrc \
        /build/packaging/rpm/asus-zenbook-duo-ux8406.spec /out/*.rpm \
        > /tmp/rpmlint.log 2>&1 || rc=$?; \
    cat /tmp/rpmlint.log; \
    if grep -q ': E: ' /tmp/rpmlint.log; then echo "rpmlint: errors"; exit 1; fi; \
    if [ "$rc" -ne 0 ] && [ "$rc" -ne 64 ]; then echo "rpmlint failed: $rc"; exit 1; fi

FROM scratch AS packages
COPY --from=build /out/ /

FROM base AS install
# Tumbleweed does not package udev-hid-bpf, which the keyboard package needs;
# only the second-screen and guard packages are installed. The extension
# package is not installed either: it would pull in a whole GNOME.
RUN --mount=type=bind,from=build,source=/out,target=/pkgs \
    zypper -n install --allow-unsigned-rpm /pkgs/asus-zenbook-duo-ux8406-second-screen-*.rpm \
        /pkgs/asus-zenbook-duo-ux8406-tcc-guard-*.rpm \
    && zypper clean -a
