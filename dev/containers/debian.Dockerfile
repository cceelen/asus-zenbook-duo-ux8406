# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
# Debian: build the debs, then install them on a clean system.
#
# The packages alone, into dist/debian/:
#   docker build -f dev/containers/debian.Dockerfile --target packages \
#     --output dist/debian .
#
# RELEASE is the Debian release (trixie, the default, is Debian 13; testing
# also builds). KEYBOARD=yes installs the keyboard package in the install
# stage too, which needs udev-hid-bpf, not in Debian 13:
#   docker build -f dev/containers/debian.Dockerfile --target install \
#     --build-arg RELEASE=testing --build-arg KEYBOARD=yes .
ARG RELEASE=trixie

# The base images, pinned by digest. Renovate updates the digests. BuildKit
# builds only the stage that RELEASE selects.
FROM docker.io/library/debian:trixie@sha256:913f6706df59a68922d1dd08f78c2476560a8d367897200a6005b00e5f67c2d5 AS base-trixie
FROM docker.io/library/debian:testing@sha256:f35963177a0584848d5222b3d2b4f4ebcc6434d39db0a709c3088237ef619cbf AS base-testing

FROM base-${RELEASE} AS build
RUN apt-get update -q \
    && apt-get install -qy --no-install-recommends build-essential debhelper \
        ca-certificates git fakeroot meson ninja-build pkgconf systemd-dev udev \
        cargo rustc clang libbpf-dev linux-libc-dev python3 libglib2.0-bin \
        nodejs lintian \
    && rm -rf /var/lib/apt/lists/*
# The tests of the guard need an ordinary user: root ignores file permissions.
RUN useradd -m builder
COPY --chown=builder . /src
USER builder
WORKDIR /src
# The source archive, as a release makes it (meson dist, from a git
# checkout; it carries the crates in vendor/). This step needs the network.
RUN git init -q . && git add -A \
    && git -c user.name=dev -c user.email=dev@localhost commit -qm dev \
    && meson setup /tmp/dist -Dscreen=false -Dguard=false -Dkeyboard=false -Dgnome=false -Drotation=false \
    && meson dist -C /tmp/dist --no-tests --formats gztar
# The packages, from that archive, without network, and lintian on the result
# (source package and all binary packages, via the .changes file): every tag is
# shown, and an error fails the build.
RUN --network=none set -eu; \
    version=$(dpkg-parsechangelog -l packaging/debian/changelog -S Version | cut -d- -f1); \
    mkdir /home/builder/work /home/builder/out; cd /home/builder/work; \
    cp /tmp/dist/meson-dist/asus-zenbook-duo-ux8406-$version.tar.gz \
        asus-zenbook-duo-ux8406_$version.orig.tar.gz; \
    tar -xzf asus-zenbook-duo-ux8406_$version.orig.tar.gz; \
    cd asus-zenbook-duo-ux8406-$version; cp -r /src/packaging/debian debian; \
    dpkg-buildpackage -us -uc -rfakeroot; \
    cd /home/builder/work; \
    lintian --info --display-info --pedantic --fail-on error \
        asus-zenbook-duo-ux8406_$version-*_amd64.changes; \
    find /home/builder/work -maxdepth 1 -type f -exec cp {} /home/builder/out/ \;

FROM scratch AS packages
COPY --from=build /home/builder/out/ /

FROM base-${RELEASE} AS install
# No systemd runs here; the policy layer keeps invoke-rc.d from trying to
# start the guard, as on any container image.
RUN printf '#!/bin/sh\nexit 101\n' > /usr/sbin/policy-rc.d \
    && chmod +x /usr/sbin/policy-rc.d
# udev-hid-bpf, which the keyboard package needs, is in testing and unstable
# but not in Debian 13 (trixie). The extension package is built but not
# installed here: it would pull in a whole GNOME.
ARG KEYBOARD=no
RUN --mount=type=bind,from=build,source=/home/builder/out,target=/pkgs \
    apt-get update -q \
    && if [ "$KEYBOARD" = yes ]; then \
           set -- /pkgs/asus-zenbook-duo-ux8406-second-screen_*.deb /pkgs/asus-zenbook-duo-ux8406-tcc-guard_*.deb \
               /pkgs/asus-zenbook-duo-ux8406-keyboard-bpf_*.deb; \
       else set -- /pkgs/asus-zenbook-duo-ux8406-second-screen_*.deb /pkgs/asus-zenbook-duo-ux8406-tcc-guard_*.deb; fi \
    && apt-get install -qy "$@" \
    && rm -rf /var/lib/apt/lists/*
