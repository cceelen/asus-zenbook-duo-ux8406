# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
# Arch: build the packages, then install them on a clean system. The stages
# run the scripts of this directory, as .github/workflows/packages.yml does.
#
# The packages alone, into dist/arch/:
#   docker build -f dev/containers/arch.Dockerfile --target packages \
#     --output dist/arch .

# The base image, pinned by digest. Renovate updates the tag and the digest;
# packages.yml pins its own images for the CI builds.
FROM docker.io/library/archlinux:latest@sha256:e82623936fa1a69700b48e98b058df6eaab18b737c27473a7234a40fdecb9413 AS base

FROM base AS tools
COPY dev/containers/build-deps.sh /tmp/
# git: for meson dist below.
RUN sh /tmp/build-deps.sh arch && pacman -S --noconfirm --needed git && pacman -Scc --noconfirm \
    && mkdir /out && chown builder /out

FROM tools AS build
COPY --chown=builder . /src
USER builder
WORKDIR /src
# The source archive, as a release makes it (meson dist, from a git
# checkout; it carries the crates in vendor/). This step needs the network.
RUN git init -q . && git add -A \
    && git -c user.name=dev -c user.email=dev@localhost commit -qm dev \
    && meson setup /tmp/dist -Dscreen=false -Dguard=false -Dkeyboard=false -Dgnome=false -Drotation=false -Dkde_shortcuts=false -Dkde=false -Dkde_rotation=false \
    && meson dist -C /tmp/dist --no-tests --formats gztar
RUN --network=none sh dev/containers/build.sh arch \
    /tmp/dist/meson-dist/asus-zenbook-duo-ux8406-*.tar.gz /out

FROM scratch AS packages
COPY --from=build /out/ /

FROM base AS install
RUN --mount=type=bind,from=build,source=/out,target=/pkgs \
    --mount=type=bind,source=dev/containers,target=/scripts \
    KEYBOARD=yes GNOME=yes KDE=yes sh /scripts/install.sh arch /pkgs \
    && pacman -Scc --noconfirm
