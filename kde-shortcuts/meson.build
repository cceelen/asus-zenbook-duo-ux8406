# Default global shortcuts for KDE Plasma: a file that kglobalacceld reads
# from its data directory. Nothing is compiled.
install_data(
  'asus-zenbook-duo-ux8406-shortcuts.desktop',
  install_dir: datadir / 'kglobalaccel',
  install_mode: 'rw-r--r--',
)

if install_licences
  install_data(
    licence_mit,
    install_dir: licensedir / package_names['kde-shortcuts'],
    install_mode: 'rw-r--r--',
  )
endif

test(
  'shortcuts',
  python,
  args: [meson.current_source_dir() / 'tests' / 'check_shortcuts.py'],
  suite: 'kde-shortcuts',
)
