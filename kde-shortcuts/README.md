# asus-zenbook-duo-ux8406-kde-shortcuts

**Default KDE Plasma shortcuts for function keys of the ASUS Zenbook Duo UX8406:
the emoji key (F11) opens Plasma's emoji selector. The package is optional:
without it, you can set the same shortcut yourself.**

| Key         | Key code    | Qt name      | Action                            |
| ----------- | ----------- | ------------ | --------------------------------- |
| F11 (emoji) | `KEY_PROG1` | `Launch (1)` | Emoji selector (`plasma-emojier`) |

The keys need the [keyboard package](../keyboard-bpf/README.md).

F12 (MyASUS) sends `KEY_F13`, which xkeyboard-config makes `XF86Tools`. Plasma
binds that key to System Settings by itself, so this package does not bind it.

## How it operates

kglobalacceld reads the file `asus-zenbook-duo-ux8406-shortcuts.desktop` from
`/usr/share/kglobalaccel/`. Each action of the file is a global shortcut that
runs a program. The shortcut shows in System Settings, Keyboard, Shortcuts,
under "ASUS Zenbook Duo UX8406", where you can change it.

kglobalacceld reads the folder when it starts, that is, at the start of the
session, not when a package is installed: log out and in after the install. It
skips a file with `NoDisplay=true` and a file that is not a valid application,
so the main entry has no `NoDisplay` and has an `Exec`. The folder is not a menu
folder: the file does not show in a menu.

## Test

```sh
meson test -C build --suite kde-shortcuts
```

The test reads the file as kglobalacceld does: each action has a name, a program
and a key that the keyboard program sends, and the main entry is one that
kglobalacceld does not skip.
