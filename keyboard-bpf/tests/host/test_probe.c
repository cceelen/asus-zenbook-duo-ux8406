// probe: which interface is taken, the start-up commands and the udev
// properties that steer them.
#include <string.h>

#include "captures.h"
#include "fake_kernel.h"
#include "test_support.h"

enum {
  kReportId = 0x5a,
  kUsbSize = 90,
  kUsbOffset = 57,
  kBtSize = 257,
  kBtOffset = 167,
  kFeatureBytes = 16,
  kFeatureReport = 2,
  kSetReport = 9,
  kFnCommand = 0xd0,
  kFnEsc = 0x4e,
  kBacklightCommand = 0xba,
  kBacklightParameter1 = 0xc5,
  kBacklightParameter2 = 0xc4,
  kHid = 42,
  kUnset = 1234, /* what retval holds before the probe sets it */
  kStale = 9,    /* a state that the probe must replace */
};

static const unsigned char kSignature[] = {
    0x06, 0x31, 0xff, 0x09, 0x76, 0xa1, 0x01, 0x85, kReportId,
};

/* An interface: the size of its descriptor and where the hotkey signature is
 * in it (offset 0: nowhere). */
struct Shape {
  unsigned int size;
  unsigned int offset;
};

static const struct Shape kUsb = {kUsbSize, kUsbOffset};
static const struct Shape kBt = {kBtSize, kBtOffset};
static const struct Shape kUsbNoSignature = {kUsbSize, 0};
static const struct Shape kBtNoSignature = {kBtSize, 0};
static const struct Shape kUsbWrongSize = {kUsbSize + 1, kUsbOffset};
static const struct Shape kBtWrongSize = {kBtSize - 1, kBtOffset};
static const struct Shape kEmpty = {0, 0};
static const struct Shape kUsbBtOffset = {kUsbSize, kBtOffset};
static const struct Shape kBtUsbOffset = {kBtSize, kUsbOffset};

static void make_args(struct hid_bpf_probe_args* args,
                      const struct Shape* shape) {
  memset(args, 0, sizeof(*args));
  args->hid = kHid;
  args->rdesc_size = shape->size;
  args->retval = kUnset;
  if (shape->offset != 0) {
    memcpy(args->rdesc + shape->offset, kSignature, sizeof(kSignature));
  }
}

static int run_probe(const struct Shape* shape) {
  struct hid_bpf_probe_args args;

  make_args(&args, shape);
  CHECK_EQ(probe(&args), 0);
  return args.retval;
}

static void check_no_effects() {
  CHECK_EQ(fake_kernel()->allocs, 0);
  CHECK_EQ(fake_kernel()->hw_count, 0);
  CHECK_EQ(fake_contexts_in_use(), 0);
}

static void no_hotkeys() {
  const struct Shape* shapes[] = {
      &kUsbNoSignature, &kBtNoSignature, &kUsbWrongSize, &kBtWrongSize,
      &kEmpty,          &kUsbBtOffset,   &kBtUsbOffset,
  };

  for (size_t i = 0; i < sizeof(shapes) / sizeof(shapes[0]); i++) {
    CHECK_EQ(run_probe(shapes[i]), -EINVAL);
  }
  CHECK_EQ(fake_kernel()->init_calls, 0);
  check_no_effects();
}

static void signature_with_one_byte_wrong() {
  for (size_t i = 0; i < sizeof(kSignature); i++) {
    struct hid_bpf_probe_args usb;
    struct hid_bpf_probe_args blue;

    make_args(&usb, &kUsb);
    make_args(&blue, &kBt);
    usb.rdesc[kUsbOffset + i]++;
    blue.rdesc[kBtOffset + i]++;
    CHECK_EQ(probe(&usb), 0);
    CHECK_EQ(usb.retval, -EINVAL);
    CHECK_EQ(probe(&blue), 0);
    CHECK_EQ(blue.retval, -EINVAL);
  }
  check_no_effects();
}

