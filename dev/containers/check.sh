#!/bin/sh
# The check of an installation, in the image of one distribution
# (dev/containers/<distro>.Dockerfile): the installed files, the programs,
# and the removal. compose.yaml and .github/workflows/packages.yml run it.
#
#   check.sh fedora|opensuse|debian|arch|alpine
#
# KEYBOARD and KDE (yes or no, default no) say which of those packages
# the image has installed.
set -eu

packages="asus-zenbook-duo-ux8406-second-screen asus-zenbook-duo-ux8406-tcc-guard"
if [ "${KEYBOARD:-no}" = yes ]; then
  packages="$packages asus-zenbook-duo-ux8406-keyboard-bpf"
fi
if [ "${KDE:-no}" = yes ]; then
  packages="$packages asus-zenbook-duo-ux8406-kde-shortcuts asus-zenbook-duo-ux8406-kde-keys plasma-builtin-screen-rotation"
fi
HELPER=/usr/libexec/asus-ux8406-second-screen

# shellcheck disable=SC2086 # $packages is a list of words.
case "${1:?which distribution}" in
  fedora)
    verify() { rpm --verify $packages; }
    remove() { dnf remove -y $packages; }
    ;;
  opensuse)
    verify() { rpm --verify $packages; }
    remove() { rpm --erase $packages; }
    ;;
  debian)
    # dpkg --verify reports what differs and exits 0 all the same.
    verify() { out=$(dpkg --verify $packages); echo "$out"; test -z "$out"; }
    remove() { dpkg --purge $packages; }
    ;;
  arch)
    verify() { pacman -Qkk $packages; }
    remove() { pacman -Rns --noconfirm $packages; }
    HELPER=/usr/lib/asus-ux8406-second-screen
    ;;
  alpine)
    # Alpine splits the udev rule, the OpenRC script and the documentation
    # off. apk has no verify for installed files: every file the packages
    # list has to be there.
    packages="asus-zenbook-duo-ux8406-second-screen asus-zenbook-duo-ux8406-second-screen-udev asus-zenbook-duo-ux8406-tcc-guard asus-zenbook-duo-ux8406-tcc-guard-openrc asus-zenbook-duo-ux8406-tcc-guard-doc"
    verify() { for f in $(apk info -Lq $packages); do test -e "/$f"; done; }
    remove() { apk del $packages; }
    ;;
  *)
    echo "unknown distribution: $1" >&2
    exit 2
    ;;
esac

verify
echo "installed files verified"
"$HELPER" --version
asus-ux8406-tcc-guard --version
if [ "${KEYBOARD:-no}" = yes ]; then
  udev-hid-bpf --version
  udev-hid-bpf inspect \
    /usr/lib/firmware/hid/bpf/0010-ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.o \
    | grep -c '"pid"' | grep -qx 3
  echo "udev-hid-bpf reads the keyboard program"
  # The helper that keeps the keyboard's state, and its rule: before
  # the rule of udev-hid-bpf, and with the helper's own path.
  "${HELPER%/*}/asus-ux8406-keyboard-state" --version
  grep -c "\"${HELPER%/*}/asus-ux8406-keyboard-state " \
    /usr/lib/udev/rules.d/80-asus-ux8406-keyboard-state.rules | grep -qx 2
  "${HELPER%/*}/asus-ux8406-keyboard-state" load 0003:0B05:1BF2.0001
  echo "keyboard state helper in place"
else
  echo "keyboard package not installed: no udev-hid-bpf here"
fi
if [ "${KDE:-no}" = yes ]; then
  test -f /usr/share/kglobalaccel/asus-zenbook-duo-ux8406-shortcuts.desktop
  "${HELPER%/*}/asus-ux8406-kde-keys" --version
  # The autostart entries name the programs by their installed paths.
  grep -qx "Exec=${HELPER%/*}/asus-ux8406-kde-keys" /etc/xdg/autostart/asus-ux8406-kde-keys.desktop
  test -f /usr/share/kwin/scripts/asus-zenbook-duo-ux8406-keys/metadata.json
  test -f /usr/share/kwin/scripts/asus-zenbook-duo-ux8406-keys/contents/code/main.js
  plasma-builtin-screen-rotation --version
  grep -qx "Exec=/usr/bin/plasma-builtin-screen-rotation run" \
    /etc/xdg/autostart/plasma-builtin-screen-rotation.desktop
  test -f /usr/share/plasma/plasmoids/io.github.cceelen.builtin-screen-rotation/metadata.json
  test -f /usr/share/plasma/plasmoids/io.github.cceelen.builtin-screen-rotation/contents/ui/main.qml
  echo "KDE Plasma files in place"
else
  echo "KDE Plasma packages not installed here"
fi
# Removal: the package manager runs the removal scripts, and no file of
# the packages stays.
remove
for f in "$HELPER" "${HELPER%/*}/asus-ux8406-keyboard-state" \
    /usr/lib/udev/rules.d/80-asus-ux8406-keyboard-state.rules \
    /usr/bin/asus-ux8406-tcc-guard /usr/sbin/asus-ux8406-tcc-guard \
    /usr/lib/systemd/system/asus-ux8406-tcc-guard.service /etc/init.d/asus-ux8406-tcc-guard \
    /usr/lib/udev/rules.d/90-asus-ux8406-second-screen.rules \
    /usr/lib/udev/hwdb.d/82-hid-bpf-asus-ux8406.hwdb \
    /usr/lib/firmware/hid/bpf/0010-ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.o \
    /usr/share/kglobalaccel/asus-zenbook-duo-ux8406-shortcuts.desktop \
    "${HELPER%/*}/asus-ux8406-kde-keys" \
    /etc/xdg/autostart/asus-ux8406-kde-keys.desktop \
    /usr/share/kwin/scripts/asus-zenbook-duo-ux8406-keys \
    /usr/bin/plasma-builtin-screen-rotation \
    /etc/xdg/autostart/plasma-builtin-screen-rotation.desktop \
    /usr/share/plasma/plasmoids/io.github.cceelen.builtin-screen-rotation; do
  if [ -e "$f" ]; then echo "left after removal: $f"; exit 1; fi
done
echo "packages removed, nothing left"
echo "OK"
