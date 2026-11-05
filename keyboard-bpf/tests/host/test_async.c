// The deferred commands (set_backlight, set_fn_lock), run through the fake
// work queue.
#include <stddef.h>

#include "fake_kernel.h"
#include "test_support.h"

enum {
  kReportId = 0x5a,
  kCodeFnEsc = 0x4e,
  kCodeBacklight = 0xc7,
  kReportBytes = 6,
  kHid = 7,
  kFeatureBytes = 16,
  kFeatureReport = 2,
  kSetReport = 9,
};

/* Queue the command that a key press queues. */
static void press(__u8 code) {
  struct hid_bpf_ctx hctx = {
      .hid = kHid,
      .allocated_size = kReportBytes,
      .size = kReportBytes,
  };
  const __u8 report[kReportBytes] = {kReportId, code};

  for (size_t i = 0; i < sizeof(report); i++) {
    fake_kernel()->data[i] = report[i];
  }
  CHECK_EQ(zenbook_duo_kbd_fix_event(&hctx), 0);
}

static void check_request(int index, const __u8* want) {
  const struct FakeHwCall* call = &fake_kernel()->hw[index];

  CHECK_EQ(call->size, kFeatureBytes);
  CHECK_EQ(call->type, kFeatureReport);
  CHECK_EQ(call->reqtype, kSetReport);
  CHECK_BYTES(call->report, want, kFeatureBytes);
  CHECK(call->ctx != nullptr);
}

static void backlight_sends_the_level() {
  const __u8 want[kFeatureBytes] = {kReportId, 0xba, 0xc5, 0xc4, 3};

  press(kCodeBacklight);
  CHECK_EQ(fake_run_delayed(), 1);
  CHECK_EQ(fake_kernel()->queue_results[0], 0);
  CHECK_EQ(fake_kernel()->hw_count, 1);
  check_request(0, want);
  CHECK_EQ(fake_kernel()->allocs, 1);
  CHECK_EQ(fake_kernel()->releases, 1);
  CHECK_EQ(fake_contexts_in_use(), 0);
  CHECK_EQ(fake_kernel()->alloc_hid[0], kHid);
}

static void backlight_uses_the_level_at_run_time() {
  const __u8 want[kFeatureBytes] = {kReportId, 0xba, 0xc5, 0xc4, 2};

  press(kCodeBacklight);
  kbd_view()->backlight = 2;
  CHECK_EQ(fake_run_delayed(), 1);
  check_request(0, want);
}

static void backlight_without_state() {
  press(kCodeBacklight);
  fake_kernel()->map_fail = 1;
  CHECK_EQ(fake_run_delayed(), 1);
  CHECK_EQ(fake_kernel()->queue_results[0], -EINVAL);
  CHECK_EQ(fake_kernel()->hw_count, 0);
  CHECK_EQ(fake_contexts_in_use(), 0);
}

static void backlight_request_fails() {
  press(kCodeBacklight);
  fake_kernel()->hw_results[0] = -EIO;
  CHECK_EQ(fake_run_delayed(), 1);
  CHECK_EQ(fake_kernel()->queue_results[0], -EIO);
}

static void backlight_request_returns_a_length() {
  press(kCodeBacklight);
  fake_kernel()->hw_results[0] = kFeatureBytes;
  CHECK_EQ(fake_run_delayed(), 1);
  CHECK_EQ(fake_kernel()->queue_results[0], 0);
}

/* -1 is the smallest error: a limit of "below -1" would let it through. */
static void smallest_error_is_returned() {
  fake_kernel()->hw_results[0] = -1;
  fake_kernel()->hw_results[1] = -1;
  press(kCodeBacklight);
  press(kCodeFnEsc);
  CHECK_EQ(fake_run_delayed(), 2);
  CHECK_EQ(fake_kernel()->queue_results[0], -1);
  CHECK_EQ(fake_kernel()->queue_results[1], -1);
}

static void fn_lock_sends_the_row_mode() {
  const __u8 want[kFeatureBytes] = {kReportId, 0xd0, kCodeFnEsc, 1};

  press(kCodeFnEsc);
  CHECK_EQ(fake_run_delayed(), 1);
  CHECK_EQ(fake_kernel()->queue_results[0], 0);
  CHECK_EQ(fake_kernel()->hw_count, 1);
  check_request(0, want);
  CHECK_EQ(fake_contexts_in_use(), 0);
}

static void fn_lock_uses_the_mode_at_run_time() {
  const __u8 want[kFeatureBytes] = {kReportId, 0xd0, kCodeFnEsc, 0};

  press(kCodeFnEsc);
  kbd_view()->fn_lock = 0;
  CHECK_EQ(fake_run_delayed(), 1);
  check_request(0, want);
}

static void fn_lock_without_state() {
  press(kCodeFnEsc);
  fake_kernel()->map_fail = 1;
  CHECK_EQ(fake_run_delayed(), 1);
  CHECK_EQ(fake_kernel()->queue_results[0], -EINVAL);
  CHECK_EQ(fake_kernel()->hw_count, 0);
}

static void fn_lock_request_fails() {
  press(kCodeFnEsc);
  fake_kernel()->hw_results[0] = -EBUSY;
  CHECK_EQ(fake_run_delayed(), 1);
  CHECK_EQ(fake_kernel()->queue_results[0], -EBUSY);
}

static void fn_lock_request_returns_a_length() {
  press(kCodeFnEsc);
  fake_kernel()->hw_results[0] = 1;
  CHECK_EQ(fake_run_delayed(), 1);
  CHECK_EQ(fake_kernel()->queue_results[0], 0);
}

static void the_two_commands_keep_their_own_buffers() {
  press(kCodeBacklight);
  press(kCodeFnEsc);
  CHECK_EQ(fake_run_delayed(), 2);
  CHECK_EQ(fake_kernel()->hw[0].report[1], 0xba);
  CHECK_EQ(fake_kernel()->hw[1].report[1], 0xd0);
  CHECK_EQ(fake_kernel()->allocs, fake_kernel()->releases);
}

static const struct TestCase kCases[] = {
    TEST_CASE(backlight_sends_the_level),
    TEST_CASE(backlight_uses_the_level_at_run_time),
    TEST_CASE(backlight_without_state),
    TEST_CASE(backlight_request_fails),
    TEST_CASE(backlight_request_returns_a_length),
    TEST_CASE(smallest_error_is_returned),
    TEST_CASE(fn_lock_sends_the_row_mode),
    TEST_CASE(fn_lock_uses_the_mode_at_run_time),
    TEST_CASE(fn_lock_without_state),
    TEST_CASE(fn_lock_request_fails),
    TEST_CASE(fn_lock_request_returns_a_length),
    TEST_CASE(the_two_commands_keep_their_own_buffers),
};

const struct TestGroup kAsyncTests = {
    kCases,
    sizeof(kCases) / sizeof(kCases[0]),
};
