/*
 * Detachable keyboard of the ASUS Zenbook Duo UX8406CA: hotkeys and keyboard
 * backlight.
 *
 * The keyboard is 0b05:1bf2 on USB (docked on the lower panel, or on its
 * cable) and 0b05:1bf3 over Bluetooth. Volume keys and the display switch
 * come as standard reports. The other keys of the function row come in a
 * vendor report (ID 0x5a) whose five bytes are all declared as one vendor
 * usage, so that the key code arrives as a value and no key is ever seen:
 *
 *   5a 10  display brightness down     5a 7e  emoji
 *   5a 20  display brightness up       5a 86  MyASUS
 *   5a c7  keyboard backlight          5a 9c  swap windows between panels
 *   5a 7c  microphone mute             5a 6a  lower panel on/off
 *   5a 4e  Fn+Esc
 *
 * This program
 *  - declares the first byte of that report as an array over standard
 *    usages, and its collection as a Consumer Control, which on USB is what
 *    makes the interface an input device at all;
 *  - replaces the vendor code by the index of the matching usage;
 *  - switches the backlight on and off itself when its key is pressed, as
 *    the keyboard does nothing on that key and there is no LED device to
 *    hand it to;
 *  - switches the function row between hotkeys and F1 to F12 on Fn+Esc. The
 *    keyboard does that by itself only until the host has sent it a command
 *    (such as the one for the backlight); from then on it merely reports the
 *    key;
 *  - starts every connection with the function row and the backlight that
 *    two udev properties of the HID device give, and without them with F1
 *    to F12; Fn+Esc activates the hotkeys. Docking and undocking moves the
 *    keyboard between USB and Bluetooth, which are two HID devices with one
 *    instance of this program each, and the keyboard comes up without its
 *    backlight and not in the same row state on both. The instances cannot
 *    share the state themselves: udev-hid-bpf pins every map below the
 *    device's own directory and refuses a map pinned by name;
 *  - drops a report the keyboard sends unasked (5a 3d ...), whose further
 *    bytes would otherwise be taken for keys.
 */

// The order matters: the BPF headers need the types of vmlinux.h first, and
// wq_compat.h comes after hid_bpf_helpers.h and before hid_bpf_async.h.
// clang-format off
#include "hid_bpf.h"
#include "hid_bpf_helpers.h"
#include "vmlinux.h"
#include <linux/bpf.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_helpers.h>
// Needed by hid_bpf_async.h, which reaches it through a macro that
// include-cleaner does not follow.
#include "wq_compat.h"  // NOLINT(misc-include-cleaner)
#define HID_BPF_ASYNC_MAX_CTX 2 /* deferred calls: backlight, Fn lock */
#include "hid_bpf_async.h"
// clang-format on

enum {
  kVidAsustek = 0x0B05,
  kPidKeyboardUsb = 0x1BF2, /* UX8406CA */
  kPidKeyboardBt = 0x1BF3,  /* UX8406CA */
  /* UX8406MA; attaches only if its report descriptor is the same */
  kPidKeyboardUsbMa = 0x1B2C,
};

// The loader looks for this section, the union is global and writable.
// NOLINTNEXTLINE(misc-use-internal-linkage,cppcoreguidelines-avoid-non-const-global-variables)
HID_BPF_CONFIG(
    HID_DEVICE(BUS_USB, HID_GROUP_GENERIC, kVidAsustek, kPidKeyboardUsb),
    HID_DEVICE(BUS_BLUETOOTH, HID_GROUP_GENERIC, kVidAsustek, kPidKeyboardBt),
    HID_DEVICE(BUS_USB, HID_GROUP_GENERIC, kVidAsustek, kPidKeyboardUsbMa), );

enum {
  kHotkeyReportId = 0x5a,
  kHotkeyReportSize = 6,   /* report ID and five bytes */
  kRawCodeIndex = 5,       /* the last of them */
  kFeatureReportSize = 16, /* report ID and fifteen bytes */
};

