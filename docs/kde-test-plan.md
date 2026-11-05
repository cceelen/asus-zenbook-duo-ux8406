# Test plan for the KDE packages

**How to try the KDE packages without a change of the installed system, from the
most complete test to the least: a live USB on the laptop, a virtual machine, a
container. Only the live USB tests the hardware. The automated tests
(`meson test`) need no KDE session; this plan is for what they cannot show.**

The packages: [keys](../kde-keys/README.md),
[rotation](../kde-rotation/README.md), [shortcuts](../kde-shortcuts/README.md).

## What each way can test

| Item                                         | A: live USB | B: virtual machine | C: container |
| -------------------------------------------- | ----------- | ------------------ | ------------ |
| Packages install and the programs start      | yes         | yes                | yes          |
| KWin takes the script; shortcuts in settings | yes         | yes                | partly ¹     |
| Real connectors `eDP-1`, `eDP-2`, built-in   | yes         | no ²               | no ²         |
| Lower screen off and on with the key         | yes         | as `Virtual-*` ²   | as virtual ² |
| Keyboard on the screen (connector gone)      | yes         | no                 | no           |
| Touchscreens and pens on their screens       | yes         | no                 | no           |
| Orientation sensor                           | yes         | fake ³             | fake ³       |
| Applet in the system tray                    | yes         | yes                | no           |
| KWin's saved layout at the next login        | yes ⁴       | yes                | no           |

1. Without a seat KWin binds no key; the script loads, and a call over D-Bus
   stands in for the key.
2. The keys program works only with the connectors `eDP-1` and `eDP-2` and on a
   UX8406 (DMI). In a VM the outputs are `Virtual-1`, `Virtual-2` and the DMI
   name is another one, so the keys program ends at once. The rotation program
   takes `eDP`, `LVDS` and `DSI` as built-in; a VM has none of them, so it shows
   no control. A VM tests the packages, the script, the applet and KWin's
   behaviour, not the layouts of the panels.
3. python-dbusmock has a template for iio-sensor-proxy; the orientation is then
   set by hand.
4. A live session keeps its home in RAM: log out and in, but do not restart. The
   Fedora KDE live session has no logout and login (it logs in by itself), so
   this row needs an installed system or a virtual machine.

## A: live USB on the laptop (recommended)

Fedora 44 KDE Plasma Desktop (Fedora KDE edition), or Kubuntu 26.04 (Plasma
6.6). Both panels, the sensor and the keyboard are there. Nothing is written to
the internal disk. Do the checks below by hand.

