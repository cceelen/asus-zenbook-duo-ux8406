# Test plan for the KDE packages

**How to try the KDE packages without a change of the installed system, from the
most complete test to the least: a live USB on the laptop, a virtual machine, a
container. Only the live USB tests the hardware. The automated tests
(`meson test`) need no KDE session; this plan is for what they cannot show.**

The packages: [keys](../kde-keys/README.md),
[shortcuts](../kde-shortcuts/README.md).

## What each way can test

| Item                                         | A: live USB | B: virtual machine | C: container |
| -------------------------------------------- | ----------- | ------------------ | ------------ |
| Packages install and the programs start      | yes         | yes                | yes          |
| KWin takes the script; shortcuts in settings | yes         | yes                | partly ¹     |
| Real connectors `eDP-1`, `eDP-2`, built-in   | yes         | no ²               | no ²         |
| Lower screen off and on with the key         | yes         | as `Virtual-*` ²   | as virtual ² |
| Keyboard on the screen (connector gone)      | yes         | no                 | no           |
| Touchscreens and pens on their screens       | yes         | no                 | no           |
| KWin's saved layout at the next login        | yes ³       | yes                | no           |

1. Without a seat KWin binds no key; the script loads, and a call over D-Bus
   stands in for the key.
2. The keys program works only with the connectors `eDP-1` and `eDP-2` and on a
   UX8406 (DMI). In a VM the outputs are `Virtual-1`, `Virtual-2` and the DMI
   name is another one, so the keys program ends at once. A VM tests the
   packages, the script and KWin's behaviour, not the layouts of the panels.
3. A live session keeps its home in RAM: log out and in, but do not restart. The
   Fedora KDE live session has no logout and login (it logs in by itself), so
   this row needs an installed system or a virtual machine.

## A: live USB on the laptop (recommended)

Fedora 44 KDE Plasma Desktop (Fedora KDE edition), or Kubuntu 26.04 (Plasma
6.6). Both panels and the keyboard are there. Nothing is written to the internal
disk. Do the checks below by hand.

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
4. Start the program as Plasma does at the login: through the autostart units
   that systemd makes from the entries in `/etc/xdg/autostart`. The session
   needs no logout and no login. KWin reads its scripts only when it starts, so
   KWin loads the script by itself only if the live image has the packages
   installed already.
5. Do the checks below, one by one. The lower screen is the one next to the
   keyboard and the upper one has the camera.
6. Shut down: nothing stays.

## B: virtual machine with two screens

QEMU/KVM with virt-manager or `virt-install`, from the same live image, with two
heads (`-device virtio-vga,max_outputs=2`, or two `virtio-gpu` heads in
virt-manager's video settings). Install the packages as in A.

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

Items 1 and 2 are for the outputs, 3 to 10 for the keys, and 11 for the
shortcuts.

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

## Points that only a KDE session can settle

| Point                                                                        | Check |
| ---------------------------------------------------------------------------- | ----- |
| `registerShortcut` takes `Launch (9)` as the key of XF86Launch9              | 4, 5  |
| A user binding stays after an update of the script                           | 4     |
| KWin takes `enable 0` for an internal panel on 6.6 (no `disable` capability) | 5     |
| The saved layout after the second-screen helper takes `eDP-2` away and back  | 7     |
| `sendClientToScreen` keeps tiles and size; property names of 6.6             | 8     |
| Names of the ELAN devices in KWin; the pen is a device of its own            | 3, 9  |
| A touchscreen without its screen is mapped to the upper one                  | 10    |
| The autostart entry gets `WAYLAND_DISPLAY`, and runs after KWin              | 3     |