/*
 * On USB the hotkey collection is the second and last one of interface 4;
 * over Bluetooth, where all interfaces are one descriptor, the same vendor
 * collection follows it instead of preceding it.
 */
enum {
  kUsbRdescSize = 90,
  kUsbHotkeyOffset = 57,
  kBtRdescSize = 257,
  kBtHotkeyOffset = 167,
  kHotkeyCollectionSize = 33,
};

/* Vendor codes of the hotkey report. */
enum {
  kCodeNone = 0x00,
  kCodeBrightnessDown = 0x10,
  kCodeBrightnessUp = 0x20,
  kCodeStatus = 0x3d, /* not a key, see above */
  kCodeFnEsc = 0x4e,
  kCodeLowerPanel = 0x6a,
  kCodeMicMute = 0x7c,
  kCodeEmoji = 0x7e,
  kCodeMyAsus = 0x86,
  kCodeSwapWindows = 0x9c,
  kCodeBacklight = 0xc7,
};

/* The first byte of the report after the fixup: an index into the usages
 * listed in kFixedHotkeys, 0 for no key.
 */
enum hotkey {
  kHotkeyNone = 0,
  kHotkeyBrightnessDown = 1,
  kHotkeyBrightnessUp = 2,
  kHotkeyMicMute = 3,
  kHotkeyEmoji = 4,
  kHotkeyMyAsus = 5,
  kHotkeySwapWindows = 6,
  kHotkeyLowerPanel = 7,
  kHotkeyLast = kHotkeyLowerPanel,
};

/* The first bytes of the hotkey collection as the keyboard declares it. */
static const __u8 kHotkeySignature[] = {
    0x06, 0x31,
    0xff,                  /* Usage Page (Vendor 0xff31)		*/
    0x09, 0x76,            /* Usage (0x76)				*/
    0xa1, 0x01,            /* Collection (Application)		*/
    0x85, kHotkeyReportId, /*   Report ID (0x5a)			*/
};

/* The hotkey collection as it is given to the kernel. */
static const __u8 kFixedHotkeys[] = {
    0x05,
    0x0c, /* Usage Page (Consumer)		*/
    0x09,
    0x01, /* Usage (Consumer Control)		*/
    0xa1,
    0x01, /* Collection (Application)		*/
    0x85,
    kHotkeyReportId, /*   Report ID (0x5a)			*/
    0x15,
    0x01, /*   Logical Minimum (1)		*/
    0x25,
    kHotkeyLast, /*   Logical Maximum (7)		*/
    0x75,
    0x08, /*   Report Size (8)			*/
    0x95,
    0x01, /*   Report Count (1)			*/
    /*   Usages, in the order of enum hotkey:			*/
    0x0b,
    0x70,
    0x00,
    0x0c,
    0x00, /* Consumer: Brightness Decrement	*/
    0x0b,
    0x6f,
    0x00,
    0x0c,
    0x00, /* Consumer: Brightness Increment	*/
    0x0b,
    0xa9,
    0x00,
    0x01,
    0x00, /* Desktop: System Microphone Mute	*/
    0x0b,
    0x86,
    0x00,
    0x01,
    0x00, /* Desktop: System App Menu (PROG1)	*/
    0x0b,
    0x68,
    0x00,
    0x07,
    0x00, /* Keyboard: F13 (desktops: settings)	*/
    0x0b,
    0x6e,
    0x00,
    0x07,
    0x00, /* Keyboard: F19			*/
    0x0b,
    0x6d,
    0x00,
    0x07,
    0x00, /* Keyboard: F18			*/
    0x81,
    0x00, /*   Input (Data,Array,Abs)		*/
    0x95,
    0x04, /*   Report Count (4)			*/
    0x81,
    0x03, /*   Input (Const,Var,Abs)		*/
    /*   The feature report, unchanged:				*/
    0x06,
    0x31,
    0xff, /*   Usage Page (Vendor 0xff31)		*/
    0x09,
    0x76, /*   Usage (0x76)			*/
    0x15,
    0x00, /*   Logical Minimum (0)		*/
    0x26,
    0xff,
    0x00, /*   Logical Maximum (255)		*/
    0x95,
    0x0f, /*   Report Count (15)			*/
    0xb1,
    0x02, /*   Feature (Data,Var,Abs)		*/
    0xc0, /* End Collection			*/
};

