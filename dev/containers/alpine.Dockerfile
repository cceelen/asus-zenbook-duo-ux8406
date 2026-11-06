# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
# Alpine: build the apks, then install them on a clean system. The stages run
# the scripts of this directory, as .github/workflows/packages.yml does.
#
# The packages alone, into dist/alpine/:
#   docker build -f dev/containers/alpine.Dockerfile --target packages \
#     --output dist/alpine .

# The base image, pinned by digest. Renovate updates the tag and the digest;
# packages.yml reads the image from here.
FROM docker.io/library/alpine:edge@sha256:020dfcbaaf4cc1078bf2d9c7ba31a8466e334061dcd2f248001d68f79e52c000 AS base

FROM base AS tools
COPY dev/containers/build-deps.sh /tmp/
RUN sh /tmp/build-deps.sh alpine && rm -rf /var/cache/apk/* \
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
RUN --network=none sh dev/containers/build.sh alpine \
    /tmp/dist/meson-dist/asus-zenbook-duo-ux8406-*.tar.gz /out

FROM scratch AS packages
COPY --from=build /out/ /

FROM base AS install
# The packages are signed with the key of the build stage; the install takes
# them without it (--allow-untrusted).
RUN --mount=type=bind,from=build,source=/out,target=/pkgs \
    --mount=type=bind,source=dev/containers,target=/scripts \
    sh /scripts/install.sh alpine /pkgs