static void real_captures_are_taken() {
  struct hid_bpf_probe_args args;

  make_args(&args, &kEmpty);
  args.rdesc_size = sizeof(kUsbCapture);
  memcpy(args.rdesc, kUsbCapture, sizeof(kUsbCapture));
  CHECK_EQ(probe(&args), 0);
  CHECK_EQ(args.retval, 0);
  make_args(&args, &kEmpty);
  args.rdesc_size = sizeof(kBtCapture);
  memcpy(args.rdesc, kBtCapture, sizeof(kBtCapture));
  CHECK_EQ(probe(&args), 0);
  CHECK_EQ(args.retval, 0);
}

static void first_init_fails() {
  fake_kernel()->init_fail_at = 0;
  fake_kernel()->init_error = -ENOMEM;
  CHECK(run_probe(&kUsb) != 0);
  CHECK_EQ(fake_kernel()->init_calls, 1);
  check_no_effects();
}

static void second_init_fails() {
  fake_kernel()->init_fail_at = 1;
  fake_kernel()->init_error = -ENOSPC;
  CHECK(run_probe(&kBt) != 0);
  CHECK_EQ(fake_kernel()->init_calls, 2);
  check_no_effects();
}

static void no_context() {
  fake_kernel()->alloc_fail = 1;
  CHECK_EQ(run_probe(&kUsb), -EINVAL);
  CHECK_EQ(fake_kernel()->releases, 0);
  CHECK_EQ(fake_kernel()->hw_count, 0);
}

static void no_state() {
  fake_kernel()->map_fail = 1;
  CHECK_EQ(run_probe(&kUsb), -EINVAL);
  CHECK_EQ(fake_kernel()->allocs, 1);
  CHECK_EQ(fake_kernel()->releases, 1);
  CHECK_EQ(fake_kernel()->bad_releases, 0);
  CHECK_EQ(fake_contexts_in_use(), 0);
  CHECK_EQ(fake_kernel()->hw_count, 0);
}

static void check_released_once() {
  CHECK_EQ(fake_kernel()->allocs, 1);
  CHECK_EQ(fake_kernel()->releases, 1);
  CHECK_EQ(fake_kernel()->bad_releases, 0);
  CHECK_EQ(fake_contexts_in_use(), 0);
  CHECK_EQ(fake_kernel()->alloc_hid[0], kHid);
}

struct Feature {
  __u8 bytes[kFeatureBytes];
};

static struct Feature fn_report(__u8 mode) {
  return (struct Feature){
      {
          kReportId,
          kFnCommand,
          kFnEsc,
          mode,
      },
  };
}

static struct Feature backlight_report(__u8 level) {
  return (struct Feature){
      {
          kReportId,
          kBacklightCommand,
          kBacklightParameter1,
          kBacklightParameter2,
          level,
      },
  };
}

/* The index-th request: a feature report to the context that was allocated,
 * of the 16 bytes. */
static void check_request(int index, struct Feature want) {
  const struct FakeHwCall* call = &fake_kernel()->hw[index];

  CHECK_EQ(call->size, kFeatureBytes);
  CHECK_EQ(call->type, kFeatureReport);
  CHECK_EQ(call->reqtype, kSetReport);
  CHECK_BYTES(call->report, want.bytes, kFeatureBytes);
  CHECK(call->ctx == &fake_kernel()->pool[0]);
}

static void success_without_properties() {
  kbd_view()->fn_lock = kStale;
  kbd_view()->backlight = kStale;
  CHECK_EQ(run_probe(&kUsb), 0);
  CHECK_EQ(fake_kernel()->init_calls, 2);
  CHECK_EQ(fake_kernel()->hw_count, 1);
  check_request(0, fn_report(1));
  CHECK_EQ(kbd_view()->fn_lock, 1);
  CHECK_EQ(kbd_view()->backlight, 0);
  check_released_once();
}

static void success_with_properties() {
  set_property(UDEV_PROP_ASUS_UX8406_KBD_FN_LOCK, "0");
  set_property(UDEV_PROP_ASUS_UX8406_KBD_BACKLIGHT, "2");
  CHECK_EQ(run_probe(&kBt), 0);
  CHECK_EQ(fake_kernel()->hw_count, 2);
  check_request(0, fn_report(0));
  check_request(1, backlight_report(2));
  CHECK_EQ(kbd_view()->fn_lock, 0);
  CHECK_EQ(kbd_view()->backlight, 2);
  check_released_once();
}

