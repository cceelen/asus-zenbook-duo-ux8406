# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
# Alpine: build the apks, then install on a clean system.
#
# The packages alone, into dist/alpine/:
#   docker build -f dev/containers/alpine.Dockerfile --target packages \
#     --output dist/alpine .

# The base image, pinned by digest. Renovate updates the tag and the digest.
FROM docker.io/library/alpine:edge@sha256:020dfcbaaf4cc1078bf2d9c7ba31a8466e334061dcd2f248001d68f79e52c000 AS base

FROM base AS build
# alpine-sdk has abuild, fakeroot and the C toolchain. The build
# dependencies are installed here, as root: abuild is run with -d and
# installs nothing itself.
RUN apk add --no-cache alpine-sdk git abuild-meson cargo glib-dev meson nodejs \
        python3 samurai atools
# The packages are signed with a key made for this one build; apk has to trust
# it for the repository index to be written. Tests need an ordinary user.
RUN adduser -D builder \
    && addgroup builder abuild \
    && su builder -c 'abuild-keygen -a -n' \
    && find /home/builder -name '*.rsa.pub' -exec cp {} /etc/apk/keys/ \;
COPY --chown=builder . /src
USER builder
WORKDIR /src
# The source archive, as a release makes it (meson dist, from a git
# checkout; it carries the crates in vendor/). This step needs the network.
RUN git init -q . && git add -A \
    && git -c user.name=dev -c user.email=dev@localhost commit -qm dev \
    && meson setup /tmp/dist -Dscreen=false -Dguard=false -Dkeyboard=false -Dgnome=false -Drotation=false \
    && meson dist -C /tmp/dist --no-tests --formats gztar
# The packages, from that archive, without network. The APKBUILD names the
# archive of the GitHub release; abuild finds a file of that name in SRCDEST
# and does not download it. abuild is run with -d: it checks no dependencies
# and so needs no root. apkbuild-lint (atools) and abuild validate run first;
# a finding fails the build.
RUN --network=none set -eu; \
    mkdir -p /home/builder/work/pkg /home/builder/work/src /home/builder/out; \
    cp /tmp/dist/meson-dist/*.tar.gz /home/builder/work/src/; \
    cd /home/builder/work/pkg; \
    cp /src/packaging/alpine/* .; \
    SRCDEST=/home/builder/work/src abuild checksum; \
    apkbuild-lint APKBUILD; \
    abuild validate; \
    abuild -d -m -P /home/builder/work/repo -s /home/builder/work/src; \
    find /home/builder/work/repo -name '*.apk' -exec cp {} /home/builder/out/ \;

FROM scratch AS packages
COPY --from=build /home/builder/out/ /

FROM base AS install
# Alpine has no udev-hid-bpf, so there is no keyboard package.
RUN --mount=type=bind,from=build,source=/home/builder/out,target=/pkgs \
    apk add --no-cache --allow-untrusted /pkgs/asus-zenbook-duo-ux8406-second-screen-[0-9]*.apk \
        /pkgs/asus-zenbook-duo-ux8406-second-screen-udev-*.apk \
        /pkgs/asus-zenbook-duo-ux8406-tcc-guard-[0-9]*.apk \
        /pkgs/asus-zenbook-duo-ux8406-tcc-guard-openrc-*.apk \
        /pkgs/asus-zenbook-duo-ux8406-tcc-guard-doc-*.apk
# The init script is valid shell, and OpenRC reads it.
RUN sh -n /etc/init.d/asus-ux8406-tcc-guard \
    && apk add --no-cache --virtual .openrc openrc \
    && rc-service asus-ux8406-tcc-guard describe \
    && apk del .openrc
