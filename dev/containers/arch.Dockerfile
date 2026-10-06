# syntax=docker/dockerfile:1
# Arch: build the packages, then install them on a clean system.
#
# The packages alone, into dist/arch/:
#   docker build -f dev/containers/arch.Dockerfile --target packages \
#     --output dist/arch .

FROM docker.io/library/archlinux:latest AS build
# The makedepends of the PKGBUILD, and git for the source archive.
# udev-hid-bpf is needed at run time only.
RUN pacman -Syu --noconfirm --needed base-devel git meson rust clang libbpf \
        linux-api-headers python glib2 nodejs systemd namcap \
    && pacman -Scc --noconfirm
# makepkg refuses root, and the tests are about file permissions.
RUN useradd -m builder && mkdir /out && chown builder /out
COPY --chown=builder . /src
USER builder
WORKDIR /src
# The source archive, as a release makes it (meson dist, from a git
# checkout; it carries the crates in vendor/). This step needs the network.
RUN git init -q . && git add -A \
    && git -c user.name=dev -c user.email=dev@localhost commit -qm dev \
    && meson setup /tmp/dist -Dscreen=false -Dguard=false -Dkeyboard=false -Dgnome=false \
    && meson dist -C /tmp/dist --no-tests --formats gztar
# The packages, from that archive, without network. --nodeps: udev-hid-bpf,
# a run-time dependency, is not in the official repositories. namcap checks the
# PKGBUILD and every package; it has no exit status for findings, so an error
# line (tagged ' E: ') fails the build, and so does a warning (' W: ') that is
# not in packaging/arch/namcap-accepted.txt. The rule splitpkgmakedeps is left out:
# it wants every run-time dependency of the split packages as a makedepends,
# and two of them cannot be: udev-hid-bpf is not in the official repositories
# (makepkg -s would fail), and gnome-shell is a whole desktop that the build
# does not use.
RUN --network=none set -eu; \
    mkdir /home/builder/work; cd /home/builder/work; \
    cp /src/packaging/arch/* /tmp/dist/meson-dist/*.tar.gz .; \
    makepkg --noconfirm --nodeps --nocolor; \
    cp *.pkg.tar.zst /out/; \
    namcap -e splitpkgmakedeps PKGBUILD > /tmp/namcap.txt 2>&1; \
    for p in *.pkg.tar.zst; do namcap "$p" >> /tmp/namcap.txt 2>&1; done; \
    cat /tmp/namcap.txt; \
    if grep -q ' E: ' /tmp/namcap.txt; then echo 'namcap reported errors' >&2; exit 1; fi; \
    grep -v '^#' namcap-accepted.txt > /tmp/accepted.txt; \
    if grep ' W: ' /tmp/namcap.txt | grep -v -x -F -f /tmp/accepted.txt; then \
        echo 'namcap reported warnings that are not accepted' >&2; exit 1; fi

FROM scratch AS packages
COPY --from=build /out/ /

FROM docker.io/library/archlinux:latest AS install
# The image leaves out /usr/share/doc and /usr/share/man when installing; the
# guard's example configuration and manual page are there.
RUN sed -i 's# usr/share/doc/\*##; s#usr/share/man/\* ##' /etc/pacman.conf
RUN --mount=type=bind,from=build,source=/out,target=/pkgs \
    pacman -Syu --noconfirm \
    && pacman -U --noconfirm /pkgs/asus-zenbook-duo-ux8406-second-screen-*.pkg.tar.zst \
        /pkgs/asus-zenbook-duo-ux8406-tcc-guard-*.pkg.tar.zst \
        /pkgs/asus-zenbook-duo-ux8406-keyboard-bpf-*.pkg.tar.zst \
        /pkgs/gnome-shell-extension-asus-zenbook-duo-ux8406-keys-*.pkg.tar.zst \
    && pacman -Scc --noconfirm