/* The collection that follows the hotkeys over Bluetooth, unchanged. */
static const __u8 kBtVendorCollection[] = {
    0x06, 0x00, 0xff, /* Usage Page (Vendor 0xff00)		*/
    0x09, 0x01,       /* Usage (0x01)				*/
    0xa1, 0x01,       /* Collection (Application)		*/
    0x85, 0x42,       /*   Report ID (0x42)			*/
    0x09, 0x06,       /*   Usage (0x06)			*/
    0x15, 0x00,       /*   Logical Minimum (0)		*/
    0x26, 0xff, 0x00, /*   Logical Maximum (255)		*/
    0x75, 0x08,       /*   Report Size (8)			*/
    0x95, 0x03,       /*   Report Count (3)			*/
    0xb1, 0x02,       /*   Feature (Data,Var,Abs)		*/
    0x85, 0x43,       /*   Report ID (0x43)			*/
    0x09, 0x06,       /*   Usage (0x06)			*/
    0x15, 0x00,       /*   Logical Minimum (0)		*/
    0x26, 0xff, 0x00, /*   Logical Maximum (255)		*/
    0x75, 0x08,       /*   Report Size (8)			*/
    0x95, 0x03,       /*   Report Count (3)			*/
    0xb1, 0x02,       /*   Feature (Data,Var,Abs)		*/
    0x06, 0x00, 0xff, /*   Usage Page (Vendor 0xff00)		*/
    0x85, 0x41,       /*   Report ID (0x41)			*/
    0x09, 0x05,       /*   Usage (0x05)			*/
    0x15, 0x00,       /*   Logical Minimum (0)		*/
    0x26, 0xff, 0x00, /*   Logical Maximum (255)		*/
    0x75, 0x08,       /*   Report Size (8)			*/
    0x96, 0x00, 0x01, /*   Report Count (256)			*/
    0xb1, 0x02,       /*   Feature (Data,Var,Abs)		*/
    0xc0,             /* End Collection			*/
};

_Static_assert(kBtRdescSize == kBtHotkeyOffset + kHotkeyCollectionSize +
                                   sizeof(kBtVendorCollection),
               "Bluetooth descriptor layout");
_Static_assert(kUsbRdescSize == kUsbHotkeyOffset + kHotkeyCollectionSize,
               "USB descriptor layout");

/* What the keyboard was last asked for on this connection. */
struct KbdState {
  __u8 fn_lock;
  __u8 backlight;
};

// The loader finds the map by its section and name; it is global and writable.
// NOLINTBEGIN(misc-use-internal-linkage,cppcoreguidelines-avoid-non-const-global-variables)
struct {
  __uint(type, BPF_MAP_TYPE_ARRAY);
  __uint(max_entries, 1);
  __type(key, __u32);
  __type(value, struct KbdState);
} zenbook_duo_kbd_state SEC(".maps");
// NOLINTEND(misc-use-internal-linkage,cppcoreguidelines-avoid-non-const-global-variables)

/*
 * What the keyboard was last asked for on its other connection, as udev
 * properties of the HID device: udev-hid-bpf fills a variable of this name
 * with the property when it loads the program, and leaves it empty when
 * there is none. One digit each: the backlight level (0 to 3) and the
 * function row (0 hotkeys, 1 F1 to F12). asus-ux8406-keyboard-state sets
 * the properties from the map above (keyboard-state/ in this tree).
 */
