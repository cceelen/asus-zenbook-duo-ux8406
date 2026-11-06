# plasma-builtin-screen-rotation

**A program for KDE Plasma that turns the built-in screens when the laptop
stands on its side or upside down, also with a keyboard, a touchpad or a mouse
connected. On its side, two screens go side by side. It is made for the two
screens of the ASUS Zenbook Duo UX8406, but it operates on a laptop of any
model. It is active after the next login, with the switch "Auto-rotate" in the
system tray. Before you remove the package, run
`plasma-builtin-screen-rotation restore`.**

This is the KDE counterpart of the
[GNOME extension](https://github.com/cceelen/gnome-extension/blob/main/gnome-rotation/README.md),
with the same layouts. It needs Plasma 6.4 or later on Wayland and
`iio-sensor-proxy`. It is not tested on KDE hardware yet: refer to the
[test plan](../docs/kde-test-plan.md).

## What it does

- Laptop on its left or right side: the built-in screens that are on are turned.
  Two screens are put side by side, in the order in which they lie.
- Laptop upright again: the layout from before comes back.
- Laptop upside down: the built-in screens that are on are turned by 180
  degrees. Two screens go one below the other, the bottom one on top; upright
  again, the layout from before comes back.

An external monitor that is beside the built-in screens, right of them or below
them, moves to make room. Other monitors stay. With the built-in screens off,
the program does nothing. It takes each built-in screen as mounted upright, as
the GNOME extension does.

KWin keeps each layout in its saved configuration
(`~/.config/kwinoutputconfig.json`): a turned layout is still there at the next
login. The program then puts the screens as the laptop stands.

## KWin's own auto-rotate

KWin turns only the first built-in screen, and by default only in tablet mode.
While the package is installed, the program sets KWin's setting "Automatic
orientation" to "Never" on each built-in screen: at each start, and each time it
changes. It keeps the value from before in its configuration file
(`~/.config/plasma-builtin-screen-rotation.toml`).

Before you remove the package, run this command as the user, in the session:

```sh
plasma-builtin-screen-rotation restore
```

It stops the program of the session, waits until its process has ended, and
gives each built-in screen KWin's setting from before. A screen that is not
connected keeps its saved value in the file until the next `restore`.

## Use

Install the package `plasma-builtin-screen-rotation` (refer to the
[README](../README.md#installation)), then log in again.

A left click on the applet "Auto-rotate" in the system tray opens its popup: the
switch "Auto-rotate" and a section for each built-in screen ("Upper screen" and
"Lower screen" for two screens, "Built-in screen" for one). A middle click on
the icon switches the rotation on and off. So does the global shortcut "Toggle
auto-rotate" of the program: it has no key at first, and you set one in System
Settings, Shortcuts, under "Auto-rotate screens". The program registers the
shortcut itself with kglobalacceld, so that it works also while the applet is
hidden. The keyboard shortcut of the applet (its settings), and the keys Space
and Enter on its icon in the system tray, open the popup, as for any applet. The
program keeps the on-off choice in its configuration file. While it is off, the
program does not use the sensor. All buttons of the popup can be reached with
the Tab key and pressed with the space key, and each has an accessible name.

### Turn a screen by hand

Each section of the popup has four buttons: upright, left side up, right side up
and upside down. The button of the present orientation is marked. Each screen
can have any orientation, also a different one than the other screen. With the
upper screen upright and the lower one upside down, two people who sit opposite
each other read the screen near to them the right way up.

- A turn by hand switches "Auto-rotate" off for the session, so that the sensor
  does not undo it. The saved setting does not change: after a new login the
  screens follow the laptop again.
- Switch "Auto-rotate" on again (the switch, a middle click or the global
  shortcut): all screens go back under the sensor and turn to the orientation of
  the laptop at once.
- The screens stay as they lie: two screens one below the other stay so, in the
  same order, and two screens side by side stay so. They are placed again from
  the top left corner of the two, with their new sizes. An external monitor
  beside them moves as for the sensor, and slides along the new edge if one
  screen is narrower than the other.
- When the monitors change (the keyboard goes on or off the lower screen, a
  monitor is plugged in) and KWin sets the layout again, each screen that is
  shown goes back to its orientation. A screen that is off keeps its orientation
  until it is back. If KWin does not take the orientation of a screen, that
  screen is not held any more, and the journal says so.
- Screens that are turned differently get their place from the order of the
  built-in screens when they follow the sensor again: the upper screen is the
  first one.
- Not for a screen that is off, a screen that is mirrored, or a monitor without
  a mode: the buttons of such a screen do nothing.

When no sensor gives the orientation, the popup has no switch, but the buttons
of each screen, and two more buttons. Each turns all built-in screens by 90
degrees, counterclockwise or clockwise: left side up, upright, right side up. To
try the buttons on a machine with a sensor, stop the service:
`sudo systemctl stop iio-sensor-proxy`.

On a machine without a built-in screen, the applet is hidden and the program
does nothing.

## How it operates

| Part                             | Where it runs           | What it does                                       |
| -------------------------------- | ----------------------- | -------------------------------------------------- |
| `plasma-builtin-screen-rotation` | user session, autostart | follows the sensor, gives the layouts to KWin      |
| Applet "Auto-rotate"             | in the system tray      | shows the switch or the buttons, calls the program |

- The program claims the accelerometer of iio-sensor-proxy
  (`net.hadess.SensorProxy`, system bus) while the rotation is on. A thread does
  all calls to the service, so that the program is ready, sets KWin's policies
  and answers its applet while the service is slow; a claim that gets no answer
  is tried again, after a pause of 2 seconds that doubles up to 60. When the
  laptop holds an orientation for 0.7 seconds, the program gives KWin the layout
  for it.
- The layouts come from [kde-common](../kde-common/README.md): the logic of the
  GNOME extension in Rust, checked against the same cases. KWin takes a layout
  in one configuration of `kde_output_management_v2` (both screens and the moved
  monitors together).
- The program's service `io.github.cceelen.BuiltinScreenRotation` on the session
  bus has the properties `Enabled` (false while a screen is held, whatever the
  saved setting says), `Control` (`switch`, `buttons` or `none`), `Screens` (an
  array of connector, name and orientation, with signals), `CanTurnClockwise`,
  `CanTurnCounterclockwise`, and the methods `SetEnabled`, `Toggle` (the program
  flips its state; the middle click calls it, and the global shortcut does the
  same), `TurnScreen(connector, orientation)`, `Turn` (all screens by 90
  degrees) and `Quit`. The program logs each change of the state with the method
  that asked (`rotation off (Toggle)`, `rotation off (shortcut)`), and each turn
  by hand (`eDP-2: turned to bottom-up by hand (TurnScreen)`).
- The applet has no logic of its own: it shows the properties and calls the
  methods, with Plasma's QML module `org.kde.plasma.workspace.dbus`. The system
  tray shows it while the service is on the bus
  (`X-Plasma-DBusActivationService`).
- An XDG autostart entry starts the program, as for the
  [keys](../kde-keys/README.md#how-it-operates). Its log:
  `journalctl --user -b -g plasma-builtin-screen-rotation`.

## Test

```sh
meson test -C build --suite plasma-builtin-screen-rotation
```

- `cargo test` runs the program against a fake KWin and a fake iio-sensor-proxy
  on a private bus (`kde-fakes/`). The tests examine the turns (each side,
  upright again, a saved turned layout, upside down, a refusal of KWin), the
  sensor (claim and release, the switch, a service that starts again, an
  accelerometer that goes, a refused claim, the shortcut registered with a fake
  kglobalacceld, a service that answers late or not at all, the worker with its
  pauses between two tries), the applet's service (switch, `Toggle`, buttons, no
  built-in screen, `Quit`), KWin's policy (set to never, kept, set again, an old
  KWin), `restore`, and the ends (no configuration directory, a bad file, no
  Wayland, not KWin, no bus, a second instance, the end of KWin, SIGTERM).
- `applet` checks the metadata of the applet and that its QML uses the names of
  the program's service.

The tests run each region of the program. The QML is not run: there is no QML
test without Plasma.
