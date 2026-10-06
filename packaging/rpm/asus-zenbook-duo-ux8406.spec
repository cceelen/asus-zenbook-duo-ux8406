# Builds on Fedora, RHEL and openSUSE.
Name:           asus-zenbook-duo-ux8406
Version:        0.1.0
Release:        1%{?dist}
Summary:        User-space support for the ASUS Zenbook Duo UX8406
License:        MIT AND GPL-2.0-only
URL:            https://github.com/cceelen/asus-zenbook-duo-ux8406
BugURL:         %{url}/issues
# The archive of the release: every file of the tree and the vendored crates.
# rpmbuild takes the file of that name from its sources directory; a build
# service fetches it from here.
Source0:        %{url}/releases/download/v%{version}/%{name}-%{version}.tar.gz

# The machine is x86-64 only.
ExclusiveArch:  x86_64

BuildRequires:  meson >= 1.1
BuildRequires:  cargo
BuildRequires:  clang
BuildRequires:  libbpf-devel
BuildRequires:  nodejs
BuildRequires:  python3
BuildRequires:  systemd-rpm-macros
%if 0%{?suse_version}
BuildRequires:  ninja
BuildRequires:  glib2-tools
BuildRequires:  linux-glibc-devel
%else
BuildRequires:  ninja-build
BuildRequires:  glib2
BuildRequires:  kernel-headers
%endif

# The HID-BPF object is not a host binary and must keep its sections: the
# strip scripts are off. The debuginfo step strips the Rust programs; it
# takes executables only.
%global __brp_strip %{nil}
%global __brp_strip_comment_note %{nil}
%global __brp_strip_static_archive %{nil}
%global __brp_strip_lto %{nil}

%global udevdir %{_prefix}/lib/udev
# Where the distribution does not turn Cargo's stripping off (openSUSE,
# RHEL), the programs carry no debug information and the debugsource package
# would be empty.
%if 0%{?suse_version} || 0%{?rhel}
%undefine _debugsource_packages
%endif

%global guarddocdir %{_docdir}/asus-zenbook-duo-ux8406-tcc-guard

%description
Small pieces that make the ASUS Zenbook Duo UX8406 behave under Linux
as it does under its vendor's software, without kernel changes.

%package -n asus-zenbook-duo-ux8406-second-screen
Summary:        Lower panel of the Zenbook Duo UX8406 off while the keyboard lies on it
License:        MIT
Requires:       udev

%description -n asus-zenbook-duo-ux8406-second-screen
While the detachable keyboard lies on the lower built-in panel, the panel's
display connector is reported as disconnected, so that the desktop treats it
like an unplugged monitor and uses its saved layout for the upper panel alone.
Taking the keyboard off brings the panel back. The lower panel is also kept at
the brightness of the upper one, which is the only one desktops set. udev
rules run the helper when the keyboard arrives on or leaves its dock port and
when the upper panel's brightness changes.

Does nothing on any other model.

%package -n asus-zenbook-duo-ux8406-tcc-guard
Summary:        CPU thermal-offset guard for the ASUS Zenbook Duo UX8406
License:        MIT
%{?systemd_requires}

%description -n asus-zenbook-duo-ux8406-tcc-guard
A root service that raises the CPU thermal offset when the embedded controller
of the Zenbook Duo UX8406 warns of heat, and takes it back when the warning
is over. The service is enabled when the package is installed (preset). It
logs one line and exits on any other model. An optional configuration file,
/etc/asus-ux8406-tcc-guard.toml, overrides the defaults; an example is in the
documentation directory.

%package -n asus-zenbook-duo-ux8406-keyboard-bpf
Summary:        HID-BPF program for the hotkeys of the Zenbook Duo UX8406 keyboard
License:        GPL-2.0-only
BuildArch:      noarch
Requires:       udev-hid-bpf

%description -n asus-zenbook-duo-ux8406-keyboard-bpf
Makes the function row of the detachable keyboard (USB 0b05:1bf2, Bluetooth
0b05:1bf3) work with the generic HID driver of the kernel: display brightness,
microphone mute, emoji, MyASUS, the two display keys, the keyboard lighting
and Fn+Esc. Loaded by udev-hid-bpf when the keyboard connects.

%package -n gnome-shell-extension-asus-zenbook-duo-ux8406-keys
Summary:        GNOME Shell extension for the display keys of the Zenbook Duo UX8406
License:        MIT
BuildArch:      noarch
Requires:       gnome-shell >= 45
Recommends:     asus-zenbook-duo-ux8406-keyboard-bpf

%description -n gnome-shell-extension-asus-zenbook-duo-ux8406-keys
The key right of F12 switches the lower panel on and off, and F8 swaps the
windows of the two panels. The keyboard delivers these keys with
asus-zenbook-duo-ux8406-keyboard-bpf. To be enabled per user:
gnome-extensions enable asus-zenbook-duo-ux8406-keys@cceelen.github.io

%prep
%autosetup

%build
# The macros give the directories: Fedora ships no udev.pc and systemd.pc for
# Meson to ask, and defines no macro for the udev directory itself.
# -Dlicensedir installs each package's licence text and the notices of the two
# Rust programs to %%{_defaultlicensedir}/<package name>/ (%%{_licensedir}
# itself is that directory plus this spec's name).
%meson \
    -Dudevdir=%{udevdir} \
    -Dsystemdsystemunitdir=%{_unitdir} \
    -Dfirmwaredir=%{_prefix}/lib/firmware \
    -Dlicensedir=%{_defaultlicensedir}
