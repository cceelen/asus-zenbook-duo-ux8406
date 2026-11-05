//! The guard against an installed system.

use asus_ux8406_kde_live_test::live::check;
use rstest::rstest;

use crate::support::Fixture;

const OVERLAY: &str = "proc /proc proc rw 0 0\noverlay / overlay rw,lowerdir=/run/rootfsbase 0 0\n";

fn root(cmdline: &str, mounts: &str) -> Fixture {
    let fixture = Fixture::new();

    fixture.file("/proc/cmdline", cmdline);
    fixture.file("/proc/mounts", mounts);
    fixture
}

#[rstest]
#[case(
    "BOOT_IMAGE=/vmlinuz root=live:CDLABEL=Fedora rd.live.image quiet",
    OVERLAY
)]
#[case("rd.live.image", "/dev/mapper/live-rw / ext4 rw 0 0\n")]
#[case(
    "root=live:LABEL=X",
    "/dev/loop1 / squashfs ro 0 0\n/dev/sda1 /x ext4 rw 0 0\n/dev/mapper/live-rw / ext4 rw 0 0\n"
)]
#[case("rd.live.image", "LiveOS_rootfs / ext4 rw 0 0\n")]
fn a_live_image_passes(#[case] cmdline: &str, #[case] mounts: &str) {
    let fixture = root(cmdline, mounts);

    assert_eq!(check(fixture.root.path()), Ok(()));
}

#[rstest]
#[case("root=UUID=1234 ro quiet", OVERLAY, "no rd.live.image")]
#[case("rd.live.image", "/dev/nvme0n1p3 / ext4 rw 0 0\n", "/dev/nvme0n1p3")]
#[case("rd.live.image", "proc /proc proc rw 0 0\n", "no root file system")]
#[case("", "", "no rd.live.image")]
fn an_installed_system_is_refused(
    #[case] cmdline: &str,
    #[case] mounts: &str,
    #[case] reason: &str,
) {
    let fixture = root(cmdline, mounts);
    let error = check(fixture.root.path()).unwrap_err();

    assert!(error.contains(reason), "{error}");
}
