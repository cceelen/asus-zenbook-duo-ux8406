# syntax=docker/dockerfile:1
# Alpine: build the apks, then install on a clean system.
#
# The packages alone, into dist/alpine/:
#   docker build -f dev/containers/alpine.Dockerfile --target packages \
#     --output dist/alpine .
ARG RELEASE=edge

FROM docker.io/library/alpine:${RELEASE} AS build
# alpine-sdk has abuild, fakeroot and the C toolchain. The build
# dependencies are installed here, as root: abuild is run with -d and
# installs nothing itself.
RUN apk add --no-cache alpine-sdk git abuild-meson cargo clang glib-dev libbpf-dev \
        linux-headers meson nodejs python3 samurai atools
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
    && meson setup /tmp/dist -Dscreen=false -Dguard=false -Dkeyboard=false -Dgnome=false \
    && meson dist -C /tmp/dist --no-tests --formats gztar
# The packages, from that archive, without network. abuild is run with -d:
# it checks no dependencies and so needs no root. apkbuild-lint (atools) and
# abuild validate run first; a finding fails the build.
RUN --network=none set -eu; \
    mkdir -p /home/builder/work/pkg /home/builder/out; cd /home/builder/work/pkg; \
    cp /src/packaging/alpine/* /tmp/dist/meson-dist/*.tar.gz .; \
    abuild checksum; \
    apkbuild-lint APKBUILD; \
    abuild validate; \
    abuild -d -m -P /home/builder/work/repo -s /home/builder/work/src; \
    find /home/builder/work/repo -name '*.apk' -exec cp {} /home/builder/out/ \;

FROM scratch AS packages
COPY --from=build /home/builder/out/ /

FROM docker.io/library/alpine:${RELEASE} AS install
# Alpine does not package udev-hid-bpf, which the keyboard package needs; only
# the second-screen and guard packages can be installed.
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
