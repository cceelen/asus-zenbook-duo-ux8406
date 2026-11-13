//! The report that the helper sends, as a HID device gets it: a virtual HID
//! device made with `/dev/uhid` has a hidraw node, and the kernel hands each
//! feature report that is set on the node to the program that made the
//! device. Making a virtual device needs root and `/dev/uhid`; without them
//! the tests say why they do nothing, and pass.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use asus_ux8406_keyboard_state::sleep::{REPORT_ID, REPORT_SIZE, report};
use asus_ux8406_keyboard_state::{HidrawSender, ReportSender};
use rstest::rstest;

/// The device of the kernel that makes virtual HID devices.
const UHID: &str = "/dev/uhid";
/// `O_NONBLOCK` on x86-64, Arm and RISC-V: a read that has nothing to give
/// returns at once, and the test can stop waiting.
const NONBLOCK: i32 = 0o4000;
/// How long the test waits for the kernel.
const WAIT: Duration = Duration::from_secs(5);

/// The types of events of `/dev/uhid`, as `linux/uhid.h` numbers them.
const UHID_DESTROY: u32 = 1;
const UHID_CREATE2: u32 = 11;
const UHID_SET_REPORT: u32 = 13;
const UHID_SET_REPORT_REPLY: u32 = 14;
/// The size of `struct uhid_event`: the type, and the largest request.
const EVENT_SIZE: usize = 4 + 4372;
/// The place of the fields of `struct uhid_create2_req`, after the type.
const CREATE_NAME: usize = 4;
const CREATE_DESCRIPTOR_SIZE: usize = 4 + 128 + 64 + 64;
const CREATE_BUS: usize = CREATE_DESCRIPTOR_SIZE + 2;
const CREATE_VENDOR: usize = CREATE_BUS + 2;
const CREATE_PRODUCT: usize = CREATE_VENDOR + 4;
const CREATE_DESCRIPTOR: usize = CREATE_PRODUCT + 4 + 4 + 4;
/// `BUS_VIRTUAL`.
const BUS_VIRTUAL: u16 = 6;
/// A vendor and a product that are not those of the keyboard: the rules of
/// the keyboard must not see the virtual device.
const VENDOR: u32 = 0x1209;
const PRODUCT: u32 = 0x0001;

/// The report descriptor: a vendor application with one feature report, ID
/// 0x5a and fifteen bytes of data.
const DESCRIPTOR: [u8; 23] = [
    0x06, 0x00, 0xff, // usage page: vendor
    0x09, 0x01, // usage
    0xa1, 0x01, // collection: application
    0x85, REPORT_ID, // report ID
    0x09, 0x01, // usage
    0x15, 0x00, // logical minimum 0
    0x26, 0xff, 0x00, // logical maximum 255
    0x75, 0x08, // 8 bits
    0x95, 0x0f, // 15 items
    0xb1, 0x02, // feature: data, variable, absolute
    0xc0, // end of the collection
];

/// Why the tests do nothing here, if they cannot run.
fn cannot_run() -> Option<String> {
    if !fs::metadata("/proc/self").is_ok_and(|own| own.uid() == 0) {
        return Some("not root: a virtual HID device can be made only by root".to_owned());
    }
    if fs::metadata(UHID).is_err() {
        return Some(format!("{UHID} is not there"));
    }
    None
}

/// A virtual HID device, destroyed when it goes out of scope.
struct Virtual {
    uhid: File,
    name: String,
}

