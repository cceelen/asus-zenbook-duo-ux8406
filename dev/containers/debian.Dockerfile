# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
# Debian: build the debs, then install them on a clean system. The stages run
# the scripts of this directory, as .github/workflows/packages.yml does.
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

# The base images, pinned by digest. Renovate updates the digests; packages.yml
# pins its own images for the CI builds. BuildKit builds only the stage that RELEASE selects.
FROM docker.io/library/debian:trixie@sha256:2dc419c6d7fe3dad0a2f9fd469c34cc90b0f9ba642d8fde837fe3d7d34a0e7f1 AS base-trixie
FROM docker.io/library/debian:testing@sha256:cf15c569a84dfda832e421ab27a8049e962ba8e90571612396cd8db799279e4e AS base-testing

FROM base-${RELEASE} AS tools
COPY dev/containers/build-deps.sh /tmp/
# git: for meson dist below.
RUN sh /tmp/build-deps.sh debian && apt-get install -qy --no-install-recommends git ca-certificates && rm -rf /var/lib/apt/lists/* \
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
RUN --network=none sh dev/containers/build.sh debian \
    /tmp/dist/meson-dist/asus-zenbook-duo-ux8406-*.tar.gz /out

FROM scratch AS packages
COPY --from=build /out/ /

FROM base-${RELEASE} AS install
ARG KEYBOARD=no
RUN --mount=type=bind,from=build,source=/out,target=/pkgs \
    --mount=type=bind,source=dev/containers,target=/scripts \
    KEYBOARD=$KEYBOARD sh /scripts/install.sh debian /pkgs \
    && rm -rf /var/lib/apt/lists/*
