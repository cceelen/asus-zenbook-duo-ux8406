# asus-zenbook-duo-ux8406-kde-keys

**The KDE Plasma part for two keys of the ASUS Zenbook Duo UX8406 and for its
touchscreens. The key right of F12 switches the lower screen on and off, F8
swaps the windows of the two screens, and each touchscreen and pen gets its
screen. It is active after the next login. The keys need the keyboard package.**

This is the KDE counterpart of the [GNOME extension](../gnome-keys/README.md).
It needs Plasma 6 on Wayland. It is not tested on KDE hardware yet: refer to the
[test plan](../docs/kde-test-plan.md).

## Keys

- The key right of F12 switches the lower screen on and off. With the keyboard
  on the lower screen, the key does nothing.
- F8 swaps the windows of the two screens. With one screen, the key does
  nothing.

The package [asus-zenbook-duo-ux8406-keyboard-bpf](../keyboard-bpf/README.md) is
necessary: it makes the keyboard send these keys, as `XF86Launch9` and `F19`. To
give the two actions other keys, use System Settings, Keyboard, Shortcuts, KWin:
"Switch the lower screen on or off" and "Swap the windows of the two screens".

### The state of the lower screen

KWin keeps each change of the screens in its saved configuration
(`~/.config/kwinoutputconfig.json`). There is no change for the session only, as
in GNOME. Thus:

| Event                            | Lower screen                    |
| -------------------------------- | ------------------------------- |
| Key right of F12                 | On or off; KWin keeps it.       |
| Next login                       | As KWin kept it.                |
| Keyboard put on the lower screen | Off: the connector is gone.     |
| Keyboard taken off               | As before: KWin's saved layout. |
| First login of a user            | On.                             |

The part for the keyboard on the screen is the
[second-screen helper](../second-screen/README.md). This program adds no rule at
login.

## Touch

Without a setting, KWin gives each touchscreen the first screen of its size. The
two screens have one size, so a touch on the lower screen acts on the upper
screen.

The program sets the screen of each touchscreen and each pen that has no screen,
through KWin's property `outputName`. KWin keeps it in `~/.config/kcminputrc`.
The program reads the ids of the touch controllers from the machine (`ELAN9008`
upper, `ELAN9009` lower, as in GNOME). It does not change a device that has a
screen. To set another screen, use System Settings, Touchscreen.

Limit: a device that you gave a monitor that is not connected at login has no
screen at that time, and the program gives it its panel.

## How it operates

| Part                   | Where it runs           | What it does                                             |
| ---------------------- | ----------------------- | -------------------------------------------------------- |
| KWin script            | in KWin                 | binds the two keys, swaps the windows, calls the program |
| `asus-ux8406-kde-keys` | user session, autostart | switches the lower screen, sets the touch devices        |

- A KWin script cannot change the screens. The script calls the method
  `ToggleLowerScreen` of the program's service
  `io.github.cceelen.AsusUX8406.Keys` on the session bus.
- The program is a client of KWin's Wayland protocols `kde_output_device_v2` and
  `kde_output_management_v2`. KWin 6.6 gives each screen as a global of its own;
  KWin 6.8 gives them through `kde_output_device_registry_v2`. The program takes
  the one that KWin has.
- The layouts come from [kde-common](../kde-common/README.md): the logic of the
  GNOME extension in Rust. The program keeps the layout from before the key
  switched the screen off, for the session, and puts it back at the next press.
  Without it, the lower screen goes below the upper screen.
- The touch devices are KWin's objects `/org/kde/KWin/InputDevice/eventN` on the
  session bus.
- An XDG autostart entry (`/etc/xdg/autostart/asus-ux8406-kde-keys.desktop`,
  `OnlyShowIn=KDE`) starts the program. It operates with and without systemd,
  and the program gets `WAYLAND_DISPLAY` of the session. Where Plasma uses
  systemd, it starts the entry as a user unit, with its log in the journal:
  `journalctl --user -b -g asus-ux8406-kde-keys`.
- The KWin script is installed in `/usr/share/kwin/scripts/` with
  `EnabledByDefault`. To switch it off: System Settings, Window Management, KWin
  Scripts.

On a machine whose DMI product name does not contain `UX8406`, the program logs
one line and ends.

## Test

```sh
meson test -C build --suite asus-ux8406-kde-keys
```

- `cargo test` runs the program against a fake KWin: a Wayland server with the
  two protocols, and KWin's input devices on a private bus that the test starts
  (`dbus-daemon`). The tests examine the key (off, on, the remembered layout,
  the keyboard on the screen, a refusal of KWin), the touch devices (each panel,
  a device with a screen, a new device, devices that fail), and the ends
  (another model, no Wayland, not KWin, no bus, a second instance, the end of
  KWin). The fakes are in `kde-fakes/`.
- `script` runs the KWin script in node, in a stand-in for KWin's scripting API:
  the two shortcuts, the D-Bus call, and which windows move.
- `package` compares the metadata of the script with the install directory.
- `tests/fixtures/ux8406ca/` is the part of `/sys` that the program reads.

The tests run each region of the program, and each line and branch of the
script.
