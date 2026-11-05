# kde-live-test

**A development tool that tests the KDE packages in the Fedora KDE live session
on the laptop. The tester runs one command from the data partition of the stick;
the tool installs the packages, checks what a machine can check, asks for the
physical actions that remain and decides from the events what happened. It is
not packaged and not installed.**

## Use

1. Put the packages in `rpms/` next to the program on the stick's data
   partition. Make that partition ext4: the program must keep its execute bit,
   and the live user (uid 1000) must own the files. Boot the stick and start the
   live session. No network is needed if the image has all requirements of the
   packages (see "Offline").

2. Open Konsole and run the program from the partition. The live session mounts
   it under the label of the partition; the label is only the path part,
   `<label>` here:

   ```sh
   /run/media/liveuser/<label>/ux8406-kde-live-test
   ```

3. `sudo` asks for the password of `liveuser`. The user has none: press Enter.
   The tool renews the sudo rights about every 60 seconds until it ends, so the
   guided steps cannot lose them.

4. Do what the tool asks, one action at a time. Type `s` and Enter to skip an
   action. There is no time limit: the tool waits until the action is done or
   you skip it (`--timeout SECONDS` gives up an action after that time). The
   lower screen is the one next to the keyboard, the upper one has the camera;
   every prompt says so. The tool keeps its terminal window above the other
   windows while it asks, and closes each window that it opened as soon as the
   check is done. The tool asks for no action with a pen; with a pen at hand,
   add `--pen`.

The live session has no logout and login (it logs in by itself), so the tool
tests the real autostart path in the session. Plasma starts the programs through
`xdg-desktop-autostart.target`; systemd's autostart generator makes a unit
`app-<id>@autostart.service` from each entry in `/etc/xdg/autostart`. The tool
runs `systemctl --user daemon-reload` for the new entries, checks that the units
exist, starts them (the target, or the units by name if the target is active
already) and checks that they are active and the programs have their names on
the bus. KWin reads its scripts at its start: the report has a `SKIP` row for
"KWin loads the script by itself", because only a new session can show it. A new
run starts from the beginning and moves the earlier `results/` to
`results.previous/`.

### Image with the packages already installed

If every package in `rpms/` is installed in the image with the same name,
version, release and architecture (`rpm -q` of the name from each file), udev
loaded the keyboard program at the boot, Plasma started both autostart units and
KWin loaded the script at the login, all before the tool runs. The tool then
looks at this state first, before it changes anything, and writes the section
"As booted": KWin loaded the script by itself (`isScriptLoaded`); both
`app-<id>@autostart.service` units are active and their programs own their bus
names; the keyboard program is attached; kglobalacceld has the shortcuts; the
policy is `never` on both panels (read only, no change). The row "packages
already in the image" replaces the `dnf` install; `rpm -V` and the requirement
checks still run. The activation steps and the other checks follow as for a
fresh install; the rows that run twice carry the suffix "(after activation)" the
second time. The `SKIP` row "KWin loads the script by itself" is not written. If
a package is missing or has another version, the tool installs all of them as
before.

### Offline

If every requirement of the packages is in the live image already, the tool runs
`dnf install --disablerepo=*` on the files: dnf does not load the metadata of a
repository, so no network is needed. If dnf has to fetch something, the tool
needs the network and stops with the names if there is none. The test tools
(`libinput`, `kscreen-doctor`, `konsole`, ...) are in the live image; if one is
missing and there is no network, the report has a `SKIP` row that says so.

The tool finds its directory from its own path. It refuses to run if the root
file system is not the overlay of a live image (`rd.live.image` on the kernel
command line, and an overlay or `/dev/mapper/live-*` as `/`), so it never
changes an installed system. It uses the network only for dnf, and only when dnf
has something to fetch. The tool needs only the programs of the live image;
where one is missing and the network is there, it installs the package with dnf.

## Results

`results/report.md` has one row for each check: name, automatic or guided, `OK`,
`FAIL` or `SKIP`, and the detail. A row is written when its check ends, so a
crash keeps what was done. `results/raw/` has the logs: the output of dnf, the
two programs, `journalctl --user` and the kernel log, `kscreen-doctor -o`,
`libinput list-devices` and `libinput debug-events`, `kwinoutputconfig.json`,
`kcminputrc`, `kglobalshortcutsrc`, the versions. An earlier `results/` moves to
`results.previous/`.

## What it checks