1. Build the packages (`dist/fedora/`, or the pull request's artefacts) and put
   the package files on a data partition of the live stick. The data partition
   is ext4, so that the live user (uid 1000) owns the files.
2. Start the live image and choose the live session. The test needs no network:
   when all requirements of the packages are in the live image, dnf installs
   from the files and does not contact a repository. A network is needed only if
   a requirement is missing from the image.
3. Open Konsole and install the packages from the data partition with
   `sudo dnf install`. The live user `liveuser` has no password, but `sudo` asks
   for one: press Enter at the question.
4. Start the two programs as Plasma does at the login: through the autostart
   units that systemd makes from the entries in `/etc/xdg/autostart`. The
   session needs no logout and no login. KWin reads its scripts only when it
   starts, so KWin loads the script by itself only if the live image has the
   packages installed already.
5. Do the checks below, one by one. The lower screen is the one next to the
   keyboard and the upper one has the camera.
6. Shut down: nothing stays.

## B: virtual machine with two screens

QEMU/KVM with virt-manager or `virt-install`, from the same live image, with two
heads (`-device virtio-vga,max_outputs=2`, or two `virtio-gpu` heads in
virt-manager's video settings). Install the packages as in A.

A fake sensor, in the VM:

```sh
sudo systemctl stop iio-sensor-proxy
sudo python3 -m dbusmock --system --template iio-sensor-proxy &
gdbus call --system -d net.hadess.SensorProxy -o /net/hadess/SensorProxy \
    -m org.freedesktop.DBus.Mock.SetInternalProperty \
    net.hadess.SensorProxy AccelerometerOrientation '<"left-up">'
```

(The method name of the template is to be checked against the python-dbusmock
version of the image.)

## C: KWin in a container

`kwin_wayland --virtual --width 2880 --height 1800 --output-count 2` runs KWin
without a seat and without a GPU, in a rootless podman or toolbox container
(Fedora 44 with `kwin-wayland`, `plasma-workspace` and `dbus-daemon`). With
`dbus-run-session` around it, the programs of this tree can run in the same
container against a real KWin: the two outputs are virtual, so this checks the
protocol versions, the script loading
(`qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.loadScript`), the D-Bus
names, and that KWin takes a layout of two outputs in one configuration. It is a
candidate for an automated smoke test in CI. It has not been tried: the KWin and
Qt packages are several hundred MB, and the container's outputs are not built-in
panels (note 2).

## Checks

Items 1 and 2 are for the outputs, 3 to 10 for the keys, 11 for the shortcuts,
and 12 to 17 for the rotation.

01. `plasmashell --version`, `echo $XDG_SESSION_TYPE` (`wayland`).

02. `kscreen-doctor -o`: `eDP-1` and `eDP-2` as Panel, with two different UUIDs.

03. `journalctl --user -b -g asus-ux8406-kde-keys`: "ready", and one line for
    each touch device that got a screen.

04. System Settings, Keyboard, Shortcuts, KWin: "Switch the lower screen on or
    off" with `Launch (9)`, "Swap the windows of the two screens" with `F19`.

05. The key right of F12, three times, with the keyboard on the cable and on
    Bluetooth: off, on in the same place, off. `kwinoutputconfig.json` changes
    each time.

06. Log out with the lower screen off, log in: still off. The key: on.

07. Keyboard on the lower screen: the screen goes off; the key does nothing.
    Keyboard off again: the state from before.

08. F8 with windows on both screens: maximized, tiled, minimized, fullscreen,
    Xwayland windows, a window on all desktops. The Plasma panels stay. With the
    lower screen off: nothing.

09. Touch the lower screen: it acts there. Pen on each screen, also turned. In
    `~/.config/kcminputrc` each device has `OutputUuid`. Log out and in: the
    same.

10. With the lower screen off or the keyboard on it: does a touch on the lower
    screen do anything (the controller can still send)?

11. F11: the emoji selector opens. F12: System Settings opens (Plasma's own
    binding of `XF86Tools`).

12. `journalctl --user -b -g plasma-builtin-screen-rotation`: "ready", and
    "KWin's auto-rotate in-tablet-mode is now never" for each panel. System
    Settings, Display: "Automatic orientation" is "Never" on both.

13. Laptop on its left side, its right side, upright, upside down, with the
    keyboard on the cable. The two screens turn and go side by side; upside down
    they turn by 180 degrees and go one below the other, the lower one on top;
    upright, the layout from before. Also with the lower screen off: the upper
    one turns. With an external monitor right of the panels.

14. The applet: a middle click switches it off (the icon changes, the screens no
    longer turn) and on; a left click opens the popup, and so do Space and Enter
    on the icon in the system tray (they must not toggle). System Settings,
    Shortcuts, "Auto-rotate screens": the action "Toggle auto-rotate" is there
    with no key; give it a key: the key switches the rotation off and on, also
    with the applet hidden, and the journal says `rotation off (shortcut)`. With
    `iio-sensor-proxy` stopped: the buttons of each screen and the two buttons
    for all screens.

    A screen turned by hand: in the popup, turn the lower screen upside down:
    only that screen turns, the switch shows off, `Enabled` is false and the
    saved setting does not change
    (`~/.config/plasma-builtin-screen-rotation.toml`). Put the keyboard on the
    lower screen and take it off: the lower screen comes back upside down. Turn
    the laptop: the screens do not follow. Switch Auto-rotate on: both screens
    turn to the orientation of the laptop at once. Every button can be reached
    with the Tab key, pressed with the space key, and has an accessible name
    (Orca reads "Lower screen: Upside down"). Log out and in: the screens follow
    the sensor.

15. Set "Automatic orientation" to "Always" in System Settings: it becomes
    "Never" again, and the file has `always`.

16. `plasma-builtin-screen-rotation restore`: the program ends, System Settings
    shows the old value, the file has no policies.

17. Fractional scale 1.5 and 1.75 on the panels: the turned layout has no gap
    and no overlap (KWin may round the logical size other than
    `round(pixels / scale)`).

## Points that only a KDE session can settle

| Point                                                                        | Check                   |
| ---------------------------------------------------------------------------- | ----------------------- |
| `registerShortcut` takes `Launch (9)` as the key of XF86Launch9              | 4, 5                    |
| A user binding stays after an update of the script                           | 4                       |
| KWin takes `enable 0` for an internal panel on 6.6 (no `disable` capability) | 5                       |
| The saved layout after the second-screen helper takes `eDP-2` away and back  | 7                       |
| `sendClientToScreen` keeps tiles and size; property names of 6.6             | 8                       |
| Names of the ELAN devices in KWin; the pen is a device of its own            | 3, 9                    |
| A touchscreen without its screen is mapped to the upper one                  | 10                      |
| KWin takes both panels turned in one configuration; no move of other outputs | 13                      |
| The policy `never` stops KWin's own claim on the accelerometer               | 12, 13                  |
| The QML module `org.kde.plasma.workspace.dbus` and its property map in 6.4   | 14                      |
| The autostart entry gets `WAYLAND_DISPLAY`, and runs after KWin              | 3, 12                   |
| The rounding of logical sizes at fractional scales                           | 17                      |
| Whether the UX8406 has a tablet-mode switch (only for KWin's own rotation)   | `libinput list-devices` |
