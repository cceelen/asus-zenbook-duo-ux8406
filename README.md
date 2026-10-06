# Linux packages for the ASUS Zenbook Duo UX8406

Packages that fix common Linux problems on the
[ASUS Zenbook Duo UX8406](https://www.asus.com/us/laptops/for-home/zenbook/asus-zenbook-duo-2024-ux8406/).

I own a UX8406CA. The community solutions that I tried gave mixed results and
needed manual installation. The power-off problem stayed for months. These
packages are fixes or workarounds until the upstream projects (kernel, desktops,
BIOS) supply the fix.

## Problems and packages

**Power off with the lid closed.** The embedded controller (EC) sends an
undocumented thermal event ("hot bag") when the lid is closed. If the system
ignores the event, the EC switches the laptop off.

- [asus-zenbook-duo-ux8406-tcc-guard](tcc-guard/README.md): a service that
  lowers the temperature limit of the CPU when the EC sends the event, and
  raises it again in steps when the alert stops.

**Lower screen.** It does not follow the brightness control. It stays on when
the keyboard lies on it.

- [asus-zenbook-duo-ux8406-second-screen](second-screen/README.md): sets the
  lower screen to off while the keyboard is on it, and keeps the two screens at
  the same brightness.

**Keyboard.** More than half of the special function keys are not mapped.

- [asus-zenbook-duo-ux8406-keyboard-bpf](keyboard-bpf/README.md): maps the
  special function keys, the keyboard light and Fn+Esc. It operates on the dock,
  on the cable and with Bluetooth.
- [gnome-shell-extension-asus-zenbook-duo-ux8406-keys](gnome-keys/README.md):
  gives a function to the two display keys. GNOME only.

**No fix in this project:**

- Screen orientation: the screen does not rotate in book mode. The sensor and
  iio-sensor-proxy are available.
- Automatic suspend: the laptop does not suspend when the lid is closed and no
  dock or external display is connected.
- Microphone on the TRRS jack: no input. Possibly a hardware limit. Refer to the
  [SOF ticket](https://github.com/thesofproject/linux/issues/5703).
- Battery charge limit: the limit of 80 % operates. On 2026-10-03 the battery
  charged to 100 % one time. The cause is not known.

**Not examined:** fan profiles, touch and pen mapping, beam-forming microphones,
camera.

## Compatibility

No packages are published. The first table gives where the packages are expected
to operate, from the code and from container builds. The second table gives what
a person tested on the hardware. Only the second table is evidence.

Expected:

| Part          | Requirement                                                      |
| ------------- | ---------------------------------------------------------------- |
| All           | A Zenbook Duo UX8406. On other machines the programs do nothing. |
| Guard         | systemd or OpenRC                                                |
| Second screen | udev                                                             |
| Keyboard      | Linux 6.11 or later with HID-BPF and BTF, and [udev-hid-bpf]     |
| Extension     | GNOME Shell 45 or later                                          |

| Distribution                           | Packages that install in a container                               |
| -------------------------------------- | ------------------------------------------------------------------ |
| Fedora 44, Arch, Debian testing        | All                                                                |
| Debian 13, openSUSE Tumbleweed, Alpine | Guard and second screen. These distributions have no udev-hid-bpf. |
| Nix                                    | The flake builds all packages. nixpkgs has no udev-hid-bpf.        |
| EPEL 10                                | Not built in a container. It has udev-hid-bpf and GNOME Shell 49.  |

Ubuntu 24.04 is not supported: its Rust is too old.

Tested on the hardware:

| Model    | Distribution | Kernel | Desktop  | Guard | Second screen | Keyboard | Extension |
| -------- | ------------ | ------ | -------- | ----- | ------------- | -------- | --------- |
| UX8406CA | Fedora 44    | 7.2    | GNOME 50 | yes   | yes           | yes      | yes       |

The UX8406MA is not tested. Its keyboard ids and dock port can be different.
Send your result in a GitHub issue to add a line. Refer to
[CONTRIBUTING.md](CONTRIBUTING.md) for the data that helps.

## Installation

Remove other Zenbook Duo solutions before you install these packages. Refer to
[Related projects](#related-projects).

Until packages are published, build them from this tree. Refer to
[CONTRIBUTING.md](CONTRIBUTING.md).

After the installation, start the guard. The package does not start it:

```sh
sudo systemctl enable --now asus-ux8406-tcc-guard.service
```

Then log in again and enable the extension:

```sh
gnome-extensions enable asus-zenbook-duo-ux8406-keys@cceelen.github.io
```

## Known limits

Keyboard:

- The function row starts in F1–F12 mode at each connection.
- Bluetooth special function is triggering BT connect events of the keyboard and
  as far as the current testing showed does not generate a host visible event.
- The package controls the keyboard light. The desktop cannot control it.
- The first press of F4 after a connection can have no effect.

Lower screen:

- Windows from the lower screen stay on the upper screen when the lower screen
  comes back.
- If the laptop starts with the keyboard on the lower screen, the screen
  possibly stays off after you remove the keyboard. Not tested.

## Upstream fixes

State in October 2026:

- Keyboard:
  [hid-asus patch](https://ratatoskr.run/linux-input/2026/05/9004316/t) for the
  Linux kernel. Applied, then removed. Not in Linux 7.2.
- Lower screen brightness:
  [backlight capability](https://ratatoskr.run/dri-devel/2026/09/17528280/t) for
  the Linux kernel. In review.
- Thermal event: one kernel patch that only records the event. Not merged. An
  ASUS technical support ticket is open for a fix in the BIOS or in `asus-wmi`.

## Related projects

I tried, examined and learned from these projects. Their manual installation and
configuration was the reason that I did not keep them. The comparison is from
their README files in October 2026.

| Project                                                                                                 | Model    | Method                                   |
| ------------------------------------------------------------------------------------------------------- | -------- | ---------------------------------------- |
| [alesya-h/zenbook-duo-2024-ux8406ma-linux](https://github.com/alesya-h/zenbook-duo-2024-ux8406ma-linux) | UX8406MA | Bash and Python, session programs, GNOME |
| [Fmstrat/zenbook-duo-linux](https://github.com/Fmstrat/zenbook-duo-linux)                               | UX8406CA | Bash, systemd                            |
| [zakstam/zenbook-duo-linux](https://github.com/zakstam/zenbook-duo-linux)                               | UX8406MA | Rust daemons; GNOME, KDE, Niri           |
| [JowiAoun/linux-on-zenbook-duo](https://github.com/JowiAoun/linux-on-zenbook-duo)                       | UX8406MA | Bash and Python, user services, GNOME    |
| [carlosh7/asus_UX8406MA](https://github.com/carlosh7/asus_UX8406MA)                                     | UX8406MA | Shell, services, GNOME                   |

Some of them have functions that this project does not have: screen rotation,
touch and pen mapping, battery charge limit, suspend.

## Contact and contributions

Use the
[GitHub issues](https://github.com/cceelen/asus-zenbook-duo-ux8406/issues) for
problems, questions and results on your hardware.
[CONTRIBUTING.md](CONTRIBUTING.md) tells you how to build and test.

## AI policy

I am a software engineer, but Rust and BPF are not my native languages. This
project was developed with Claude Code.

## Disclaimer

This is a community project. It is not affiliated with ASUSTeK Computer Inc.
ASUS and Zenbook are trademarks of ASUSTeK Computer Inc.

The guard changes the temperature at which the CPU throttles, and the helper
switches a screen off. You use the packages at your own risk. There is no
warranty; refer to the licence.

## Licence

MIT, with one exception: the directory `keyboard-bpf/` and its package are
GPL-2.0-only, because they contain headers from the Linux kernel. `LICENSE` is
the MIT text. `LICENSES/` has both texts, and `REUSE.toml` gives the licence of
each file.

[udev-hid-bpf]: https://gitlab.freedesktop.org/libevdev/udev-hid-bpf
