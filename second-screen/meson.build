# asus-ux8406-second-screen: the helper that udev runs, and its udev rule.

# Cargo knows what has to be rebuilt, so it is asked every time.
asus_ux8406_second_screen = custom_target(
  'asus-ux8406-second-screen',
  output: 'asus-ux8406-second-screen',
  command: cargo_build + ['asus-ux8406-second-screen', '&&', 'cp', cargo_target_dir / 'release' / 'asus-ux8406-second-screen', '@OUTPUT@'],
  console: true,
  build_always_stale: true,
  install: true,
  install_dir: libexecdir,
  install_mode: 'rwxr-xr-x',
)

# The rule names the helper by its full path.
configure_file(
  input: 'udev' / '90-asus-ux8406-second-screen.rules.in',
  output: '90-asus-ux8406-second-screen.rules',
  configuration: paths,
  install: true,
  install_dir: udevdir / 'rules.d',
  install_mode: 'rw-r--r--',
)

# The copyright and permission notices of the crates in the program, from
# cargo-about (about.toml at the top of the tree).
if install_licences
  install_data(
    licence_mit, 'THIRD-PARTY-NOTICES.txt',
    install_dir: licensedir / package_names['second-screen'],
    install_mode: 'rw-r--r--',
  )
endif

test(
  'cargo test',
  cargo,
  args: ['test', '--release'] + cargo_args + ['--package', 'asus-ux8406-second-screen'],
  env: cargo_env,
  suite: 'asus-ux8406-second-screen',
  timeout: 900,
  # One cargo at a time: they share a target directory.
  is_parallel: false,
)
