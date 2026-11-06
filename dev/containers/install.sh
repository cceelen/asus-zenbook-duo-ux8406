#!/bin/sh
# Install the packages that build.sh made on a clean system of the same
# distribution, with its package manager: it gets the run-time dependencies
# from the repositories of the distribution, so a dependency that a recipe
# forgot fails here. Weak dependencies and recommends are not installed: the
# packages must work with what they require. Run as root. check.sh then
# checks the installation.
#
#   install.sh fedora|rhel|opensuse|debian|arch|alpine DIR
#
# KEYBOARD, GNOME and KDE (yes or no, default no) say which of those packages
# to install, as for check.sh.
set -eu

family=${1:?which distribution}
dir=$(realpath "${2:?which directory}")
retry() { "$@" || { sleep 30; "$@"; } || { sleep 60; "$@"; }; }

parts="second-screen tcc-guard"
[ "${KEYBOARD:-no}" = yes ] && parts="$parts keyboard-bpf"
gnome=no kde=no
[ "${GNOME:-no}" = yes ] && gnome=yes
[ "${KDE:-no}" = yes ] && kde=yes

# shellcheck disable=SC2086 # $files is a list of words.
case "$family" in
  fedora|rhel|opensuse)
    files=
    for p in $parts; do files="$files $dir/asus-zenbook-duo-ux8406-$p-[0-9]*.x86_64.rpm"; done
    [ "$gnome" = yes ] && files="$files $dir/gnome-shell-extension-asus-zenbook-duo-ux8406-keys-[0-9]*.noarch.rpm $dir/gnome-shell-extension-builtin-screen-rotation-[0-9]*.noarch.rpm"
    [ "$kde" = yes ] && files="$files $dir/asus-zenbook-duo-ux8406-kde-shortcuts-[0-9]*.noarch.rpm $dir/asus-zenbook-duo-ux8406-kde-keys-[0-9]*.x86_64.rpm $dir/plasma-builtin-screen-rotation-[0-9]*.x86_64.rpm"
    case "$family" in
      fedora)
        retry dnf install -y --setopt=install_weak_deps=False $files
        ;;
      rhel)
        # udev-hid-bpf and parts of the desktops are in EPEL.
        retry dnf install -y --setopt=install_weak_deps=False epel-release
        retry dnf install -y --setopt=install_weak_deps=False $files
        ;;
      opensuse)
        retry zypper -n install --no-recommends --allow-unsigned-rpm $files
        ;;
    esac
    ;;
  debian)
    # Services are not started, as on any container image.
    printf '#!/bin/sh\nexit 101\n' > /usr/sbin/policy-rc.d
    # The Ubuntu image leaves out /usr/share/doc and /usr/share/man when it
    # installs; the licence notices, the guard's example configuration and
    # its manual page are there.
    rm -f /etc/dpkg/dpkg.cfg.d/excludes
    chmod +x /usr/sbin/policy-rc.d
    files=
    for p in $parts; do files="$files $dir/asus-zenbook-duo-ux8406-${p}_*.deb"; done
    retry apt-get update -q
    retry apt-get install -qy --no-install-recommends $files
    ;;
  arch)
    # The image leaves out /usr/share/doc and /usr/share/man when it
    # installs; the guard's example configuration and manual page are there.
    sed -i 's# usr/share/doc/\*##; s#usr/share/man/\* ##' /etc/pacman.conf
    files=
    for p in $parts; do files="$files $dir/asus-zenbook-duo-ux8406-$p-[0-9]*.pkg.tar.zst"; done
    [ "$gnome" = yes ] && files="$files $dir/gnome-shell-extension-asus-zenbook-duo-ux8406-keys-[0-9]*.pkg.tar.zst $dir/gnome-shell-extension-builtin-screen-rotation-[0-9]*.pkg.tar.zst"
    [ "$kde" = yes ] && files="$files $dir/asus-zenbook-duo-ux8406-kde-shortcuts-[0-9]*.pkg.tar.zst $dir/asus-zenbook-duo-ux8406-kde-keys-[0-9]*.pkg.tar.zst $dir/plasma-builtin-screen-rotation-[0-9]*.pkg.tar.zst"
    retry pacman -Syu --noconfirm
    retry pacman -U --noconfirm $files
    ;;
  alpine)
    # Alpine splits the udev rule, the OpenRC script and the documentation
    # off.
    retry apk add --allow-untrusted \
      "$dir"/asus-zenbook-duo-ux8406-second-screen-[0-9]*.apk \
      "$dir"/asus-zenbook-duo-ux8406-second-screen-udev-*.apk \
      "$dir"/asus-zenbook-duo-ux8406-tcc-guard-[0-9]*.apk \
      "$dir"/asus-zenbook-duo-ux8406-tcc-guard-openrc-*.apk \
      "$dir"/asus-zenbook-duo-ux8406-tcc-guard-doc-*.apk
    # The OpenRC script parses, and OpenRC reads it.
    sh -n /etc/init.d/asus-ux8406-tcc-guard
    retry apk add --virtual .openrc openrc
    rc-service asus-ux8406-tcc-guard describe
    apk del .openrc
    ;;
  *)
    echo "unknown distribution: $family" >&2
    exit 2
    ;;
esac
