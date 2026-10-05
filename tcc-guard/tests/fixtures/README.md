# Machines for the tests

Each directory is the part of a system that `asus-ux8406-tcc-guard` looks at, laid out as
below `/`; the tests copy one into a temporary directory and run the guard
on it with `--root`.

- `ux8406ca/`: an ASUS Zenbook Duo UX8406CA. One processor package (coretemp
  on `hwmon1`, 95 C, limit 105 C) at offset 10, two hwmon devices that are no
  processors, an empty kernel log, and a configuration (`etc/asus-ux8406-tcc-guard.toml`) with short times.
- `second-package/`: laid over the first for a machine with two packages
  (coretemp on `hwmon5`, offset at `0000:80:04.0`).
