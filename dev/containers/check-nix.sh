#!/bin/sh
# The check of the Nix build: Nix has no installation step here; ROOT holds
# a link to the build result of each part (ROOT/second-screen, ...). The
# image of dev/containers/nix.Dockerfile has them in /, the nix job of
# .github/workflows/packages.yml in results/.
#
#   check-nix.sh [ROOT]
set -eu
root=$(realpath "${1:-/}")
root=${root%/}

ls -l "$root"/second-screen/libexec/asus-ux8406-second-screen "$root"/second-screen/lib/udev/rules.d/90-asus-ux8406-second-screen.rules \
  "$root"/keyboard-bpf/lib/firmware/hid/bpf/0010-ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.o \
  "$root"/keyboard-bpf/lib/udev/hwdb.d/82-hid-bpf-asus-ux8406.hwdb \
  "$root"/keyboard-bpf/libexec/asus-ux8406-keyboard-state \
  "$root"/keyboard-bpf/lib/udev/rules.d/80-asus-ux8406-keyboard-state.rules \
  "$root"/gnome-keys/share/gnome-shell/extensions/asus-zenbook-duo-ux8406-keys@cceelen.github.io/extension.js \
  "$root"/gnome-rotation/share/gnome-shell/extensions/builtin-screen-rotation@cceelen.github.io/extension.js \
  "$root"/kde-shortcuts/share/kglobalaccel/asus-zenbook-duo-ux8406-shortcuts.desktop \
  "$root"/kde-keys/libexec/asus-ux8406-kde-keys "$root"/kde-keys/etc/xdg/autostart/asus-ux8406-kde-keys.desktop \
  "$root"/kde-keys/share/kwin/scripts/asus-zenbook-duo-ux8406-keys/contents/code/main.js \
  "$root"/kde-rotation/bin/plasma-builtin-screen-rotation \
  "$root"/kde-rotation/etc/xdg/autostart/plasma-builtin-screen-rotation.desktop \
  "$root"/kde-rotation/share/plasma/plasmoids/io.github.cceelen.builtin-screen-rotation/contents/ui/main.qml
grep -c "RUN+=\"$(readlink "$root/second-screen")/libexec/asus-ux8406-second-screen " \
  "$root"/second-screen/lib/udev/rules.d/90-asus-ux8406-second-screen.rules | grep -qx 3
"$root"/second-screen/libexec/asus-ux8406-second-screen --version
grep -c "\"$(readlink "$root/keyboard-bpf")/libexec/asus-ux8406-keyboard-state " \
  "$root"/keyboard-bpf/lib/udev/rules.d/80-asus-ux8406-keyboard-state.rules | grep -qx 2
"$root"/keyboard-bpf/libexec/asus-ux8406-keyboard-state --version
ls -l "$root"/tcc-guard/bin/asus-ux8406-tcc-guard "$root"/tcc-guard/lib/systemd/system/asus-ux8406-tcc-guard.service \
  "$root"/tcc-guard/share/doc/asus-ux8406-tcc-guard/asus-ux8406-tcc-guard.toml.example
grep -q "ExecStart=$(readlink "$root/tcc-guard")/bin/asus-ux8406-tcc-guard$" \
  "$root"/tcc-guard/lib/systemd/system/asus-ux8406-tcc-guard.service
"$root"/tcc-guard/bin/asus-ux8406-tcc-guard --version
grep -qx "Exec=$(readlink "$root/kde-keys")/libexec/asus-ux8406-kde-keys" \
  "$root"/kde-keys/etc/xdg/autostart/asus-ux8406-kde-keys.desktop
"$root"/kde-keys/libexec/asus-ux8406-kde-keys --version
grep -qx "Exec=$(readlink "$root/kde-rotation")/bin/plasma-builtin-screen-rotation run" \
  "$root"/kde-rotation/etc/xdg/autostart/plasma-builtin-screen-rotation.desktop
"$root"/kde-rotation/bin/plasma-builtin-screen-rotation --version
echo "OK"
