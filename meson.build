# The parts, each in its own directory with its own meson.build:
#   asus-ux8406-second-screen     the helper that takes the lower panel off the desktop while
#                 the keyboard lies on it, and its udev rule
#   asus-ux8406-tcc-guard     the service that raises the CPU's thermal offset when the
#                 embedded controller warns, with its systemd unit or an
#                 OpenRC script
#   keyboard-bpf  the HID-BPF program for the keyboard's hotkeys, and the
#                 hwdb entry that has udev-hid-bpf load it
#   keyboard-state the helper that carries the keyboard's backlight and
#                 function row mode from one connection to the next, and
#                 its udev rule; goes with keyboard-bpf
#   gnome-keys    the GNOME Shell extension for the two display keys
#   gnome-rotation the GNOME Shell extension that turns the built-in panels
# This file sets the install directories and the cargo command; the parts
# do the rest.
#
#   meson setup build && meson compile -C build && meson test -C build
#   meson install -C build --destdir /some/stage
#
# `meson test` must run as an ordinary user: some tests check file
# permissions, which root ignores.
#
# The Rust programs are one cargo workspace (Cargo.toml here) that Meson has
# cargo build. A source archive from `meson dist` carries the crates in
# vendor/ and builds without network.

project(
  'asus-zenbook-duo-ux8406',
  # Also in Cargo.toml and in every recipe.
  version: '0.3.0',
  license: 'MIT AND GPL-2.0-only',
  # No license_files: the parts install the text that applies to each
  # package, not every text for the project as a whole.
  meson_version: '>= 1.1.0',
  default_options: ['buildtype=release'],
)

fs = import('fs')

prefix = get_option('prefix')
libexecdir = prefix / get_option('libexecdir')
sbindir = prefix / get_option('sbindir')
datadir = prefix / get_option('datadir')
docdir = datadir / 'doc'

udevdir = get_option('udevdir')
if udevdir == ''
  udev = dependency('udev', required: false)
  udevdir = udev.found() ? udev.get_variable('udevdir') : prefix / 'lib' / 'udev'
endif

unitdir = get_option('systemdsystemunitdir')
if unitdir == ''
  systemd = dependency('systemd', required: false)
  unitdir = systemd.found() ? systemd.get_variable('systemdsystemunitdir') : prefix / 'lib' / 'systemd' / 'system'
endif

firmwaredir = get_option('firmwaredir')
if firmwaredir == ''
  firmwaredir = prefix / 'lib' / 'firmware'
endif

initddir = get_option('initddir')

# Licence texts, installed when Meson's own option licensedir is set
# (-Dlicensedir=share/licenses): each package's text and third-party notices
# in a subdirectory of its own. Empty (the default): not installed; the
# packaging supplies them. The package names are defined here only.
install_licences = get_option('licensedir') != ''
licensedir = prefix / get_option('licensedir')
licence_mit = files('LICENSES/MIT.txt')
licence_gpl = files('LICENSES/GPL-2.0-only.txt')
package_names = {
  'second-screen': 'asus-zenbook-duo-ux8406-second-screen',
  'tcc-guard': 'asus-zenbook-duo-ux8406-tcc-guard',
  'keyboard-bpf': 'asus-zenbook-duo-ux8406-keyboard-bpf',
  'gnome-keys': 'gnome-shell-extension-asus-zenbook-duo-ux8406-keys',
  'gnome-rotation': 'gnome-shell-extension-builtin-screen-rotation',
}

# What templates (*.in) are filled in with.
paths = configuration_data({
  'LIBEXECDIR': libexecdir,
  'SBINDIR': sbindir,
})

python = find_program('python3')
# The tests of the two GNOME Shell extensions run on node.
node = find_program('node', required: get_option('gnome') or get_option('rotation'))
# What the two extensions share. In each extension/ directory these are links,
# for the tests and the manual tools; each part installs the files themselves.
gnome_common = files('gnome-common/display.js', 'gnome-common/state.js')

# Cargo. One target directory inside the build directory, so that nothing is
# written into the source tree.
rust = get_option('screen') or get_option('guard') or get_option('keyboard')
cargo = find_program('cargo', required: rust)
offline = get_option('offline').enabled() or (get_option('offline').auto() and fs.is_dir('vendor'))
cargo_args = ['--locked', '--manifest-path', meson.project_source_root() / 'Cargo.toml']
# A source archive has a cargo configuration that points cargo at vendor/.
# Cargo looks for it from its working directory, the build directory, so the
# file is passed explicitly.
if fs.exists('.cargo' / 'config.toml')
  cargo_args += ['--config', meson.project_source_root() / '.cargo' / 'config.toml']
endif
if offline
  cargo_args += '--offline'
endif
cargo_target_dir = meson.project_build_root() / 'cargo'
cargo_env = environment()
cargo_env.set('CARGO_TARGET_DIR', cargo_target_dir)
# The command that builds one program; the package name and the copy to
# '@OUTPUT@' follow.
cargo_build = [
  'env', 'CARGO_TARGET_DIR=' + cargo_target_dir,
  cargo, 'build', '--release',
] + cargo_args + ['--package']

meson.add_dist_script('build-aux' / 'dist-vendor.sh')

if get_option('screen')
  subdir('second-screen')
endif
if get_option('guard')
  subdir('tcc-guard')
endif
if get_option('keyboard')
  subdir('keyboard-bpf')
  subdir('keyboard-state')
endif
if get_option('gnome')
  subdir('gnome-keys')
endif
if get_option('rotation')
  subdir('gnome-rotation')
endif

summary({
  'second-screen': get_option('screen'),
  'tcc-guard': get_option('guard'),
  'keyboard-bpf': get_option('keyboard'),
  'gnome-keys': get_option('gnome'),
  'gnome-rotation': get_option('rotation'),
}, section: 'Parts', bool_yn: true)
summary({
  'init': get_option('init'),
  'udev': udevdir,
  'systemd units': unitdir,
  'firmware': firmwaredir,
  'licences': install_licences ? licensedir : 'not installed',
  'cargo offline': offline,
}, section: 'Settings', bool_yn: true)
