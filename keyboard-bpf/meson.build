# The HID-BPF program for the keyboard's hotkeys, and the hwdb entry that has
# udev-hid-bpf load it. The object is not a host binary, so Meson's C machinery
# is not used: clang is run by hand for the BPF target.
#
# The number in front of the object's name is the order udev-hid-bpf loads
# programs in; the hwdb entry names the object, and a test compares.

# The object's name, once: the build, the install and the hwdb check use it.
bpf_object_name = '0010-ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.o'

# Extra clang flags from the packager, set here (host CFLAGS do not apply to
# the BPF target and are not read).
bpf_extra_args = []

clang = find_program('clang')

# Tracing macros in libbpf's headers want to know the target architecture.
bpf_arch = {
  'x86_64': 'x86',
  'aarch64': 'arm64',
}.get(host_machine.cpu_family(), '')
if bpf_arch == ''
  error('keyboard-bpf: no BPF architecture name known for CPU family "@0@"; add it to meson.build'.format(host_machine.cpu_family()))
endif

# Debian and Ubuntu keep the architecture's part of the kernel headers
# (asm/) in a multiarch directory, which the BPF target does not search.
bpf_includes = []
cc = find_program('cc', required: false)
if cc.found()
  multiarch = run_command(cc, '-print-multiarch', check: false)
  if multiarch.returncode() == 0 and multiarch.stdout().strip() != ''
    bpf_includes += ['-idirafter', '/usr/include' / multiarch.stdout().strip()]
  endif
endif

# -ffile-prefix-map keeps source and build directories out of the debug
# information (distribution checks warn about build paths in binaries, and
# builds stay reproducible): paths are recorded relative to this directory
# and, for the shared headers, to the top of the tree. Clang takes the last
# matching map, so the part's own comes after the top's.
#
# The compiler runs with this source directory as its working directory: the
# BPF backend reads the source lines for the BTF line information from the
# recorded relative paths, and without them the object lacks the line text
# (the verifier's log loses its annotations). Meson runs commands in the
# build directory, hence the wrapper; all paths on its command line are
# absolute, and -MT gives the depfile the target name Ninja knows.
bpf_source_name = 'src/ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.c'
bpf_source = files(bpf_source_name)
run_in_dir = [
  python, '-c',
  'import os, subprocess, sys; os.chdir(sys.argv[1]); sys.exit(subprocess.call(sys.argv[2:]))',
  meson.current_source_dir(),
]
bpf_object = custom_target(
  'keyboard-bpf-object',
  input: bpf_source,
  output: bpf_object_name,
  depfile: bpf_object_name + '.d',
  command: run_in_dir + [
    clang, '-g', '-O2', '-Wall', '-Werror', '--target=bpf',
    '-I' + meson.current_source_dir() / 'include',
    bpf_includes,
    '-ffile-prefix-map=' + meson.project_source_root() + '=..',
    '-ffile-prefix-map=' + meson.current_source_dir() + '=.',
    '-fdebug-compilation-dir=.',
    '-Wno-microsoft-anon-tag', '-fms-extensions',
    '-D__TARGET_ARCH_' + bpf_arch,
    bpf_extra_args,
    '-MD', '-MF', meson.project_build_root() / '@DEPFILE@', '-MT', '@OUTPUT@',
    '-c', meson.current_source_dir() / bpf_source_name,
    '-o', meson.current_build_dir() / bpf_object_name,
  ],
  build_by_default: true,
  install: true,
  install_dir: firmwaredir / 'hid' / 'bpf',
  install_mode: 'rw-r--r--',
)

hwdb = files('udev/82-hid-bpf-asus-ux8406.hwdb')
install_data(
  hwdb,
  install_dir: udevdir / 'hwdb.d',
  install_mode: 'rw-r--r--',
)

if install_licences
  install_data(
    licence_gpl,
    install_dir: licensedir / package_names['keyboard-bpf'],
    install_mode: 'rw-r--r--',
  )
endif

# The program's report descriptors against the captured ones, and the hwdb
# against the object's name. Python must not leave bytecode next to the tests.
test(
  'rdesc',
  python,
  args: ['-B', files('tests/check_rdesc.py'), bpf_source],
  suite: 'keyboard-bpf',
  timeout: 60,
)
test(
  'hwdb',
  python,
  args: ['-B', files('tests/check_hwdb.py'), hwdb, bpf_object_name],
  suite: 'keyboard-bpf',
  timeout: 60,
)

# That the built object is what the loader expects: udev-hid-bpf reads the
# object and prints what it finds (read-only; nothing is loaded).
udev_hid_bpf = find_program('udev-hid-bpf', required: false)
if udev_hid_bpf.found()
  test(
    'inspect',
    udev_hid_bpf,
    args: ['inspect', bpf_object],
    suite: 'keyboard-bpf',
    timeout: 60,
  )
else
  message('keyboard-bpf: udev-hid-bpf not found, the "inspect" test is not registered')
endif
