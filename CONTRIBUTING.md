# Contributing

**You can help in three ways: tell which functions operate on your hardware,
report a problem, or send a change. All three go through
[GitHub](https://github.com/cceelen/asus-zenbook-duo-ux8406/issues). Only Fedora
44 on a UX8406CA is tested on the hardware, so a report from another
distribution or model is the contribution that helps most.**

| You want to                         | Go to                                       |
| ----------------------------------- | ------------------------------------------- |
| Tell what operates on your hardware | [Report your results](#report-your-results) |
| Report a problem                    | [Report a problem](#report-a-problem)       |
| Build the tree and run the tests    | [Build and test](#build-and-test)           |
| Send a change                       | [Rules for changes](#rules-for-changes)     |
| Find a part in the tree             | [Layout](#layout)                           |
| Make a release (maintainer)         | [docs/releases.md](docs/releases.md)        |
| Report a vulnerability, in private  | [SECURITY.md](SECURITY.md)                  |

## Report your results

Open an issue, also when all functions operate. Include:

- Model (`cat /sys/class/dmi/id/product_name`) and BIOS version.
- Distribution, kernel version (`uname -r`) and desktop.
- The package versions.
- What operates and what does not: lower screen (off with the keyboard on it,
  brightness), special function keys (dock, cable, Bluetooth), guard, screen
  rotation.

The packages are developed on a UX8406CA. For a UX8406MA or another sub-model,
also report the USB and Bluetooth ids of the keyboard (`udevadm info`), the USB
port of the docked keyboard, and the size of the keyboard's report descriptors.

The rotation extension is not specific to the UX8406: a report from another
laptop, with one or two built-in screens, is useful too.

## Report a problem

Give the data of [Report your results](#report-your-results), then the output of
the commands for the part.

Keyboard, lower screen or guard:

```sh
journalctl -b -k | grep -i -e hid -e bpf -e asus
journalctl -b -u asus-ux8406-tcc-guard
```

Rotation:

```sh
journalctl -b -g builtin-screen-rotation
gdctl show
monitor-sensor
```

`gdctl` is part of GNOME 48 and later; its output has the serial numbers of the
monitors. `monitor-sensor` is part of iio-sensor-proxy: turn the laptop while it
runs, then stop it with Ctrl+C.

## Build and test

Tools: `meson` 1.1 or later, `cargo` (Rust 1.85 or later), `clang`, the libbpf
headers, the kernel headers, `glib-compile-schemas`, `python3`, `node`.

```sh
meson setup build
meson test -C build
pre-commit run --all-files
```

- Do not run the tests as root: some tests make files unreadable, and root
  ignores file permissions.
- Formats and lints are [pre-commit](https://pre-commit.com) hooks. Run
  `pre-commit install` one time; `pre-commit run --all-files` runs them on the
  tree.
- `meson configure build` lists the options. For example `-Dkeyboard=false`
  omits a part, and `-Dinit=openrc` installs the guard with an OpenRC script in
  place of the systemd unit.
- `-Dunit_checks=enabled` adds the checks of the guard's systemd unit with
  `systemd-analyze` (`verify`, and an exposure of at most 1.3), as CI does.
- `meson test -C build --suite NAME` runs the tests of one part. The README of
  each part gives the name and tells what the tests examine.

### Packages in containers

The command below builds the packages of all distributions in containers. Each
build runs the linter of the distribution (rpmlint, lintian, namcap,
apkbuild-lint), installs the packages, and removes them again:

```sh
docker compose -f dev/containers/compose.yaml up --build \
    --abort-on-container-failure
```

### An extension without a package

To try an extension from the tree, install it for your user. Remove the packaged
extension first: it has the same UUID. The options switch the other parts off;
for the rotation extension, use `-Dgnome=false` in place of `-Drotation=false`.

```sh
meson setup build --prefix ~/.local -Dscreen=false -Dguard=false -Dkeyboard=false -Drotation=false
meson install -C build
```

The tests of the extensions use node and need no GNOME session. The two
extensions use the same files for the display state: `state.js` and `display.js`
in each `extension/` directory are links to `gnome-common/`, and the build
installs a copy in each extension. Their tests share the fixtures in
`gnome-common/tests`.

### Coverage

The tests run each region of the guard. To measure it, with
[cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov):

```sh
cargo llvm-cov -p asus-ux8406-tcc-guard --features failpoints \
    --ignore-filename-regex /tests/ --fail-under-regions 100
```

CI measures the coverage of all three Rust programs and of the extensions' logic
(Node's test runner) on each pull request (`ci.yml`) and each night on main
(`coverage.yml`), shows it in the summary of the run and sends it to
[Codecov](https://app.codecov.io/gh/cceelen/asus-zenbook-duo-ux8406). Only the
guard has a limit; for the others the coverage is measured, not enforced.

[![Coverage of each file](https://codecov.io/github/cceelen/asus-zenbook-duo-ux8406/graphs/tree.svg?token=CSOYWM3L9Q)](https://app.codecov.io/gh/cceelen/asus-zenbook-duo-ux8406)

### Fuzzing

Fuzz targets of the guard's parsers are in `fuzz/`, with
[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) and a nightly compiler.
`cargo fuzz list` names them, and `fuzz/seeds/` has a start for each:

```sh
cp Cargo.lock fuzz/Cargo.lock  # fuzz/ keeps no lock file of its own
mkdir -p fuzz/corpus/kmsg
cargo +nightly fuzz run kmsg fuzz/corpus/kmsg fuzz/seeds/kmsg -- -max_total_time=60
```

Pull requests check the format and lints of the fuzz targets (`ci.yml`), but do
not run them. The nightly run on main (`fuzz.yml`, one run at a time) gives each
target every input of `fuzz/seeds/` and of the saved corpus, and fuzzes it. Each
input that makes a target fail gets a private draft security advisory. Add that
input to `fuzz/seeds/<target>/` in the pull request that fixes the failure.

### Notices and source archive

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
- A change to the build, the install paths or a recipe passes the
  [container builds](#packages-in-containers).
- Rust for programs, C for the HID-BPF program, Meson for the build. No
  Makefiles. No shell scripts for logic. A few lines of shell in a step of a
  workflow are not a script.
- Use established crates for the command line, logging, signals and errors.
- No test code in `src/`. Tests are in `tests/`, one file for each topic. A test
  machine is a directory tree in `tests/fixtures/`.
- The coverage of the guard stays at 100 % of its regions.
- No licence or copyright header in the files. `REUSE.toml` gives the licence of
  each file.
- Say a fact in one document only, and link to it from the others. The
  [README](README.md) is for users, the README of a part tells how the part
  operates, this file is for contributors, and
  [docs/releases.md](docs/releases.md) is for the maintainer.

An AI reviewer (`.github/workflows/review.yml`) reads each pull request. It
approves a pull request of the owner when it finds nothing that must change, and
requests changes when it finds something that must. A finding of medium or
higher severity always requests changes. For a pull request of Renovate or of
another contributor it writes a comment; the owner reviews and approves it.

## Layout

Each part has a README that tells what the part does, how it operates and what
its tests examine.

| Directory                                   | Contents                                                                                            |
| ------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| [tcc-guard/](tcc-guard/README.md)           | Thermal guard (Rust), systemd unit, OpenRC script                                                   |
| [second-screen/](second-screen/README.md)   | Helper for the lower screen (Rust), udev rule                                                       |
| [keyboard-bpf/](keyboard-bpf/README.md)     | HID-BPF program for the keyboard (C), hwdb entry                                                    |
| [keyboard-state/](keyboard-state/README.md) | Helper that keeps the keyboard's light and row mode (Rust), udev rule; part of the keyboard package |
| [gnome-keys/](gnome-keys/README.md)         | GNOME Shell extension for the display keys and the touchscreens                                     |
| [gnome-rotation/](gnome-rotation/README.md) | GNOME Shell extension that turns the screens                                                        |
| `gnome-common/`                             | Files that the two extensions share                                                                 |
| `packaging/`                                | One directory of recipes for each packaging system; [OBS setup](packaging/obs/README.md)            |
| `dev/containers/`                           | Container builds for development                                                                    |
| `fuzz/`                                     | Fuzz targets of the guard's parsers (cargo-fuzz)                                                    |
| `docs/releases.md`                          | How to make a release                                                                               |