%meson_build

%check
# Run as the unprivileged user rpmbuild runs as; the Rust tests build first.
%meson_test -t 3

%install
%meson_install
# Meson's own record of the licences installed; the packages carry the texts.
rm -f %{buildroot}%{_defaultlicensedir}/depmf.json
# Meson installs the example below share/doc (openSUSE: doc/packages);
# it goes to the guard package's own documentation directory.
install -d %{buildroot}%{guarddocdir}
mv %{buildroot}%{_datadir}/doc/asus-ux8406-tcc-guard/asus-ux8406-tcc-guard.toml.example \
    %{buildroot}%{guarddocdir}/
rmdir %{buildroot}%{_datadir}/doc/asus-ux8406-tcc-guard
# The preset is generated here: one line, and the spec stays the only file the
# build needs besides the archive.
install -d %{buildroot}%{_presetdir}
echo 'enable asus-ux8406-tcc-guard.service' \
    > %{buildroot}%{_presetdir}/90-asus-zenbook-duo-ux8406-tcc-guard.preset

%post -n asus-zenbook-duo-ux8406-second-screen
%udev_rules_update
# Bring the panel in line with where the keyboard is, and with the upper
# panel's brightness, right now.
%{_libexecdir}/asus-ux8406-second-screen dock >/dev/null 2>&1 || :
%{_libexecdir}/asus-ux8406-second-screen brightness >/dev/null 2>&1 || :

%preun -n asus-zenbook-duo-ux8406-second-screen
# On removal, not on upgrade: a panel left forced off would stay off until
# the next boot.
if [ "$1" -eq 0 ]; then
    %{_libexecdir}/asus-ux8406-second-screen release >/dev/null 2>&1 || :
fi

%postun -n asus-zenbook-duo-ux8406-second-screen
%udev_rules_update

# Stopping the service runs "asus-ux8406-tcc-guard restore" (ExecStopPost). The macros do
# nothing harmful where systemd is not running, as in a container.
%if 0%{?suse_version}
%pre -n asus-zenbook-duo-ux8406-tcc-guard
%service_add_pre asus-ux8406-tcc-guard.service

%post -n asus-zenbook-duo-ux8406-tcc-guard
%service_add_post asus-ux8406-tcc-guard.service

%preun -n asus-zenbook-duo-ux8406-tcc-guard
%service_del_preun asus-ux8406-tcc-guard.service

%postun -n asus-zenbook-duo-ux8406-tcc-guard
%service_del_postun asus-ux8406-tcc-guard.service
%else
%post -n asus-zenbook-duo-ux8406-tcc-guard
%systemd_post asus-ux8406-tcc-guard.service

%preun -n asus-zenbook-duo-ux8406-tcc-guard
%systemd_preun asus-ux8406-tcc-guard.service

%postun -n asus-zenbook-duo-ux8406-tcc-guard
%systemd_postun_with_restart asus-ux8406-tcc-guard.service
%endif

# Fedora's macro expands to nothing (hwdb is updated by a file trigger of
# systemd); an empty scriptlet would only be reported by rpmlint.
%if 0%{?suse_version}
%post -n asus-zenbook-duo-ux8406-keyboard-bpf
%udev_hwdb_update

%postun -n asus-zenbook-duo-ux8406-keyboard-bpf
%udev_hwdb_update
%endif

%files -n asus-zenbook-duo-ux8406-second-screen
%license %{_defaultlicensedir}/asus-zenbook-duo-ux8406-second-screen
%doc README.md
%{_libexecdir}/asus-ux8406-second-screen
%{_udevrulesdir}/90-asus-ux8406-second-screen.rules

%files -n asus-zenbook-duo-ux8406-tcc-guard
%license %{_defaultlicensedir}/asus-zenbook-duo-ux8406-tcc-guard
%doc README.md
%doc %{guarddocdir}/asus-ux8406-tcc-guard.toml.example
%{_sbindir}/asus-ux8406-tcc-guard
%{_mandir}/man8/asus-ux8406-tcc-guard.8*
%{_unitdir}/asus-ux8406-tcc-guard.service
%{_presetdir}/90-asus-zenbook-duo-ux8406-tcc-guard.preset

%files -n asus-zenbook-duo-ux8406-keyboard-bpf
%license %{_defaultlicensedir}/asus-zenbook-duo-ux8406-keyboard-bpf
%doc README.md
%{_prefix}/lib/firmware/hid/bpf/0010-ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.o
%{_udevhwdbdir}/82-hid-bpf-asus-ux8406.hwdb
# openSUSE checks that every directory of a package has an owner; no package
# there owns these.
%if 0%{?suse_version}
%dir %{_prefix}/lib/firmware/hid
%dir %{_prefix}/lib/firmware/hid/bpf
%dir %{udevdir}/hwdb.d
%endif

%files -n gnome-shell-extension-asus-zenbook-duo-ux8406-keys
%license %{_defaultlicensedir}/gnome-shell-extension-asus-zenbook-duo-ux8406-keys
%doc README.md
%{_datadir}/gnome-shell/extensions/asus-zenbook-duo-ux8406-keys@cceelen.github.io/
%if 0%{?suse_version}
%dir %{_datadir}/gnome-shell
%dir %{_datadir}/gnome-shell/extensions
%endif

%changelog
* Mon Oct 05 2026 Christian Ceelen - 0.1.0-1
- Refer to CHANGELOG.md.
