# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
# openSUSE Tumbleweed: build the rpms, then install them on a clean system.
# The stages run the scripts of this directory, as
# .github/workflows/packages.yml does.
#
# The packages alone, into dist/opensuse/:
#   docker build -f dev/containers/opensuse.Dockerfile --target packages \
#     --output dist/opensuse .

# The base image, pinned by digest. Renovate updates the tag and the digest;
# packages.yml reads the image from here.
FROM registry.opensuse.org/opensuse/tumbleweed:latest@sha256:1cf7a3b7b68f0027e88fb5b5a4b4a0282fb8c18d0e2ece77342248e8244d3e2e AS base

FROM base AS tools
COPY dev/containers/build-deps.sh /tmp/
RUN sh /tmp/build-deps.sh opensuse && zypper clean -a \
    && mkdir /out && chown builder /out

FROM tools AS build
COPY --chown=builder . /build
USER builder
WORKDIR /build
# The source archive, as a release makes it (meson dist, from a git
# checkout; it carries the crates in vendor/). This step needs the network.
RUN git init -q . && git add -A \
    && git -c user.name=dev -c user.email=dev@localhost commit -qm dev \
    && meson setup /tmp/dist -Dscreen=false -Dguard=false -Dkeyboard=false -Dgnome=false -Drotation=false -Dkde_shortcuts=false -Dkde=false -Dkde_rotation=false \
    && meson dist -C /tmp/dist --no-tests --formats gztar
RUN --network=none sh dev/containers/build.sh opensuse \
    /tmp/dist/meson-dist/asus-zenbook-duo-ux8406-*.tar.gz /out

FROM scratch AS packages
COPY --from=build /out/ /

FROM base AS install
# Tumbleweed does not package udev-hid-bpf, which the keyboard package needs;
# only the second-screen and guard packages are installed.
RUN --mount=type=bind,from=build,source=/out,target=/pkgs \
    --mount=type=bind,source=dev/containers,target=/scripts \
    sh /scripts/install.sh opensuse /pkgs \
    && zypper clean -a
