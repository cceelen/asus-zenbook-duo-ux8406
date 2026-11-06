# The KDE Plasma part for the two display keys and the touchscreens: the
# program asus-ux8406-kde-keys, its autostart entry, and the KWin script that
# binds the keys.

# Cargo knows what has to be rebuilt, so it is asked every time.
asus_ux8406_kde_keys = custom_target(
  'asus-ux8406-kde-keys',
  output: 'asus-ux8406-kde-keys',
  command: cargo_build + ['asus-ux8406-kde-keys', '&&', 'cp', cargo_target_dir / 'release' / 'asus-ux8406-kde-keys', '@OUTPUT@'],
  console: true,
  build_always_stale: true,
  install: true,
  install_dir: libexecdir,
  install_mode: 'rwxr-xr-x',
)

# The entry names the program by its full path. Plasma starts it in each
# session; it does nothing on another model.
configure_file(
  input: 'autostart' / 'asus-ux8406-kde-keys.desktop.in',
  output: 'asus-ux8406-kde-keys.desktop',
  configuration: paths,
  install: true,
  install_dir: autostartdir,
  install_mode: 'rw-r--r--',
)

# The KWin script, under its id: KWin finds it there and loads it, as its
# metadata enables it by default. The package test compares the id with the
# metadata.
script_id = 'asus-zenbook-duo-ux8406-keys'
scriptdir = datadir / 'kwin' / 'scripts' / script_id
install_data('kwin-script' / 'metadata.json', install_dir: scriptdir, install_mode: 'rw-r--r--')
install_data(
  'kwin-script' / 'contents' / 'code' / 'main.js',
  install_dir: scriptdir / 'contents' / 'code',
  install_mode: 'rw-r--r--',
)

if install_licences
  install_data(
    licence_mit, 'THIRD-PARTY-NOTICES.txt',
    install_dir: licensedir / package_names['kde-keys'],
    install_mode: 'rw-r--r--',
  )
endif

# The program, against a fake KWin on a private bus: the tests
# start dbus-daemon.
# The Rust tests run in the one cargo test of meson.build.
rust_test_packages += 'asus-ux8406-kde-keys'
rust_test_suites += 'asus-ux8406-kde-keys'
foreach t : ['script', 'package']
  test(
    t,
    node,
    args: ['--test', meson.current_source_dir() / 'tests' / t + '.test.js'],
    env: {'SCRIPT_ID': script_id},
    suite: 'asus-ux8406-kde-keys',
    timeout: 60,
  )
endforeach
