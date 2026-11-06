# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
# openSUSE Tumbleweed: build the rpms from the same spec as Fedora, then
# install on a clean system.
#
# The packages alone, into dist/opensuse/:
#   docker build -f dev/containers/opensuse.Dockerfile --target packages \
#     --output dist/opensuse .

# The base image, pinned by digest. Renovate updates the tag and the digest.
FROM registry.opensuse.org/opensuse/tumbleweed:latest@sha256:1cf7a3b7b68f0027e88fb5b5a4b4a0282fb8c18d0e2ece77342248e8244d3e2e AS base

# The build tools and the build user: the only stage that the Actions cache
# keeps. Each layer from the source on is new in each run.
FROM base AS tools
RUN zypper -n install rpm-build git cargo clang libbpf-devel meson ninja python3 \
        systemd-devel systemd-rpm-macros linux-glibc-devel gzip tar \
        nodejs-default dbus-1-daemon rpmlint \
    && zypper clean -a
# The tests refuse to run as root, so everything from here on is the
# unprivileged user's.
RUN useradd -m builder \
    && mkdir -p /build /rpm /out \
    && chown -R builder:builder /build /rpm /out
FROM tools AS build
COPY --chown=builder:builder . /build
USER builder
WORKDIR /build
# The source archive, as a release makes it (meson dist, from a git
# checkout; it carries the crates in vendor/). This step needs the network.
RUN git init -q . && git add -A \
    && git -c user.name=dev -c user.email=dev@localhost commit -qm dev \
    && meson setup /tmp/dist -Dscreen=false -Dguard=false -Dkeyboard=false -Dkde_shortcuts=false -Dkde=false -Dkde_rotation=false \
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
# only the second-screen and guard packages are installed.
RUN --mount=type=bind,from=build,source=/out,target=/pkgs \
    zypper -n install --allow-unsigned-rpm /pkgs/asus-zenbook-duo-ux8406-second-screen-*.rpm \
        /pkgs/asus-zenbook-duo-ux8406-tcc-guard-*.rpm \
    && zypper clean -a
