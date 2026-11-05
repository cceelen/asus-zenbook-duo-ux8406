# kde-common

**The library that the two KDE programs share: the display logic of the GNOME
extensions in Rust, checked against the same cases, and the client of KWin's
output management. It is part of the programs and installs nothing.**

| Module     | What it has                                                                 | GNOME file it comes from               |
| ---------- | --------------------------------------------------------------------------- | -------------------------------------- |
| `geometry` | the layout as plain data; sizes, valid layouts, moves                       | `gnome-common/state.js`                |
| `layout`   | the lower screen out of the layout and back                                 | `gnome-keys/extension/layout.js`       |
| `rotation` | the layout for an orientation of the laptop; the control of the applet      | `gnome-rotation/extension/rotation.js` |
| `touch`    | the touch controllers of the machine and their panels                       | `gnome-keys/extension/touch.js`        |
| `output`   | KWin's outputs on Wayland: what they are, and layouts and policies to apply | (KDE only)                             |
| `daemon`   | the event loop and the log of the two programs                              | (KDE only)                             |
| `protocol` | the Wayland code, made from the XML files in `protocols/`                   | (KDE only)                             |

The GNOME extensions keep their JavaScript. Where the two cannot be the same
(the mutter D-Bus data, the GNOME Shell objects), each desktop has its own code;
the logic on plain data is the same in both.

## The shared cases

`gnome-common/tests/vectors/*.json` has the cases for both implementations: each
a call, its input and its result. `gnome-common/tests/vectors.js` makes them
from the scenarios of the GNOME tests, and the GNOME code gives the results. The
node tests (`gnome-keys/tests/vectors.test.js`,
`gnome-rotation/tests/vectors.test.js`) check that the files are those that the
cases give and that the JavaScript gives each result; the Rust test
`tests/logic/vectors.rs` checks that Rust gives each result. After a change of
the cases or of the GNOME logic, write the files again:

```sh
node gnome-common/tests/write-vectors.js
```

Two differences, both outside the cases: a monitor without a mode takes no room
in Rust (JavaScript fails there), and a turn in another direction than
`clockwise` or `counterclockwise` is none in Rust.

## The protocols

`protocols/kde-output-device-v2.xml` (version 26) and
`protocols/kde-output-management-v2.xml` (version 22) are copies from
[plasma-wayland-protocols](https://invent.kde.org/libraries/plasma-wayland-protocols),
of 2026-11-03, under the licence that they state (MIT-CMU). The crate
`wayland-protocols-plasma` has an older version without
`kde_output_device_registry_v2`, which KWin 6.8 uses in place of one global for
each screen. The client binds what KWin offers, up to these versions.

## Test

```sh
meson test -C build --suite asus-ux8406-kde-common
```

- `tests/logic/`: the layouts on plain data, one file for each module, and the
  shared cases.
- `tests/outputs/`: the output client against the fake KWin of `kde-fakes/`, in
  the forms of KWin 6.6 (a global for each screen), 6.8 (the registry) and an
  old KWin without the auto-rotate policy: what KWin describes, the requests of
  a layout, refusals, policies, screens that come and go, and the end of the
  connection.
- `tests/event_loop/`: the event loop with a handler that writes down what it
  gets: commands, a time to wake up, changes of the outputs, the end of KWin.

The tests run each region of the library, except the generated code of
`protocol`.
