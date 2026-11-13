# asus-ux8406-keyboard-state: the helper that carries the keyboard's backlight
# and function row mode from one connection to the next, its udev rule and its
# system-sleep hook.
# Part of the keyboard package, next to the HID-BPF program.

# Cargo knows what has to be rebuilt, so it is asked every time.
asus_ux8406_keyboard_state = custom_target(
  'asus-ux8406-keyboard-state',
  output: 'asus-ux8406-keyboard-state',
  command: cargo_build + ['asus-ux8406-keyboard-state', '&&', 'cp', cargo_target_dir / 'release' / 'asus-ux8406-keyboard-state', '@OUTPUT@'],
  console: true,
  build_always_stale: true,
  install: true,
  install_dir: libexecdir,
  install_mode: 'rwxr-xr-x',
)

# The rule names the helper by its full path.
configure_file(
  input: 'udev' / '80-asus-ux8406-keyboard-state.rules.in',
  output: '80-asus-ux8406-keyboard-state.rules',
  configuration: paths,
  install: true,
  install_dir: udevdir / 'rules.d',
  install_mode: 'rw-r--r--',
)

# The hook has systemd run the helper before and after a sleep, with the words
# that systemd gives it. The hook names the helper by its full path. Its name
# in the build tree is not that of the helper, which is built there too.
sleep_hook = configure_file(
  input: 'systemd' / 'asus-ux8406-keyboard-state.sleep.in',
  output: 'asus-ux8406-keyboard-state.sleep',
  configuration: paths,
)
install_data(
  sleep_hook,
  install_dir: sleepdir,
  install_mode: 'rwxr-xr-x',
  rename: 'asus-ux8406-keyboard-state',
)

# The copyright and permission notices of the crates in the program, from
# cargo-about (about.toml at the top of the tree).
if install_licences
  install_data(
    licence_mit, 'THIRD-PARTY-NOTICES.txt',
    install_dir: licensedir / package_names['keyboard-bpf'],
    install_mode: 'rw-r--r--',
  )
endif

# The Rust tests run in the one cargo test of meson.build.
rust_test_packages += 'asus-ux8406-keyboard-state'
rust_test_suites += 'asus-ux8406-keyboard-state'
