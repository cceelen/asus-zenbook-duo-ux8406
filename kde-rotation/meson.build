# The KDE Plasma part that turns the built-in screens: the program
# plasma-builtin-screen-rotation, its autostart entry, and its applet in the
# system tray.

# Cargo knows what has to be rebuilt, so it is asked every time. The program
# is in bindir: its command `restore` is for people.
plasma_builtin_screen_rotation = custom_target(
  'plasma-builtin-screen-rotation',
  output: 'plasma-builtin-screen-rotation',
  command: cargo_build + ['asus-ux8406-kde-rotation', '&&', 'cp', cargo_target_dir / 'release' / 'plasma-builtin-screen-rotation', '@OUTPUT@'],
  console: true,
  build_always_stale: true,
  install: true,
  install_dir: bindir,
  install_mode: 'rwxr-xr-x',
)

# The entry names the program by its full path. Plasma starts it in each
# session.
configure_file(
  input: 'autostart' / 'plasma-builtin-screen-rotation.desktop.in',
  output: 'plasma-builtin-screen-rotation.desktop',
  configuration: paths,
  install: true,
  install_dir: autostartdir,
  install_mode: 'rw-r--r--',
)

# The applet, under its id: Plasma finds it there, and the system tray shows
# it by itself. The package test compares the id with the metadata.
applet_id = 'io.github.cceelen.builtin-screen-rotation'
appletdir = datadir / 'plasma' / 'plasmoids' / applet_id
install_data('applet' / 'metadata.json', install_dir: appletdir, install_mode: 'rw-r--r--')
install_data(
  'applet' / 'contents' / 'ui' / 'main.qml',
  install_dir: appletdir / 'contents' / 'ui',
  install_mode: 'rw-r--r--',
)

install_man('man' / 'plasma-builtin-screen-rotation.1')

if install_licences
  install_data(
    licence_mit, 'THIRD-PARTY-NOTICES.txt',
    install_dir: licensedir / package_names['kde-rotation'],
    install_mode: 'rw-r--r--',
  )
endif

# The program against a fake KWin and a fake iio-sensor-proxy on a private
# bus: the tests start dbus-daemon.
test(
  'cargo test',
  cargo,
  args: ['test', '--release'] + cargo_args + ['--package', 'asus-ux8406-kde-rotation'],
  env: cargo_env,
  suite: 'plasma-builtin-screen-rotation',
  timeout: 900,
  # One cargo at a time: they share a target directory.
  is_parallel: false,
)
test(
  'applet',
  node,
  args: ['--test', meson.current_source_dir() / 'tests' / 'applet.test.js'],
  env: {'APPLET_ID': applet_id},
  suite: 'plasma-builtin-screen-rotation',
  timeout: 60,
)
