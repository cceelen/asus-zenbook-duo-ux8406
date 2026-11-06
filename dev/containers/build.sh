#!/bin/sh
# The packages of one distribution, from the source archive of a release
# (meson dist), and the linter of the distribution on the recipe and on the
# packages. Run as the user that build-deps.sh makes, without network. The
# recipes come from packaging/ of this tree. A linter error, or a warning
# that the recipe does not accept, fails the build.
#
# Without network: the Dockerfiles run it with RUN --network=none. The job
# containers of .github/workflows/packages.yml have network; there,
# CARGO_NET_OFFLINE=true makes cargo fail on any crate that is not in vendor/
# of the archive.
#
#   build.sh fedora|rhel|opensuse|debian|arch|alpine ARCHIVE OUT
#
# ARCHIVE is asus-zenbook-duo-ux8406-<version>.tar.gz; the packages go to OUT.
set -eu

family=${1:?which distribution}
archive=$(realpath "${2:?which archive}")
mkdir -p "${3:?which directory}"
out=$(realpath "$3")
tree=$(realpath "$(dirname "$0")/../..")
work=$(mktemp -d)

case "$family" in
  fedora|rhel|opensuse)
    rpmbuild --define "_topdir $work" --define "_sourcedir ${archive%/*}" \
      -ba "$tree/packaging/rpm/asus-zenbook-duo-ux8406.spec"
    cp "$work"/RPMS/*/*.rpm "$work"/SRPMS/*.rpm "$out"/
    # The full output is printed. rpmlint's own exit status depends on a
    # badness threshold that differs between distributions: the build fails
    # on any error, and on any other failure of rpmlint.
    rc=0
    rpmlint -r "$tree/packaging/rpm/asus-zenbook-duo-ux8406.rpmlintrc" \
      "$tree/packaging/rpm/asus-zenbook-duo-ux8406.spec" "$out"/*.rpm \
      > "$work/rpmlint.log" 2>&1 || rc=$?
    cat "$work/rpmlint.log"
    if grep -q ': E: ' "$work/rpmlint.log"; then echo "rpmlint: errors" >&2; exit 1; fi
    if [ "$rc" -ne 0 ] && [ "$rc" -ne 64 ]; then echo "rpmlint failed: $rc" >&2; exit 1; fi
    ;;
  debian)
    version=$(dpkg-parsechangelog -l "$tree/packaging/debian/changelog" -S Version | cut -d- -f1)
    cd "$work"
    cp "$archive" "asus-zenbook-duo-ux8406_$version.orig.tar.gz"
    tar -xzf "asus-zenbook-duo-ux8406_$version.orig.tar.gz"
    cp -r "$tree/packaging/debian" "asus-zenbook-duo-ux8406-$version/debian"
    # An artifact does not keep the file modes.
    chmod +x "asus-zenbook-duo-ux8406-$version/debian/rules"
    (cd "asus-zenbook-duo-ux8406-$version" && dpkg-buildpackage -us -uc -rfakeroot)
    # The Debian profile on Ubuntu too: the recipe is Debian's, and its
    # changelog names unstable, which Ubuntu's profile rejects.
    lintian --profile debian --info --display-info --pedantic --fail-on error \
      "asus-zenbook-duo-ux8406_$version"-*_amd64.changes
    find "$work" -maxdepth 1 -type f -exec cp {} "$out"/ \;
    ;;
  arch)
    # --nodeps: udev-hid-bpf, a run-time dependency, is not in the official
    # repositories. namcap has no exit status for findings: an error line
    # (' E: ') fails the build, and so does a warning (' W: ') that is not
    # in packaging/arch/namcap-accepted.txt. The rule splitpkgmakedeps is
    # left out: it wants every run-time dependency of the split packages as
    # a makedepends, and these cannot be: udev-hid-bpf is not in the
    # official repositories (makepkg -s would fail), and gnome-shell, kwin,
    # plasma-workspace and plasma-desktop are a whole desktop that the build
    # does not use.
    cd "$work"
    cp "$tree"/packaging/arch/* "$archive" .
    makepkg --noconfirm --nodeps --nocolor
    cp ./*.pkg.tar.zst "$out"/
    namcap -e splitpkgmakedeps PKGBUILD > namcap.txt 2>&1
    for p in ./*.pkg.tar.zst; do namcap "$p" >> namcap.txt 2>&1; done
    cat namcap.txt
    if grep -q ' E: ' namcap.txt; then echo 'namcap reported errors' >&2; exit 1; fi
    grep -v '^#' namcap-accepted.txt > accepted.txt
    if grep ' W: ' namcap.txt | grep -v -x -F -f accepted.txt; then
      echo 'namcap reported warnings that are not accepted' >&2
      exit 1
    fi
    ;;
  alpine)
    mkdir "$work/pkg" "$work/src"
    cp "$archive" "$work/src/"
    cd "$work/pkg"
    cp "$tree"/packaging/alpine/* .
    SRCDEST="$work/src" abuild checksum
    apkbuild-lint APKBUILD
    abuild validate
    abuild -d -m -P "$work/repo" -s "$work/src"
    find "$work/repo" -name '*.apk' -exec cp {} "$out"/ \;
    ;;
  *)
    echo "unknown distribution: $family" >&2
    exit 2
    ;;
esac
