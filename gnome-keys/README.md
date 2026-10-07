# gnome-shell-extension-asus-zenbook-duo-ux8406-keys

A GNOME Shell extension for the ASUS Zenbook Duo UX8406. It gives a function to
two keys of the keyboard that need the desktop, and gives each touchscreen its
screen.

## Keys

- The key right of F12 switches the lower screen on and off. The change is for
  the session only; the saved `monitors.xml` does not change. With the keyboard
  on the lower screen, the key does nothing.
- F8 swaps the windows of the two screens.

The package `asus-zenbook-duo-ux8406-keyboard-bpf` is necessary: it makes the
keyboard send these keys, as `XF86Launch9` and `F19`.

## Touch

Without a setting, GNOME gives the two touchscreens to the upper screen: the two
screens have the same size and say that they are the same monitor. A touch on
the lower screen then acts on the upper screen.

The extension sets the screen for each touchscreen and for each pen, in GNOME's
setting `output` of the device (`org.gnome.desktop.peripherals.touchscreen` and
`.tablet`), with the connector as the fourth value. It reads the ids of the
touch controllers from the machine. It does not change a setting that you made,
and the settings stay when the extension is disabled. To go back to GNOME's own
choice:

```sh
dconf reset -f /org/gnome/desktop/peripherals/touchscreens/
dconf reset -f /org/gnome/desktop/peripherals/tablets/
```

Touch is tested on the UX8406CA. The pens are not tested.

On a machine that is not a UX8406, the extension binds no key and sets nothing.

GNOME Shell 45 to 51 is declared. Only GNOME Shell 50 is tested.

## Use

After the installation, log in again. GNOME Shell on Wayland finds a new
extension only when it starts.

```sh
gnome-extensions enable asus-zenbook-duo-ux8406-keys@cceelen.github.io
```

## Test

```sh
meson test -C build --suite gnome-keys
```

The tests use node and need no GNOME session. `layout.js` and `touch.js` have
the logic and no GNOME imports; `display.js` calls
`org.gnome.Mutter.DisplayConfig`. `state.js` and `display.js` are links to
`../gnome-common`: the two extensions of this project use the same files, and
the build installs a copy in each. The tests of the two extensions also share
their fixtures: `../gnome-common/tests`.

`tests/verify.js` is a manual check in a GNOME session. It asks Mutter to verify
the current layout and the layout that the key requests. It applies nothing.

```sh
gjs -m tests/verify.js
```

## Install for one user without a package

```sh
meson setup build --prefix ~/.local -Dscreen=false -Dguard=false -Dkeyboard=false -Drotation=false
meson install -C build
```

Remove the packaged extension first: it has the same UUID.
