# The live-session test tool for the KDE packages. A development tool: it is
# not built by `meson compile`, not installed and not in any package. Only its
# tests run (with -Dlive_test=true). They start dbus-daemon and run against the
# fakes of kde-fakes.
test(
  'cargo test',
  cargo,
  args: ['test', '--release'] + cargo_args + ['--package', 'asus-ux8406-kde-live-test'],
  env: cargo_env,
  suite: 'asus-ux8406-kde-live-test',
  timeout: 900,
  # One cargo at a time: they share a target directory.
  is_parallel: false,
)