// NOLINTBEGIN(misc-use-internal-linkage,cppcoreguidelines-avoid-non-const-global-variables,readability-identifier-naming)
char UDEV_PROP_ASUS_UX8406_KBD_BACKLIGHT[4];
char UDEV_PROP_ASUS_UX8406_KBD_FN_LOCK[4];
// NOLINTEND(misc-use-internal-linkage,cppcoreguidelines-avoid-non-const-global-variables,readability-identifier-naming)

/*
 * Put the digit of a property into `value`, if the property is one digit up
 * to `max`. Says whether it was; `value` is left alone otherwise.
 */
static __always_inline bool property_digit(const char* property, __u8 max,
                                           __u8* value) {
  const char digit = property[0];

  if (digit < '0' || digit > '0' + max || property[1] != '\0') {
    return false;
  }
  *value = digit - '0';
  return true;
}

static __always_inline struct KbdState* kbd_state(void) {
  __u32 key = 0;

  return bpf_map_lookup_elem(&zenbook_duo_kbd_state, &key);
}

enum {
  kBacklightLevelIndex = 4,
  kBacklightOff = 0,
  kBacklightOn = 3, /* the brightest of the levels 1 to 3 */
  /* The command and its two parameters, as the keyboard takes them. */
  kBacklightCommand = 0xba,
  kBacklightParameter1 = 0xc5,
  kBacklightParameter2 = 0xc4,
};

/* The feature report that sets the backlight; one buffer, kept between calls.
 */
static __always_inline __u8* backlight_report(void) {
  static __u8 report[kFeatureReportSize] = {
      kHotkeyReportId,      kBacklightCommand, kBacklightParameter1,
      kBacklightParameter2, kBacklightOff,
  };

  return report;
}

// The macro fixes the callback signature and generates a global key variable.
// NOLINTNEXTLINE(misc-unused-parameters,misc-use-internal-linkage,cppcoreguidelines-avoid-non-const-global-variables,readability-identifier-length)
static int HID_BPF_ASYNC_FUN(set_backlight)(struct hid_bpf_ctx* hctx) {
  struct KbdState* state = kbd_state();
  __u8* report = backlight_report();
  int err = 0;

  if (!state) {
    return -EINVAL;
  }

  report[kBacklightLevelIndex] = state->backlight;
  err = hid_bpf_hw_request(hctx, report, kFeatureReportSize, HID_FEATURE_REPORT,
                           HID_REQ_SET_REPORT);

  return err < 0 ? err : 0;
}

enum {
  kFnLockStateIndex = 3,
  kFnLockHotkeys = 0,      /* the row sends the hotkeys */
  kFnLockFunctionKeys = 1, /* the row sends F1 to F12 */
  kFnLockCommand = 0xd0,
};

/* The feature report that sets the function row; one buffer, kept between
 * calls.
 */
static __always_inline __u8* fn_lock_report(void) {
  static __u8 report[kFeatureReportSize] = {
      kHotkeyReportId,
      kFnLockCommand,
      kCodeFnEsc,
      kFnLockHotkeys,
  };

  return report;
}

// The macro fixes the callback signature and generates a global key variable.
// NOLINTNEXTLINE(misc-unused-parameters,misc-use-internal-linkage,cppcoreguidelines-avoid-non-const-global-variables,readability-identifier-length)
static int HID_BPF_ASYNC_FUN(set_fn_lock)(struct hid_bpf_ctx* hctx) {
  struct KbdState* state = kbd_state();
  __u8* report = fn_lock_report();
  int err = 0;

  if (!state) {
    return -EINVAL;
  }

  report[kFnLockStateIndex] = state->fn_lock;
  err = hid_bpf_hw_request(hctx, report, kFeatureReportSize, HID_FEATURE_REPORT,
                           HID_REQ_SET_REPORT);

  return err < 0 ? err : 0;
}