| Part          | Automatic                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Guided (the tester acts, the tool decides)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Before        | product name, Wayland session, Plasma, KWin and kernel versions, network, sudo; requirements of each rpm (present, from the stick, to fetch; no network is told with the package names)                                                                                                                                                                                                                                                                                                                                                                                                                    |                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Installation  | `dnf install` of `rpms/*.rpm`; every package installed, every requirement provided, `rpm -V`; udev rules and hwdb reloaded; the keyboard's HID devices (vendor 0b05), the dock port, the panel connectors and the backlight triggered again; the KWin script loaded over D-Bus; the autostart units of both programs made, started and active                                                                                                                                                                                                                                                              |                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Dependencies  | udev-hid-bpf runs and the hwdb names the keyboard object; iio-sensor-proxy on the system bus; kglobalacceld; KWin's scripting and input device interfaces; the Wayland globals `kde_output_management_v2` (version 5 or later) and the output device; the QML modules of the applet                                                                                                                                                                                                                                                                                                                        |                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Outputs, keys | `eDP-1` and `eDP-2` built-in with different UUIDs in a valid layout; both programs say "ready"; the four shortcuts registered with `Launch (9)`, `F19`, `Launch (1)` and (no key) the action "Toggle auto-rotate" of the rotation program (kglobalacceld reads the shortcut file of the emoji key only when the session starts: where the packages were installed at run time, that row is a skip, and the row of the section "As booted" is the check); the ELAN touch and pen devices act on their panels; the keyboard is connected and its BPF program is pinned; the tablet-mode switch (information) | The key right of F12 three times (off, on in the same place, off); F8 with two test windows that the tool puts on the two screens; the keyboard on the lower screen (the screen goes, the key does nothing) and off it (screen and layout back); a finger on each screen, and with `--pen` also the pen (libinput event from the mapped device; a touch on the other screen is a likely slip, so the tool says which screen it saw and asks once more, and fails only if the second touch is on the wrong screen too; without `--pen` the pen row is a skip); the keyboard backlight key twice (the keyboard has no LED device: the tool reads the level in the pinned state map with `sudo /usr/libexec/asus-ux8406-keyboard-state load`, which writes nothing, and wants a change at each press; without a map you answer "Did the keyboard light change?") |
| Rotation      | The policy is `never` on both panels; a change to `always` is reverted; `restore` gives KWin's policy back, and the program starts again; `TurnScreen` of the lower screen to upside down (eDP-2 transform 2, eDP-1 unchanged, a valid layout, `Enabled` false) and `SetEnabled(true)` (both screens under the sensor)                                                                                                                                                                                                                                                                                     | The laptop on its left side, right side, upright and upside down (both panels turned by 180 degrees, the lower one on top), once with both panels on and once with the upper panel only (the tool asks for the key right of F12 where the lower screen is not as wanted): the sensor, the transform of both panels, no gap and no overlap, the old layout when upright again; the lower screen turned upside down with the applet, the keyboard on and off that screen (it comes back upside down), and Auto-rotate on again; two middle clicks on the tray applet                                                                                                                                                                                                                                                                                            |
| Shortcut keys |                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | F11 starts `plasma-emojier`; F12 starts System Settings                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |

Not covered, and left to [the test plan](../../docs/kde-test-plan.md): the
logout with the lower screen off (check 6), a touch with the lower screen off
(10), windows of all kinds for F8 (8), an external monitor (12), the buttons of
the applet without a sensor (13) and fractional scales (16).

## Layout of the code

The checks are in `src/` by topic: `preflight`, `deps` and `install` (rpm and
dnf), `activate`, `auto`, `guided`, `windows` (test windows placed and watched
through KWin's scripting interface; the scripts call back a D-Bus service of the
tool), `collect` (the raw logs), `report`, `console`, `runner` (programs behind
a trait) and `live`. The output client is `asus_ux8406_kde_common::output`; the
tool polls it in place of the event loop of the daemons, because it waits on the
tester, with a time limit only if one is given.

## Test

```sh
meson setup build -Dlive_test=true
meson test -C build --suite asus-ux8406-kde-live-test
```

The tests run against the fakes of `kde-fakes/`: a fake KWin on Wayland, a
private bus with fakes of KWin's scripting and input devices, kglobalacceld, the
sensor and the two programs, and canned answers for rpm, dnf and the other
programs. A thread plays the hands of the tester. Every region is run by a test
(CI enforces 100 %). What no fake can show is in the test plan: the checks need
the laptop to be proven.

A row "... closed" with `OK` and the text "close it by hand" is information: a
window that the tool opened did not close, and the tester closes it. The same
holds for the row "terminal window above the others", which says why KWin did
not keep the terminal on top.
