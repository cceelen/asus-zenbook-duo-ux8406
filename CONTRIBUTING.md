# Contributing

Use the
[GitHub issues](https://github.com/cceelen/asus-zenbook-duo-ux8406/issues) for
reports, questions and changes. For a vulnerability, refer to
[SECURITY.md](SECURITY.md).

## Report your results

Only Fedora 44 is tested on the hardware. A report from another distribution is
useful, also when all functions operate. Include:

- Model (`cat /sys/class/dmi/id/product_name`) and BIOS version.
- Distribution, kernel version (`uname -r`) and desktop.
- The package versions.
- What operates and what does not: lower screen (off with the keyboard on it,
  brightness), special function keys (dock, cable, Bluetooth), guard, screen
  rotation.

For a problem, add the output of:

```sh
journalctl -b -k | grep -i -e hid -e bpf -e asus
journalctl -b -u asus-ux8406-tcc-guard
```

The rotation extension is not specific to the UX8406: a report from another
laptop, with one or two built-in screens, is useful too. For a problem with the
rotation, add the output of these commands. `gdctl` is part of GNOME 48 and
later; its output has the serial numbers of the monitors. `monitor-sensor` is
part of iio-sensor-proxy: turn the laptop while it runs, then stop it with
Ctrl+C.

```sh
journalctl -b -g builtin-screen-rotation
gdctl show
monitor-sensor
```

The packages are developed on a UX8406CA. For a UX8406MA or another sub-model,
also report the USB and Bluetooth ids of the keyboard (`udevadm info`), the USB
port of the docked keyboard, and the size of the keyboard's report descriptors.

## Build and test

Tools: `meson` 1.1 or later, `cargo` (Rust 1.85 or later), `clang`, the libbpf
headers, the kernel headers, `glib-compile-schemas`, `python3`, `node`.

```sh
meson setup build
meson test -C build
pre-commit run --all-files
```

Formats and lints are [pre-commit](https://pre-commit.com) hooks. Run
`pre-commit install` one time; `pre-commit run --all-files` runs them on the
tree.

Do not run the tests as root. `meson configure build` lists the options; for
example `-Dkeyboard=false` omits a part.

The command below builds the packages of all distributions in containers. Each
build runs the linter of the distribution (rpmlint, lintian, namcap,
apkbuild-lint), installs the packages, and removes them again:

```sh
docker compose -f dev/containers/compose.yaml up --build \
    --abort-on-container-failure
```

Coverage of the guard, with
[cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov):

```sh
cargo llvm-cov -p asus-ux8406-tcc-guard --features failpoints \
    --ignore-filename-regex /tests/ --fail-under-regions 100
```

Fuzz targets of the guard's parsers are in `fuzz/`, with
[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) and a nightly compiler.
`cargo fuzz list` names them, and `fuzz/seeds/` has a start for each:

```sh
mkdir -p fuzz/corpus/kmsg
cargo +nightly fuzz run kmsg fuzz/corpus/kmsg fuzz/seeds/kmsg -- -max_total_time=60
```

After a change of `Cargo.lock`, write the third-party notices again with
[cargo-about](https://github.com/EmbarkStudios/cargo-about). The commands are in
`about.toml`.

`meson dist -C build` makes the source archive with the crates in `vendor/`. It
uses the committed files only.

## Rules for changes

- The title of a pull request is a
  [Conventional Commits](https://www.conventionalcommits.org) title, for example
  `fix(guard): ...`. The changelog and the next version come from these titles.
- `meson test` and `pre-commit run --all-files` pass.
- A change to the build, the install paths or a recipe passes the container
  builds.
- Rust for programs, C for the HID-BPF program, Meson for the build. No
  Makefiles. No shell scripts for logic.
- Use established crates for the command line, logging, signals and errors.
- No test code in `src/`. Tests are in `tests/`, one file for each topic. A test
  machine is a directory tree in `tests/fixtures/`.
- The tests run each region of the guard (coverage 100 %).
- No licence or copyright header in the files. `REUSE.toml` gives the licence of
  each file.

## Layout

```text
second-screen/   helper for the lower screen (Rust), udev rule
tcc-guard/       thermal guard (Rust), systemd unit, OpenRC script
keyboard-bpf/    HID-BPF program (C), hwdb entry
keyboard-state/  helper that keeps the keyboard's light and row mode (Rust),
                 udev rule; part of the keyboard-bpf package
gnome-keys/      GNOME Shell extension for the display keys
gnome-rotation/  GNOME Shell extension that turns the screens
gnome-common/    files that the two extensions share
packaging/       one directory of recipes for each packaging system
dev/containers/  container builds for development
fuzz/            fuzz targets of the guard's parsers (cargo-fuzz)
docs/releases.md how to make a release
```
