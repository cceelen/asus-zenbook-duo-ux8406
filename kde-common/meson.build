# What the KDE programs share: the logic of the layouts (checked against the
# vectors of the GNOME extensions) and the client of KWin's output
# management. It is a library of the programs and installs nothing.
test(
  'cargo test',
  cargo,
  args: ['test', '--release'] + cargo_args + ['--package', 'asus-ux8406-kde-common'],
  env: cargo_env,
  suite: 'asus-ux8406-kde-common',
  timeout: 900,
  # One cargo at a time: they share a target directory.
  is_parallel: false,
)
