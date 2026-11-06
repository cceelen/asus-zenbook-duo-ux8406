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

KDE Plasma:

```sh
journalctl --user -b -g asus-ux8406-kde-keys
journalctl --user -b -g plasma-builtin-screen-rotation
kscreen-doctor -o
```

## Build and test

Tools: `meson` 1.1 or later, `cargo` (Rust 1.85 or later), `clang`, the libbpf
headers, the kernel headers, `glib-compile-schemas`, `python3`, `node`, and
`dbus-daemon` (the tests of the KDE programs start a private bus).

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
meson setup build --prefix ~/.local -Dscreen=false -Dguard=false -Dkeyboard=false -Drotation=false -Dkde_shortcuts=false -Dkde=false -Dkde_rotation=false
meson install -C build
```

The tests of the extensions use node and need no GNOME session. The two
extensions use the same files for the display state: `state.js` and `display.js`
in each `extension/` directory are links to `gnome-common/`, and the build
installs a copy in each extension. Their tests share the fixtures in
`gnome-common/tests`.

The KDE programs are tested against the fakes in `kde-fakes/`; for a test on KDE
itself, refer to the [test plan](docs/kde-test-plan.md).

The tests of the extensions run with Node's test runner, and CI fails the run
below 100 % of lines, branches or functions of the modules that they load. The
modules that need a GNOME Shell (`extension.js`, `display.js`, `sensor.js`) are
not loaded by a test, so they are not in it. The same check here:

```sh
node --test --experimental-test-coverage --test-coverage-exclude='**/tests/**' \
    --test-coverage-lines=100 --test-coverage-branches=100 --test-coverage-functions=100 \
    gnome-keys/tests/layout.test.js gnome-keys/tests/touch.test.js \
    gnome-keys/tests/vectors.test.js gnome-rotation/tests/hold.test.js gnome-rotation/tests/rotation.test.js \
    gnome-rotation/tests/vectors.test.js kde-keys/tests/script.test.js
```

### Coverage

The tests run each region and each branch of the five Rust programs. To measure
the regions, with [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov),
in one shell:

```sh
eval "$(cargo llvm-cov show-env --sh)"
cargo llvm-cov clean --workspace
cargo test --locked -p asus-ux8406-tcc-guard --features failpoints
cargo test --locked -p asus-ux8406-second-screen -p asus-ux8406-keyboard-state
cargo test --locked -p asus-ux8406-kde-common -p asus-ux8406-kde-keys -p asus-ux8406-kde-rotation
cargo llvm-cov report --ignore-filename-regex '/tests/|kde-fakes/|kde-common/src/protocol\.rs' --fail-under-regions 100
```

Without the next step, three lines of `keyboard-state/src/pins.rs` are not run:
the test `tests/root` makes a BPF array map, pins it on the BPF file system
(`/sys/fs/bpf/asus-ux8406-test-<pid>-<test>/`, removed at the end) and reads it
with the program's reader. That needs root. As an ordinary user the test says
why it does nothing and passes. CI runs it with `sudo`. To run it here, in the
same shell, before the report: build as the user, then run only the binary as
root. Its coverage data goes to the same directory as that of the other tests.

```sh
bin=$(cargo test --locked -p asus-ux8406-keyboard-state --test root --no-run \
    --message-format=json |
    jq -r 'select(.target.name == "root" and .executable != null) | .executable')
sudo env LLVM_PROFILE_FILE="$LLVM_PROFILE_FILE" "$bin" --nocapture
```

Every branch of the Rust programs is run by a test too. Stable `rustc` does not
take the flag for the branch counters. CI uses a nightly compiler of a fixed
date for it (the job `rust-branches` in `ci.yml`, which fails if one branch is
not run): move that date forward on purpose, in a change of its own, when the
job passes with the new nightly. To measure the branches with the stable
compiler of your distribution, as a test for yourself only, run the same steps
with:

```sh
export CARGO_TARGET_DIR=$HOME/.cache/branchcov RUSTC_BOOTSTRAP=1
eval "$(cargo llvm-cov show-env --sh --branch)"
```

and, in place of the report above:

```sh
cargo llvm-cov report --branch --ignore-filename-regex '/tests/|kde-fakes/|kde-common/src/protocol\.rs' \
    --json --summary-only --output-path branches.json
