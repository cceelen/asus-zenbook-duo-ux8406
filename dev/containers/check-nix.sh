#!/bin/sh
# The check of the Nix build (dev/containers/nix.Dockerfile): Nix has no
# installation step here, the image holds the build results as links.
# compose.yaml and .github/workflows/packages.yml run it.
set -eu

ls -l /second-screen/libexec/asus-ux8406-second-screen /second-screen/lib/udev/rules.d/90-asus-ux8406-second-screen.rules \
  /keyboard-bpf/lib/firmware/hid/bpf/0010-ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.o \
  /keyboard-bpf/lib/udev/hwdb.d/82-hid-bpf-asus-ux8406.hwdb \
  /keyboard-bpf/libexec/asus-ux8406-keyboard-state \
  /keyboard-bpf/lib/udev/rules.d/80-asus-ux8406-keyboard-state.rules \
  /gnome-keys/share/gnome-shell/extensions/asus-zenbook-duo-ux8406-keys@cceelen.github.io/extension.js \
  /gnome-rotation/share/gnome-shell/extensions/builtin-screen-rotation@cceelen.github.io/extension.js \
  /kde-shortcuts/share/kglobalaccel/asus-zenbook-duo-ux8406-shortcuts.desktop \
  /kde-keys/libexec/asus-ux8406-kde-keys /kde-keys/etc/xdg/autostart/asus-ux8406-kde-keys.desktop \
  /kde-keys/share/kwin/scripts/asus-zenbook-duo-ux8406-keys/contents/code/main.js \
  /kde-rotation/bin/plasma-builtin-screen-rotation \
  /kde-rotation/etc/xdg/autostart/plasma-builtin-screen-rotation.desktop \
  /kde-rotation/share/plasma/plasmoids/io.github.cceelen.builtin-screen-rotation/contents/ui/main.qml
grep -c "RUN+=\"$(readlink /second-screen)/libexec/asus-ux8406-second-screen " \
  /second-screen/lib/udev/rules.d/90-asus-ux8406-second-screen.rules | grep -qx 3
/second-screen/libexec/asus-ux8406-second-screen --version
grep -c "\"$(readlink /keyboard-bpf)/libexec/asus-ux8406-keyboard-state " \
  /keyboard-bpf/lib/udev/rules.d/80-asus-ux8406-keyboard-state.rules | grep -qx 2
/keyboard-bpf/libexec/asus-ux8406-keyboard-state --version
ls -l /tcc-guard/bin/asus-ux8406-tcc-guard /tcc-guard/lib/systemd/system/asus-ux8406-tcc-guard.service \
  /tcc-guard/share/doc/asus-ux8406-tcc-guard/asus-ux8406-tcc-guard.toml.example
grep -q "ExecStart=$(readlink /tcc-guard)/bin/asus-ux8406-tcc-guard$" \
  /tcc-guard/lib/systemd/system/asus-ux8406-tcc-guard.service
/tcc-guard/bin/asus-ux8406-tcc-guard --version
grep -qx "Exec=$(readlink /kde-keys)/libexec/asus-ux8406-kde-keys" \
  /kde-keys/etc/xdg/autostart/asus-ux8406-kde-keys.desktop
/kde-keys/libexec/asus-ux8406-kde-keys --version
grep -qx "Exec=$(readlink /kde-rotation)/bin/plasma-builtin-screen-rotation run" \
  /kde-rotation/etc/xdg/autostart/plasma-builtin-screen-rotation.desktop
/kde-rotation/bin/plasma-builtin-screen-rotation --version
echo "OK"
