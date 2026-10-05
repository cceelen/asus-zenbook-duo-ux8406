# asus-ux8406-tcc-guard: the service, what starts it (a systemd unit or an OpenRC
# script), and the example of its configuration file.

# Cargo knows what has to be rebuilt, so it is asked every time.
asus_ux8406_tcc_guard = custom_target(
  'asus-ux8406-tcc-guard',
  output: 'asus-ux8406-tcc-guard',
  command: cargo_build + ['asus-ux8406-tcc-guard', '&&', 'cp', cargo_target_dir / 'release' / 'asus-ux8406-tcc-guard', '@OUTPUT@'],
  console: true,
  build_always_stale: true,
  install: true,
  install_dir: sbindir,
  install_mode: 'rwxr-xr-x',
)

# Both name the program by its full path.
if get_option('init') == 'systemd'
  configure_file(
    input: 'systemd' / 'asus-ux8406-tcc-guard.service.in',
    output: 'asus-ux8406-tcc-guard.service',
    configuration: paths,
    install: true,
    install_dir: unitdir,
    install_mode: 'rw-r--r--',
  )
else
  # Not under the name it is installed as: in the build directory that name
  # is the program's.
  initd = configure_file(
    input: 'openrc' / 'asus-ux8406-tcc-guard.initd.in',
    output: 'asus-ux8406-tcc-guard.initd',
    configuration: paths,
  )
  install_data(
    initd,
    rename: 'asus-ux8406-tcc-guard',
    install_dir: initddir,
    install_mode: 'rwxr-xr-x',
  )
endif

install_man('man' / 'asus-ux8406-tcc-guard.8')

# Every setting with its default, explained. Not installed as the
# configuration itself: without a file the defaults apply.
install_data(
  'asus-ux8406-tcc-guard.toml.example',
  install_dir: docdir / 'asus-ux8406-tcc-guard',
  install_mode: 'rw-r--r--',
)

# The copyright and permission notices of the crates in the program, from
# cargo-about (about.toml at the top of the tree).
if install_licences
  install_data(
    licence_mit, 'THIRD-PARTY-NOTICES.txt',
    install_dir: licensedir / package_names['tcc-guard'],
    install_mode: 'rw-r--r--',
  )
endif

# The tests run twice: as built for installation, and with the fail points
# switched on. The fail points (src/fault.rs, the fail crate) let
# tests/scenarios/faults.rs cause system failures that no arrangement of
# files can; a build without the feature has none.
foreach name, features : {
  'cargo test': [],
  'cargo test, fault injection': ['--features', 'failpoints'],
}
  test(
    name,
    cargo,
    args: ['test', '--release'] + cargo_args + ['--package', 'asus-ux8406-tcc-guard'] + features,
    env: cargo_env,
    suite: 'asus-ux8406-tcc-guard',
    timeout: 1200,
    # One cargo at a time: they share a target directory.
    is_parallel: false,
  )
endforeach