jq -e '.data[0].totals.branches.notcovered == 0' branches.json
```

CI measures the coverage of all five Rust programs and of the extensions' logic
(Node's test runner) on each pull request (`ci.yml`) and each night on main
(`coverage.yml`), shows it in the summary of the run and sends it to
[Codecov](https://app.codecov.io/gh/cceelen/asus-zenbook-duo-ux8406). The five
Rust programs have a limit of 100 % of their regions and branches; for the
extensions' logic the coverage is measured, not enforced. The KDE programs that
the tests start write their coverage too: they end at SIGTERM, not SIGKILL.

[![Coverage of each file](https://codecov.io/github/cceelen/asus-zenbook-duo-ux8406/graphs/tree.svg?token=CSOYWM3L9Q)](https://app.codecov.io/gh/cceelen/asus-zenbook-duo-ux8406)

### The HID-BPF program

The program in `keyboard-bpf/` runs in the kernel, so its tests compile the
unchanged source as host C against fakes of the BPF headers and of the kernel
functions it calls (`keyboard-bpf/tests/host/`). They need `clang`, `llvm-cov`
and `llvm-profdata`. `meson test -C build --suite keyboard-bpf` runs them: the
tests (`host`), the same with AddressSanitizer and UndefinedBehaviorSanitizer
(`host-sanitize`), the coverage, which fails below 100 % of lines, functions and
branches of the program's file (`host-coverage`), and a mutation check that
fails if a mutant of the source survives that is not argued as equivalent in
`tests/mutate.py` (`host-mutation`). By hand, from the tree's root:

```sh
python3 keyboard-bpf/tests/run_host.py coverage \
    keyboard-bpf/src/ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.c ~/.cache/bpfcov
```

Meson registers `host-sanitize`, `host-coverage` and `host-mutation` only after
`meson setup build -Dhost_checks=enabled`: the clang of a distribution can lack
the sanitizer runtimes and the C library of the host.

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
not run them. The weekly run on main (`fuzz.yml`, one run at a time) gives each
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
- The coverage of the JavaScript that the tests load (the logic of the
  extensions) stays at 100 % of its lines, branches and functions.
- No test code in `src/`. Tests are in `tests/`, one file for each topic. A test
  machine is a directory tree in `tests/fixtures/`.
- The coverage of the five Rust programs stays at 100 % of their regions and of
  their branches (nightly compiler).
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
| [kde-keys/](kde-keys/README.md)             | KDE Plasma program for the lower screen key and the touchscreens (Rust), KWin script                |
| [kde-rotation/](kde-rotation/README.md)     | KDE Plasma program that turns the screens (Rust), applet in the system tray (QML)                   |
| [kde-common/](kde-common/README.md)         | What the two KDE programs share (Rust): the layouts, KWin's output management, the cases of GNOME   |
| `kde-fakes/`                                | Fake KWin and sensor and a private bus for the tests of the KDE programs; not installed             |
| [kde-shortcuts/](kde-shortcuts/README.md)   | Default KDE Plasma shortcuts for function keys                                                      |
| `packaging/`                                | One directory of recipes for each packaging system; [OBS setup](packaging/obs/README.md)            |
| `dev/containers/`                           | Container builds for development                                                                    |
| `fuzz/`                                     | Fuzz targets of the guard's parsers (cargo-fuzz)                                                    |
| `docs/releases.md`                          | How to make a release                                                                               |
| `docs/kde-test-plan.md`                     | How to try the KDE packages on KDE                                                                  |