static __always_inline enum hotkey hotkey_of(__u8 code) {
  switch (code) {
    case kCodeBrightnessDown:
      return kHotkeyBrightnessDown;
    case kCodeBrightnessUp:
      return kHotkeyBrightnessUp;
    case kCodeMicMute:
      return kHotkeyMicMute;
    case kCodeEmoji:
      return kHotkeyEmoji;
    case kCodeMyAsus:
      return kHotkeyMyAsus;
    case kCodeSwapWindows:
      return kHotkeySwapWindows;
    case kCodeLowerPanel:
      return kHotkeyLowerPanel;
    default:
      return kHotkeyNone;
  }
}

SEC(HID_BPF_DEVICE_EVENT)
// The loader looks the program up by its name, so it is global.
// NOLINTNEXTLINE(misc-use-internal-linkage)
int BPF_PROG(zenbook_duo_kbd_fix_event, struct hid_bpf_ctx* hctx) {
  __u8* data = hid_bpf_get_data(hctx, 0 /* offset */, kHotkeyReportSize);
  struct KbdState* state = kbd_state();

  if (!data || !state) {
    return 0; /* EPERM check */
  }

  if (data[0] != kHotkeyReportId) {
    return 0;
  }

  if (data[1] == kCodeStatus) {
    return -1;
  }

  if (data[1] == kCodeBacklight) {
    state->backlight = state->backlight ? kBacklightOff : kBacklightOn;
    HID_BPF_ASYNC_DELAYED_CALL(set_backlight, hctx, 10);
  }

  if (data[1] == kCodeFnEsc) {
    state->fn_lock = state->fn_lock ? kFnLockHotkeys : kFnLockFunctionKeys;
    HID_BPF_ASYNC_DELAYED_CALL(set_fn_lock, hctx, 10);
  }

  /*
   * The last byte is padding for the kernel; the vendor code is kept
   * there for whoever reads the raw reports.
   */
  data[kRawCodeIndex] = data[1];
  data[1] = hotkey_of(data[1]);
  data[2] = 0;
  data[3] = 0;
  data[4] = 0;

  return 0;
}

SEC(HID_BPF_RDESC_FIXUP)
// The loader looks the program up by its name, so it is global.
// NOLINTNEXTLINE(misc-use-internal-linkage)
int BPF_PROG(zenbook_duo_kbd_fix_rdesc, struct hid_bpf_ctx* hctx) {
  __u8* data = hid_bpf_get_data(hctx, 0 /* offset */,
                                HID_MAX_DESCRIPTOR_SIZE /* size */);

  if (!data) {
    return 0; /* EPERM check */
  }

  if (hctx->size == kUsbRdescSize) {
    __builtin_memcpy(data + kUsbHotkeyOffset, kFixedHotkeys,
                     sizeof(kFixedHotkeys));
    return kUsbHotkeyOffset + sizeof(kFixedHotkeys);
  }

  if (hctx->size == kBtRdescSize) {
    __builtin_memcpy(data + kBtHotkeyOffset, kFixedHotkeys,
                     sizeof(kFixedHotkeys));
    __builtin_memcpy(data + kBtHotkeyOffset + sizeof(kFixedHotkeys),
                     kBtVendorCollection, sizeof(kBtVendorCollection));
    return kBtHotkeyOffset + sizeof(kFixedHotkeys) +
           sizeof(kBtVendorCollection);
  }

  return 0;
}

// The loader finds the struct_ops by its section and name; it is global and
// writable.
// NOLINTNEXTLINE(misc-use-internal-linkage,cppcoreguidelines-avoid-non-const-global-variables)
HID_BPF_OPS(zenbook_duo_keyboard) = {
    .hid_device_event = (void*)zenbook_duo_kbd_fix_event,
    .hid_rdesc_fixup = (void*)zenbook_duo_kbd_fix_rdesc,
};

