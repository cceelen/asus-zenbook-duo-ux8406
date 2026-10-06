# gnome-shell-extension-asus-zenbook-duo-ux8406-keys

A GNOME Shell extension for the ASUS Zenbook Duo UX8406. It gives a function to
two keys of the keyboard that need the desktop:

- The key right of F12 switches the lower screen on and off. The change is for
  the session only; the saved `monitors.xml` does not change. With the keyboard
  on the lower screen, the key does nothing.
- F8 swaps the windows of the two screens.

The package `asus-zenbook-duo-ux8406-keyboard-bpf` is necessary: it makes the
keyboard send these keys, as `XF86Launch9` and `F19`. On a machine that is not a
UX8406, the extension binds no key.

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

The tests use node and need no GNOME session. `layout.js` has the logic and no
GNOME imports; `display.js` calls `org.gnome.Mutter.DisplayConfig`. `state.js`
and `display.js` are links to `../gnome-common`: the two extensions of this
project use the same files, and the build installs a copy in each.

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
