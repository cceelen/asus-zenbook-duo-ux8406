#!/bin/sh
# The build tools of one distribution, and the user that builds: rpmbuild,
# makepkg and abuild run without root. Run as root, in a container of the
# distribution. The Dockerfiles (stage tools) and
# .github/workflows/packages.yml run it.
#
#   build-deps.sh fedora|opensuse|debian|arch|alpine
set -eu

# A mirror can fail to send some files; the package manager then stops and
# changes nothing. retry starts the command again, two times, after a pause.
retry() { "$@" || { sleep 30; "$@"; } || { sleep 60; "$@"; }; }

case "${1:?which distribution}" in
  fedora)
    retry dnf install -y rpm-build git cargo clang libbpf-devel meson ninja-build \
      python3 systemd-devel systemd-rpm-macros kernel-headers glib2 nodejs rpmlint
    useradd -m builder
    ;;
  opensuse)
    retry zypper -n install rpm-build git cargo clang libbpf-devel meson ninja \
      python3 systemd-devel systemd-rpm-macros linux-glibc-devel gzip tar \
      glib2-tools nodejs-default rpmlint
    useradd -m builder
    ;;
  debian)
    retry apt-get update -q
    retry apt-get install -qy --no-install-recommends build-essential debhelper \
      ca-certificates git fakeroot meson ninja-build pkgconf systemd-dev udev \
      cargo rustc clang libbpf-dev linux-libc-dev python3 libglib2.0-bin \
      nodejs lintian
    useradd -m builder
    ;;
  arch)
    # rust, not cargo: two Arch packages provide cargo (rust, rustup), and
    # pacman without a person to ask cannot choose.
    retry pacman -Syu --noconfirm --needed base-devel git meson rust clang \
      libbpf linux-api-headers python glib2 nodejs systemd namcap
    useradd -m builder
    ;;
  alpine)
    retry apk add alpine-sdk git abuild-meson cargo glib-dev meson nodejs \
      python3 samurai atools
    # The packages are signed with a key made for this one build; apk has to
    # trust it to install them.
    adduser -D builder
    addgroup builder abuild
    su builder -c 'abuild-keygen -a -n'
    find /home/builder -name '*.rsa.pub' -exec cp {} /etc/apk/keys/ \;
    ;;
  *)
    echo "unknown distribution: $1" >&2
    exit 2
    ;;
esac