/* Whether the hotkey collection starts at this offset of the descriptor. */
static __always_inline bool has_hotkeys_at(const struct hid_bpf_probe_args* ctx,
                                           unsigned int offset) {
  return __builtin_memcmp(ctx->rdesc + offset, kHotkeySignature,
                          sizeof(kHotkeySignature)) == 0;
}

/* Whether this is the interface that has the hotkeys. On USB the IDs match
 * five interfaces; only one has them.
 */
static __always_inline bool has_hotkeys(const struct hid_bpf_probe_args* ctx) {
  if (ctx->rdesc_size == kUsbRdescSize &&
      has_hotkeys_at(ctx, kUsbHotkeyOffset)) {
    return true;
  }

  if (ctx->rdesc_size == kBtRdescSize && has_hotkeys_at(ctx, kBtHotkeyOffset)) {
    return true;
  }

  return false;
}

/*
 * No "ASUS Tech.Inc." handshake here, as hid-asus sends it to other
 * keyboards: after it this keyboard only reports Fn+Esc and no longer
 * switches the function row by itself.
 *
 * The keyboard comes up with the function row in a state that cannot be
 * read, and not the same one on each connection. Starting with the
 * remembered state, or with F1 to F12 if there is none, makes Fn+Esc switch
 * from a known state.
 *
 * The command is sent from here and waited for, which holds the connection
 * up for as long as the keyboard takes to answer (about half a second over
 * Bluetooth). It cannot be handed to the work queue from this function: the
 * program then loads and is dropped again at once.
 */
static __always_inline void start(struct hid_bpf_ctx* hctx,
                                  struct KbdState* state) {
  /* A keyboard that refuses this still has its keys. */
  state->fn_lock = kFnLockFunctionKeys;
  property_digit(UDEV_PROP_ASUS_UX8406_KBD_FN_LOCK, kFnLockFunctionKeys,
                 &state->fn_lock);
  fn_lock_report()[kFnLockStateIndex] = state->fn_lock;
  hid_bpf_hw_request(hctx, fn_lock_report(), kFeatureReportSize,
                     HID_FEATURE_REPORT, HID_REQ_SET_REPORT);

  /*
   * The keyboard comes up without its backlight after a change between the
   * dock and Bluetooth. With a remembered level the backlight is set to it;
   * without one it is left as the keyboard has it.
   */
  state->backlight = kBacklightOff;
  if (property_digit(UDEV_PROP_ASUS_UX8406_KBD_BACKLIGHT, kBacklightOn,
                     &state->backlight)) {
    backlight_report()[kBacklightLevelIndex] = state->backlight;
    hid_bpf_hw_request(hctx, backlight_report(), kFeatureReportSize,
                       HID_FEATURE_REPORT, HID_REQ_SET_REPORT);
  }
}

SEC("syscall")
// The loader looks the program up by its name, so it is global.
// NOLINTNEXTLINE(misc-use-internal-linkage)
int probe(struct hid_bpf_probe_args* ctx) {
  struct KbdState* state = kbd_state();
  struct hid_bpf_ctx* hctx = NULL;

  if (!has_hotkeys(ctx)) {
    ctx->retval = -EINVAL;
    return 0;
  }

  ctx->retval =
      HID_BPF_ASYNC_INIT(set_backlight) || HID_BPF_ASYNC_INIT(set_fn_lock);
  if (ctx->retval) {
    return 0;
  }

  hctx = hid_bpf_allocate_context(ctx->hid);
  if (!hctx || !state) {
    if (hctx) {
      hid_bpf_release_context(hctx);
    }
    ctx->retval = -EINVAL;
    return 0;
  }

  start(hctx, state);
  hid_bpf_release_context(hctx);

  return 0;
}

// The loader looks for this name in this section; the string is writable.
// NOLINTNEXTLINE(bugprone-reserved-identifier,cert-dcl37-c,cert-dcl51-cpp,readability-identifier-naming,misc-use-internal-linkage,cppcoreguidelines-avoid-non-const-global-variables)
char _license[] SEC("license") = "GPL";
