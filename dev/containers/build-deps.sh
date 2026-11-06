#!/bin/sh
# The build tools of one distribution, and the user that builds: rpmbuild,
# makepkg and abuild run without root (useradd, and su from util-linux,
# which the smallest images leave out). Run as root, in a container of the
# distribution. The Dockerfiles (stage tools) and
# .github/workflows/packages.yml run it. Only what the build needs: no weak
# dependencies or recommends.
#
#   build-deps.sh fedora|rhel|opensuse|debian|arch|alpine
set -eu

# A mirror can fail to send some files; the package manager then stops and
# changes nothing. retry starts the command again, two times, after a pause.
retry() { "$@" || { sleep 30; "$@"; } || { sleep 60; "$@"; }; }

case "${1:?which distribution}" in
  fedora|rhel)
    if [ "$1" = rhel ]; then
      # EPEL and CodeReady Builder have the build tools that RHEL lacks.
      retry dnf install -y --setopt=install_weak_deps=False epel-release dnf-plugins-core
      dnf config-manager --set-enabled crb
    fi
    retry dnf install -y --setopt=install_weak_deps=False rpm-build cargo clang \
      libbpf-devel meson ninja-build python3 systemd-devel systemd-rpm-macros \
      kernel-headers glib2 nodejs rpmlint shadow-utils util-linux
    useradd -m builder
    ;;
  opensuse)
    retry zypper -n install --no-recommends rpm-build cargo clang libbpf-devel \
      meson ninja python3 systemd-devel systemd-rpm-macros linux-glibc-devel \
      gzip tar glib2-tools nodejs-default rpmlint shadow util-linux
    useradd -m builder
    ;;
  debian)
    retry apt-get update -q
    retry apt-get install -qy --no-install-recommends build-essential debhelper \
      fakeroot meson ninja-build pkgconf systemd-dev udev \
      cargo rustc clang libbpf-dev linux-libc-dev python3 libglib2.0-bin \
      nodejs lintian
    useradd -m builder
    ;;
  arch)
    # rust, not cargo: two Arch packages provide cargo (rust, rustup), and
    # pacman without a person to ask cannot choose.
    retry pacman -Syu --noconfirm --needed base-devel meson rust clang \
      libbpf linux-api-headers python glib2 nodejs systemd namcap
    useradd -m builder
    ;;
  alpine)
    retry apk add alpine-sdk abuild-meson cargo glib-dev meson nodejs \
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
