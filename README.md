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

**Screen rotation.** GNOME does not turn the screens when the laptop stands on
its side (book mode).

- [gnome-shell-extension-builtin-screen-rotation](gnome-rotation/README.md):
  turns the two screens and puts them side by side. GNOME only. It is not
  specific to this model.

**No fix in this project:**

- Microphone on the TRRS jack: no input. Possibly a hardware limit. Refer to the
  [SOF ticket](https://github.com/thesofproject/linux/issues/5703).
- Battery charge limit: the limit of 80 % operates. The battery charged above it
  two times, to 87 % on 2026-09-30 and to 100 % on 2026-10-03. The cause is not
  known.

**Login screen.** It has its own display settings and does not read those of
your session. Without a saved layout it switches all screens on, the built-in
screens also, at each logout. To give it the layouts of your session, copy the
file. Do this again after you change the arrangement of your monitors.

GDM 50 (Fedora 44):

```sh
dir=/var/lib/gdm/seat0/config
sudo install -m 644 -o "$(sudo stat -c %u $dir)" -g "$(sudo stat -c %g $dir)" ~/.config/monitors.xml $dir/monitors.xml
sudo restorecon $dir/monitors.xml
```

Older GDM versions use `~gdm/.config/monitors.xml`. Not tested.

**Not examined:** fan profiles, touch and pen mapping, beam-forming microphones,
camera.

## Compatibility

| Symbol | Meaning                                                  |
| ------ | -------------------------------------------------------- |
| ✅     | A published package. It installs in a clean container.   |
| 🛠️     | No published package. The package builds from this tree. |
| ❌     | Not available.                                           |
| 🤖     | Tested on the hardware.                                  |

| Distribution         | Guard | Second screen | Keyboard | Keys  | Rotation |
| -------------------- | ----- | ------------- | -------- | ----- | -------- |
| Fedora 44            | ✅ 🤖 | ✅ 🤖         | ✅ 🤖    | ✅ 🤖 | ✅ 🤖    |
| Fedora 43            | ✅    | ✅            | ✅       | ✅    | ✅       |
| RHEL 10 and rebuilds | ✅ ¹  | ✅ ¹          | ✅ ¹     | ✅ ¹  | ✅ ¹     |
| Arch                 | ✅    | ✅            | ✅       | ✅    | ✅       |
| openSUSE Tumbleweed  | ✅    | ✅            | ✅ ²     | ✅    | ✅       |
| openSUSE Slowroll    | ✅ ³  | ✅ ³          | ✅ ² ³   | ✅ ³  | ✅ ³     |
| openSUSE Leap 16.0   | ✅    | ✅            | ❌ ⁴     | ✅    | ✅       |
| Debian 13            | ✅    | ✅            | ❌ ⁴     | ✅    | ✅       |
| Debian testing       | ✅    | ✅            | ✅       | ✅    | ✅       |
| Debian unstable      | ✅    | ✅            | ✅       | ✅    | ✅       |
| Ubuntu 26.04         | ✅    | ✅            | ✅       | ✅    | ✅       |
| Alpine               | 🛠️    | 🛠️            | ❌ ⁴     | 🛠️    | 🛠️       |
| Nix                  | 🛠️    | 🛠️            | ❌ ⁴     | 🛠️    | 🛠️       |

1. Built for RHEL 10 with EPEL 10. Installed in AlmaLinux 10 and Rocky Linux 10
   containers, not on RHEL.
2. The distribution has no [udev-hid-bpf], which the keyboard package needs. The
   personal OBS project `home:xanders` builds it. With that repository added,
   the keyboard package installs and the loader reads the program. This project
   does not maintain that repository.
3. Published, but not installed in a container: there is no container image.
4. The distribution has no [udev-hid-bpf].

🤖: UX8406CA, Fedora 44, Linux 7.2, GNOME 50. The UX8406MA is not tested; its
keyboard ids and dock port can be different. Send your result in a GitHub issue
to add a test mark. Refer to [CONTRIBUTING.md](CONTRIBUTING.md) for the data
that helps.

Requirements:

| Part             | Requirement                                                      |
| ---------------- | ---------------------------------------------------------------- |
| All but Rotation | A Zenbook Duo UX8406. On other machines the programs do nothing. |
| Guard            | systemd or OpenRC                                                |
| Second screen    | udev                                                             |
| Keyboard         | Linux 6.11 or later with HID-BPF and BTF, and [udev-hid-bpf]     |
| Keys             | GNOME Shell 45 or later                                          |
| Rotation         | GNOME Shell 45 or later, iio-sensor-proxy; a laptop of any model |

Ubuntu 24.04 is not supported: its Rust is too old.

## Installation

Remove other Zenbook Duo solutions before you install these packages. Refer to
[Related projects](#related-projects).

The packages are in two repositories:

- [COPR project](https://copr.fedorainfracloud.org/coprs/packit/cceelen-asus-zenbook-duo-ux8406-releases/)
  for Fedora.
- [OBS project](https://build.opensuse.org/project/show/home:cceelen:asus-zenbook-duo-ux8406)
  for openSUSE, Debian, Ubuntu and Arch. Its
  [download directory](https://download.opensuse.org/repositories/home:/cceelen:/asus-zenbook-duo-ux8406/)
  has one directory for each distribution.

A package list for the commands below:

```sh
pkgs="asus-zenbook-duo-ux8406-tcc-guard asus-zenbook-duo-ux8406-second-screen gnome-shell-extension-asus-zenbook-duo-ux8406-keys gnome-shell-extension-builtin-screen-rotation"
```

Where the distribution has udev-hid-bpf (refer to
[Compatibility](#compatibility)), add `asus-zenbook-duo-ux8406-keyboard-bpf` to
the list.

### Fedora and RHEL 10

Fedora 43, Fedora 44 and Rawhide:

```sh
sudo dnf copr enable packit/cceelen-asus-zenbook-duo-ux8406-releases
sudo dnf install $pkgs asus-zenbook-duo-ux8406-keyboard-bpf
```

RHEL 10 and its rebuilds need EPEL 10, the CRB repository, and the name of the
build target. On AlmaLinux 10 and Rocky Linux 10:

```sh
sudo dnf install epel-release dnf-plugins-core
sudo crb enable
sudo dnf copr enable packit/cceelen-asus-zenbook-duo-ux8406-releases rhel+epel-10-x86_64
sudo dnf install $pkgs asus-zenbook-duo-ux8406-keyboard-bpf
```

On RHEL, enable EPEL and CodeReady Builder as the
[EPEL documentation](https://docs.fedoraproject.org/en-US/epel/getting-started/)
tells you, then use the last two commands.

### openSUSE

| Distribution | `dist`                |
| ------------ | --------------------- |
| Tumbleweed   | `openSUSE_Tumbleweed` |
| Slowroll     | `openSUSE_Slowroll`   |
| Leap 16.0    | `16.0`                |

```sh
dist=openSUSE_Tumbleweed
sudo zypper addrepo --refresh https://download.opensuse.org/repositories/home:/cceelen:/asus-zenbook-duo-ux8406/$dist/home:cceelen:asus-zenbook-duo-ux8406.repo
sudo zypper install $pkgs
```

### Debian and Ubuntu

| Distribution    | `dist`            |
| --------------- | ----------------- |
| Debian 13       | `Debian_13`       |
| Debian testing  | `Debian_Testing`  |
| Debian unstable | `Debian_Unstable` |
| Ubuntu 26.04    | `xUbuntu_26.04`   |

```sh
dist=Debian_13
repo=https://download.opensuse.org/repositories/home:/cceelen:/asus-zenbook-duo-ux8406/$dist
curl -fsSL $repo/Release.key | sudo gpg --dearmor -o /etc/apt/keyrings/asus-zenbook-duo-ux8406.gpg
echo "deb [signed-by=/etc/apt/keyrings/asus-zenbook-duo-ux8406.gpg] $repo/ /" | sudo tee /etc/apt/sources.list.d/asus-zenbook-duo-ux8406.list
sudo apt update
sudo apt install $pkgs
```

### Arch

```sh
repo=https://download.opensuse.org/repositories/home:/cceelen:/asus-zenbook-duo-ux8406/Arch
curl -fsSL $repo/x86_64/home_cceelen_asus-zenbook-duo-ux8406_Arch.key -o obs.key
sudo pacman-key --add obs.key
sudo pacman-key --lsign-key "$(gpg --show-keys --with-colons obs.key | awk -F: '$1 == "fpr" { print $10; exit }')"
printf '[home_cceelen_asus-zenbook-duo-ux8406_Arch]\nServer = %s/$arch\n' "$repo" | sudo tee -a /etc/pacman.conf
sudo pacman -Sy $pkgs asus-zenbook-duo-ux8406-keyboard-bpf
```

### After the installation

The guard package enables and starts its service. On NixOS, set
`services.asus-zenbook-duo-ux8406-tcc-guard.enable = true`.

Log in again and enable the extensions:

```sh
gnome-extensions enable asus-zenbook-duo-ux8406-keys@cceelen.github.io
gnome-extensions enable builtin-screen-rotation@cceelen.github.io
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
- Thermal event: one kernel patch that only records the event. Not merged. ASUS
  support confirmed the protection: with the lid closed and under load, Windows
  hibernates. Linux is not in their validation. Questions about the event and a
  fix in the BIOS or in `asus-wmi` are with ASUS; no answer at this time.

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

Some of them have functions that this project does not have: touch and pen
mapping, battery charge limit, suspend.

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
