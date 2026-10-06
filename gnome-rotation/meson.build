# The GNOME Shell extension that turns the built-in panels. Nothing is compiled;
# the files are installed as they are.

# The one place that names the extension. It is the install directory, and
# the package test compares it with the uuid in metadata.json.
uuid = 'builtin-screen-rotation@cceelen.github.io'
extdir = datadir / 'gnome-shell' / 'extensions' / uuid

files = ['extension.js', 'rotation.js', 'sensor.js', 'metadata.json']
foreach f : files
  install_data('extension' / f, install_dir: extdir, install_mode: 'rw-r--r--')
endforeach
# What the keys extension uses too. In extension/ these two are links, for the
# tests and the manual tools; the files themselves are installed.
install_data(
  '..' / 'gnome-common' / 'display.js',
  '..' / 'gnome-common' / 'state.js',
  install_dir: extdir,
  install_mode: 'rw-r--r--',
)

if install_licences
  install_data(
    licence_mit,
    install_dir: licensedir / package_names['gnome-rotation'],
    install_mode: 'rw-r--r--',
  )
endif

# The tests need no GNOME session; tests/rotate.js does and is run by hand,
# so the files are named (a directory would pick it up). Node's test runner
# writes nothing into the source tree.
node = find_program('node')
foreach t : ['rotation', 'package']
  test(
    t,
    node,
    args: ['--test', meson.current_source_dir() / 'tests' / t + '.test.js'],
    env: {'EXTENSION_UUID': uuid},
    suite: 'gnome-rotation',
    timeout: 60,
  )
endforeach