static void refused_requests_do_not_fail_the_probe() {
  set_property(UDEV_PROP_ASUS_UX8406_KBD_BACKLIGHT, "1");
  fake_kernel()->hw_results[0] = -EIO;
  fake_kernel()->hw_results[1] = -EIO;
  CHECK_EQ(run_probe(&kUsb), 0);
  CHECK_EQ(fake_kernel()->hw_count, 2);
  check_request(1, backlight_report(1));
  check_released_once();
}

struct DigitCase {
  const char* text;
  int accepted;
  __u8 value;
};

static void fn_lock_digit(const struct DigitCase* digit) {
  test_reset();
  set_property(UDEV_PROP_ASUS_UX8406_KBD_FN_LOCK, digit->text);
  CHECK_EQ(run_probe(&kUsb), 0);
  CHECK_EQ(kbd_view()->fn_lock, digit->accepted ? digit->value : 1);
  check_request(0, fn_report(kbd_view()->fn_lock));
  CHECK_EQ(fake_kernel()->hw_count, 1);
}

static void backlight_digit(const struct DigitCase* digit) {
  test_reset();
  set_property(UDEV_PROP_ASUS_UX8406_KBD_BACKLIGHT, digit->text);
  CHECK_EQ(run_probe(&kUsb), 0);
  CHECK_EQ(kbd_view()->backlight, digit->accepted ? digit->value : 0);
  CHECK_EQ(fake_kernel()->hw_count, digit->accepted ? 2 : 1);
  if (digit->accepted) {
    check_request(1, backlight_report(digit->value));
  }
}

/* "/" is the character before "0", ":" the one after "9". */
static void fn_lock_property_edges() {
  const struct DigitCase cases[] = {
      {"0", 1, 0},  {"1", 1, 1},  {"2", 0, 0},  {"9", 0, 0},
      {"/", 0, 0},  {":", 0, 0},  {"a", 0, 0},  {"", 0, 0},
      {"01", 0, 0}, {"10", 0, 0}, {"1x", 0, 0}, {" 1", 0, 0},
  };

  for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
    fn_lock_digit(&cases[i]);
  }
}

static void backlight_property_edges() {
  const struct DigitCase cases[] = {
      {"0", 1, 0},  {"1", 1, 1},  {"3", 1, 3},  {"4", 0, 0},
      {"9", 0, 0},  {"/", 0, 0},  {"a", 0, 0},  {"", 0, 0},
      {"30", 0, 0}, {"03", 0, 0}, {"3x", 0, 0},
  };

  for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
    backlight_digit(&cases[i]);
  }
}

/* A property of four characters fills the array, with no terminator. */
static void property_with_four_digits() {
  set_property(UDEV_PROP_ASUS_UX8406_KBD_BACKLIGHT, "2222");
  CHECK_EQ(run_probe(&kUsb), 0);
  CHECK_EQ(kbd_view()->backlight, 0);
  CHECK_EQ(fake_kernel()->hw_count, 1);
}

static const struct TestCase kCases[] = {
    TEST_CASE(no_hotkeys),
    TEST_CASE(signature_with_one_byte_wrong),
    TEST_CASE(real_captures_are_taken),
    TEST_CASE(first_init_fails),
    TEST_CASE(second_init_fails),
    TEST_CASE(no_context),
    TEST_CASE(no_state),
    TEST_CASE(success_without_properties),
    TEST_CASE(success_with_properties),
    TEST_CASE(refused_requests_do_not_fail_the_probe),
    TEST_CASE(fn_lock_property_edges),
    TEST_CASE(backlight_property_edges),
    TEST_CASE(property_with_four_digits),
};

const struct TestGroup kProbeTests = {
    kCases,
    sizeof(kCases) / sizeof(kCases[0]),
};