impl Virtual {
    fn new(name: &str) -> Self {
        let mut uhid = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(NONBLOCK)
            .open(UHID)
            .expect("/dev/uhid");
        let name = format!("asus-ux8406-test-{}-{name}", std::process::id());
        let descriptor = DESCRIPTOR;
        let mut event = vec![0; EVENT_SIZE];

        event[..4].copy_from_slice(&UHID_CREATE2.to_le_bytes());
        event[CREATE_NAME..CREATE_NAME + name.len()].copy_from_slice(name.as_bytes());
        event[CREATE_DESCRIPTOR_SIZE..][..2]
            .copy_from_slice(&u16::try_from(descriptor.len()).unwrap().to_le_bytes());
        event[CREATE_BUS..][..2].copy_from_slice(&BUS_VIRTUAL.to_le_bytes());
        event[CREATE_VENDOR..][..4].copy_from_slice(&VENDOR.to_le_bytes());
        event[CREATE_PRODUCT..][..4].copy_from_slice(&PRODUCT.to_le_bytes());
        event[CREATE_DESCRIPTOR..][..descriptor.len()].copy_from_slice(&descriptor);
        uhid.write_all(&event).expect("a virtual HID device");
        Self { uhid, name }
    }

    /// The hidraw node of the device. The kernel makes it a moment after the
    /// device; its `uevent` file in sysfs names the device.
    fn node(&self) -> PathBuf {
        let wanted = format!("HID_NAME={}", self.name);
        let start = Instant::now();

        while start.elapsed() < WAIT {
            let found = fs::read_dir("/sys/class/hidraw")
                .into_iter()
                .flatten()
                .flatten()
                .find(|entry| {
                    fs::read_to_string(entry.path().join("device/uevent"))
                        .is_ok_and(|text| text.lines().any(|line| line == wanted))
                });

            if let Some(entry) = found {
                let node = PathBuf::from("/dev").join(entry.file_name());

                if node.exists() {
                    return node;
                }
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("no hidraw node for the virtual device {}", self.name);
    }

    /// The next event of the kernel with the type `wanted`, as bytes. Other
    /// events (start, open, close) are passed over.
    fn event(&mut self, wanted: u32) -> Vec<u8> {
        let start = Instant::now();
        let mut event = vec![0; EVENT_SIZE];

        while start.elapsed() < WAIT {
            match self.uhid.read(&mut event) {
                Ok(size) if size >= 4 => {
                    if event[..4] == wanted.to_le_bytes() {
                        return event;
                    }
                }
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("reading /dev/uhid: {error}"),
            }
        }
        panic!("no event of type {wanted} from /dev/uhid");
    }

    /// Answer the set-report event `event`: it is done.
    fn accept(&mut self, event: &[u8]) {
        let mut reply = vec![0; EVENT_SIZE];

        reply[..4].copy_from_slice(&UHID_SET_REPORT_REPLY.to_le_bytes());
        // The id of the request, and the error 0.
        reply[4..8].copy_from_slice(&event[4..8]);
        self.uhid.write_all(&reply).expect("a reply");
    }
}

impl Drop for Virtual {
    fn drop(&mut self) {
        let mut event = vec![0; EVENT_SIZE];

        event[..4].copy_from_slice(&UHID_DESTROY.to_le_bytes());
        let _ = self.uhid.write_all(&event);
    }
}

#[rstest]
#[case::off(0)]
#[case::dim(1)]
#[case::bright(3)]
fn the_device_gets_the_report_of_the_backlight(#[case] level: u8) {
    if let Some(reason) = cannot_run() {
        eprintln!("skipped, {reason}");
        return;
    }

    let mut device = Virtual::new(&format!("level{level}"));
    let node = device.node();
    let wanted = report(level);

    let sent = thread::scope(|scope| {
        let sender = scope.spawn(|| HidrawSender.send(&node, &wanted));
        // The kernel gives the report to the program that made the device,
        // and waits for its answer.
        let event = device.event(UHID_SET_REPORT);

        device.accept(&event);
        let size = usize::from(u16::from_le_bytes([event[4 + 4 + 2], event[4 + 4 + 3]]));

        assert_eq!(event[4 + 4], REPORT_ID, "the number of the report");
        assert_eq!(event[4 + 4 + 1], 0, "a feature report");
        assert_eq!(size, REPORT_SIZE);
        assert_eq!(&event[4 + 8..4 + 8 + size], wanted);
        sender.join().unwrap()
    });

    assert_eq!(sent, Ok(()));
}
