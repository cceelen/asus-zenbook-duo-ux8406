// zenbook_duo_kbd_fix_event: the hotkey report, one branch after the other.
// The constants are the keyboard's, written out here on purpose: the tests
// hold the program to them.
#include <string.h>

#include "fake_kernel.h"
#include "test_support.h"

enum {
  kReportId = 0x5a,
  kReportSize = 6,
  kCodeStatus = 0x3d,
  kCodeFnEsc = 0x4e,
  kCodeBacklight = 0xc7,
  kPadding = 0x11, /* a value that must not survive in bytes 2 to 4 */
  kDelayMs = 10,
};

struct Hotkey {
  __u8 code;
  __u8 index;
};

static const struct Hotkey kHotkeys[] = {
    {0x10, 1}, {0x20, 2}, {0x7c, 3}, {0x7e, 4}, {0x86, 5}, {0x9c, 6}, {0x6a, 7},
};

/* The context of an event: a report of six bytes. */
static struct hid_bpf_ctx event_ctx() {
  return (struct hid_bpf_ctx){
      .hid = 1,
      .allocated_size = kReportSize,
      .size = kReportSize,
  };
}

static void put_report(__u8 report_id, __u8 code) {
  const __u8 report[kReportSize] = {
      report_id, code, kPadding, kPadding, kPadding, kPadding,
  };

  memcpy(fake_kernel()->data, report, sizeof(report));
}

static int fix_event(__u8 report_id, __u8 code) {
  struct hid_bpf_ctx hctx = event_ctx();

  put_report(report_id, code);
  return zenbook_duo_kbd_fix_event(&hctx);
}

static void check_untouched(__u8 report_id, __u8 code) {
  const __u8 report[kReportSize] = {
      report_id, code, kPadding, kPadding, kPadding, kPadding,
  };

  CHECK_BYTES(fake_kernel()->data, report, sizeof(report));
}

static void no_data() {
  struct hid_bpf_ctx hctx = event_ctx();

  fake_kernel()->data_fail = 1;
  put_report(kReportId, kCodeBacklight);
  CHECK_EQ(zenbook_duo_kbd_fix_event(&hctx), 0);
  check_untouched(kReportId, kCodeBacklight);
  CHECK_EQ(fake_kernel()->queue_count, 0);
  CHECK_EQ(kbd_view()->backlight, 0);
}

static void report_window_too_small() {
  struct hid_bpf_ctx hctx = event_ctx();

  hctx.allocated_size = kReportSize - 1;
  put_report(kReportId, kCodeBacklight);
  CHECK_EQ(zenbook_duo_kbd_fix_event(&hctx), 0);
  check_untouched(kReportId, kCodeBacklight);
  CHECK_EQ(fake_kernel()->queue_count, 0);
}

static void no_state() {
  fake_kernel()->map_fail = 1;
  CHECK_EQ(fix_event(kReportId, kCodeBacklight), 0);
  check_untouched(kReportId, kCodeBacklight);
  CHECK_EQ(fake_kernel()->queue_count, 0);
  CHECK_EQ(fake_kernel()->map_wrong_calls, 0);
}

static void other_report_id() {
  CHECK_EQ(fix_event(kReportId - 1, kCodeBacklight), 0);
  check_untouched(kReportId - 1, kCodeBacklight);
  CHECK_EQ(fix_event(kReportId + 1, kCodeFnEsc), 0);
  check_untouched(kReportId + 1, kCodeFnEsc);
  CHECK_EQ(fake_kernel()->queue_count, 0);
  CHECK_EQ(kbd_view()->backlight, 0);
  CHECK_EQ(kbd_view()->fn_lock, 0);
}

static void status_report_is_dropped() {
  CHECK_EQ(fix_event(kReportId, kCodeStatus), -1);
  check_untouched(kReportId, kCodeStatus);
  CHECK_EQ(fake_kernel()->queue_count, 0);
}

static void check_rewritten(__u8 index, __u8 raw) {
  const __u8 want[kReportSize] = {kReportId, index, 0, 0, 0, raw};

  CHECK_BYTES(fake_kernel()->data, want, sizeof(want));
}

static void backlight_toggles() {
  const __u8 levels[] = {3, 0, 3};

  for (size_t i = 0; i < sizeof(levels); i++) {
    CHECK_EQ(fix_event(kReportId, kCodeBacklight), 0);
    check_rewritten(0, kCodeBacklight);
    CHECK_EQ(kbd_view()->backlight, levels[i]);
    CHECK_EQ(fake_kernel()->queue_count, 1);
    CHECK_EQ(fake_kernel()->queue[0].milliseconds, kDelayMs);
    CHECK_EQ(fake_run_delayed(), 1);
    CHECK_EQ(fake_kernel()->hw_count, (int)i + 1);
    CHECK_EQ(fake_kernel()->hw[i].report[4], levels[i]);
  }
  CHECK_EQ(kbd_view()->fn_lock, 0);
}

static void fn_esc_toggles() {
  const __u8 rows[] = {1, 0, 1};

  for (size_t i = 0; i < sizeof(rows); i++) {
    CHECK_EQ(fix_event(kReportId, kCodeFnEsc), 0);
    check_rewritten(0, kCodeFnEsc);
    CHECK_EQ(kbd_view()->fn_lock, rows[i]);
    CHECK_EQ(fake_kernel()->queue_count, 1);
    CHECK_EQ(fake_kernel()->queue[0].milliseconds, kDelayMs);
    CHECK_EQ(fake_run_delayed(), 1);
    CHECK_EQ(fake_kernel()->hw[i].report[3], rows[i]);
  }
  CHECK_EQ(kbd_view()->backlight, 0);
}

static void each_hotkey() {
  for (size_t i = 0; i < sizeof(kHotkeys) / sizeof(kHotkeys[0]); i++) {
    CHECK_EQ(fix_event(kReportId, kHotkeys[i].code), 0);
    check_rewritten(kHotkeys[i].index, kHotkeys[i].code);
  }
  CHECK_EQ(fake_kernel()->queue_count, 0);
  CHECK_EQ(kbd_view()->backlight, 0);
  CHECK_EQ(kbd_view()->fn_lock, 0);
}

static void unknown_codes_are_no_key() {
  const __u8 codes[] = {0x00, 0x0f, 0x11, 0x3e, 0x4f, 0xc6, 0xc8, 0xff};

  for (size_t i = 0; i < sizeof(codes); i++) {
    CHECK_EQ(fix_event(kReportId, codes[i]), 0);
    check_rewritten(0, codes[i]);
  }
  CHECK_EQ(fake_kernel()->queue_count, 0);
}

static const struct TestCase kCases[] = {
    TEST_CASE(no_data),
    TEST_CASE(report_window_too_small),
    TEST_CASE(no_state),
    TEST_CASE(other_report_id),
    TEST_CASE(status_report_is_dropped),
    TEST_CASE(backlight_toggles),
    TEST_CASE(fn_esc_toggles),
    TEST_CASE(each_hotkey),
    TEST_CASE(unknown_codes_are_no_key),
};

const struct TestGroup kEventTests = {
    kCases,
    sizeof(kCases) / sizeof(kCases[0]),
};
