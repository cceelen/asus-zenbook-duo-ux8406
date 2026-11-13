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
  "$root"/keyboard-bpf/lib/systemd/system-sleep/asus-ux8406-keyboard-state \
  "$root"/gnome-rotation/share/gnome-shell/extensions/builtin-screen-rotation@cceelen.github.io/extension.js \
  "$root"/kde-rotation/bin/plasma-builtin-screen-rotation \
  "$root"/kde-rotation/etc/xdg/autostart/plasma-builtin-screen-rotation.desktop \
  "$root"/kde-rotation/share/plasma/plasmoids/io.github.cceelen.builtin-screen-rotation/contents/ui/main.qml
grep -c "RUN+=\"$(readlink "$root/second-screen")/libexec/asus-ux8406-second-screen " \
  "$root"/second-screen/lib/udev/rules.d/90-asus-ux8406-second-screen.rules | grep -qx 3
"$root"/second-screen/libexec/asus-ux8406-second-screen --version
grep -c "\"$(readlink "$root/keyboard-bpf")/libexec/asus-ux8406-keyboard-state " \
  "$root"/keyboard-bpf/lib/udev/rules.d/80-asus-ux8406-keyboard-state.rules | grep -qx 2
"$root"/keyboard-bpf/libexec/asus-ux8406-keyboard-state --version
test -x "$root"/keyboard-bpf/lib/systemd/system-sleep/asus-ux8406-keyboard-state
grep -qx "exec $(readlink "$root/keyboard-bpf")/libexec/asus-ux8406-keyboard-state sleep \"\$@\"" \
  "$root"/keyboard-bpf/lib/systemd/system-sleep/asus-ux8406-keyboard-state
ls -l "$root"/tcc-guard/bin/asus-ux8406-tcc-guard "$root"/tcc-guard/lib/systemd/system/asus-ux8406-tcc-guard.service \
  "$root"/tcc-guard/share/doc/asus-ux8406-tcc-guard/asus-ux8406-tcc-guard.toml.example
grep -q "ExecStart=$(readlink "$root/tcc-guard")/bin/asus-ux8406-tcc-guard$" \
  "$root"/tcc-guard/lib/systemd/system/asus-ux8406-tcc-guard.service
"$root"/tcc-guard/bin/asus-ux8406-tcc-guard --version
grep -qx "Exec=$(readlink "$root/kde-rotation")/bin/plasma-builtin-screen-rotation run" \
  "$root"/kde-rotation/etc/xdg/autostart/plasma-builtin-screen-rotation.desktop
"$root"/kde-rotation/bin/plasma-builtin-screen-rotation --version
echo "OK"
