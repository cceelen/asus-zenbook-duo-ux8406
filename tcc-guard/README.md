# asus-ux8406-tcc-guard

A service for the ASUS Zenbook Duo UX8406. It lowers the temperature at which
the CPU throttles when the embedded controller (EC) warns of heat, and removes
the change when the machine is cool again.

## The problem

With the lid closed and under load, the EC raises a thermal warning: ACPI WMI
event 0x6D, which the kernel logs as `asus_wmi: Unknown key code 0x6d`. If the
CPU stays hot, the EC cuts the power. On the UX8406CA (BIOS 313) the EC waited
100 to 122 seconds before it raised the event and 142 to 180 seconds before it
cut the power.

## What it does

- It reads the kernel log for the warning and the CPU package temperature. These
  are its only inputs.
- On a warning it raises the thermal offset of the CPU
  (`/sys/bus/pci/devices/0000:00:04.0/tcc_offset_degree_celsius`) in steps until
  the temperature is below the level at which the EC stops its alert.
- It holds the offset, then lowers it one degree at a time while no new warning
  comes, until the offset is the value that it found at the start.
- It keeps nothing from one warning to the next.

When the service stops, it writes the original offset back. It also keeps that
value in `/run/asus-ux8406-tcc-guard`, so that after a kill the next start or
`asus-ux8406-tcc-guard restore` writes it back. The unit runs `restore` after
each stop.

On a machine whose product name does not contain `UX8406`, the service logs one
line and ends. The setting `model` changes that.

## Use

The packages enable and start the service. After a build from the source, do
that with `systemctl enable --now asus-ux8406-tcc-guard.service`. The log:

```sh
journalctl -u asus-ux8406-tcc-guard
```

`man asus-ux8406-tcc-guard` gives the commands and options.

Settings are optional. The file is `/etc/asus-ux8406-tcc-guard.toml`; root must
own it and no other user may write it. `asus-ux8406-tcc-guard.toml.example` in
the documentation directory of the package gives each setting and its default.

## Test

```sh
meson test -C build --suite asus-ux8406-tcc-guard
```

Do not run the tests as root: some tests make files unreadable, and root ignores
file permissions. The tests run two times, the second time with fault injection
(`--features failpoints`, `src/fault.rs`). An installed build does not contain
the fault points.

`-Dinit=openrc` installs an OpenRC script in place of the systemd unit.

## Upstream

The fix belongs in the firmware or in the kernel (`asus-wmi`). Refer to
[Upstream fixes](../README.md#upstream-fixes).
