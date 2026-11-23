# syntax=docker/dockerfile:1@sha256:572cebbc2b15e6f84c077742952674889cb63cd5e9b80abdffcbe587b6f19183
# openSUSE Tumbleweed: build the rpms, then install them on a clean system.
# The stages run the scripts of this directory, as
# .github/workflows/packages.yml does.
#
# The packages alone, into dist/opensuse/:
#   docker build -f dev/containers/opensuse.Dockerfile --target packages \
#     --output dist/opensuse .

# The base image, pinned by digest, from Docker Hub: the openSUSE registry
# deletes the old digests of its rolling images within days. Renovate updates
# the digest; packages.yml pins its own images for the CI builds.
FROM docker.io/opensuse/tumbleweed:latest@sha256:3e4ba7f382638c643eb5805d959637d9a59ede4f640fe9407106493fc6ac9ede AS base

FROM base AS tools
COPY dev/containers/build-deps.sh /tmp/
# git: for meson dist below.
RUN sh /tmp/build-deps.sh opensuse && zypper -n install --no-recommends git-core gawk && zypper clean -a \
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
