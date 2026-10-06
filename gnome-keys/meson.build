# The GNOME Shell extension for the two display keys. Nothing is compiled
# but the GSettings schema; the rest is installed as it is.

# The one place that names the extension. It is the install directory, and
# the package test compares it with the uuid in metadata.json.
uuid = 'asus-zenbook-duo-ux8406-keys@cceelen.github.io'
extdir = datadir / 'gnome-shell' / 'extensions' / uuid

files = ['extension.js', 'layout.js', 'metadata.json']
foreach f : files
  install_data('extension' / f, install_dir: extdir, install_mode: 'rw-r--r--')
endforeach
# The files in ../gnome-common. In extension/ these two are links, for the
# tests and the manual tool; the files themselves are installed.
install_data(
  '..' / 'gnome-common' / 'display.js',
  '..' / 'gnome-common' / 'state.js',
  install_dir: extdir,
  install_mode: 'rw-r--r--',
)

schema_xml = files('extension/schemas/org.gnome.shell.extensions.asus-zenbook-duo-ux8406-keys.gschema.xml')
install_data(schema_xml, install_dir: extdir / 'schemas', install_mode: 'rw-r--r--')

# An extension carries its compiled schema in its own schemas/ directory,
# where GNOME Shell's getSettings() looks for it. The gnome module's
# compile_schemas() targets the system schema directories and cannot install
# into a directory of our choice, so glib-compile-schemas is run directly.
# The output goes to the build directory.
glib_compile_schemas = find_program('glib-compile-schemas')
custom_target(
  'gschemas.compiled',
  input: schema_xml,
  output: 'gschemas.compiled',
  command: [
    glib_compile_schemas, '--strict', '--targetdir=@OUTDIR@',
    meson.current_source_dir() / 'extension' / 'schemas',
  ],
  build_by_default: true,
  install: true,
  install_dir: extdir / 'schemas',
  install_mode: 'rw-r--r--',
)

if install_licences
  install_data(
    licence_mit,
    install_dir: licensedir / package_names['gnome-keys'],
    install_mode: 'rw-r--r--',
  )
endif

# The tests need no GNOME session; tests/verify.js does and is run by hand,
# so the files are named (a directory would pick it up). Node's test runner
# writes nothing into the source tree.
node = find_program('node')
foreach t : ['layout', 'package']
  test(
    t,
    node,
    args: ['--test', meson.current_source_dir() / 'tests' / t + '.test.js'],
    env: {'EXTENSION_UUID': uuid},
    suite: 'gnome-keys',
    timeout: 60,
  )
endforeach
