// Checks, the test table, and what the tests use of the program under test.
#ifndef UX8406_TESTS_TEST_SUPPORT_H_
#define UX8406_TESTS_TEST_SUPPORT_H_

#include <stdbool.h>
#include <stddef.h>

#include "fake_kernel.h"

#ifndef UX8406_HOST_SHIM
#error \
    "the program must be compiled against the host fakes, not the BPF headers"
#endif

struct TestCase {
  const char* name;
  void (*run)();
};

struct TestGroup {
  const struct TestCase* cases;
  size_t count;
};

void check_impl(bool passed, const char* text, const char* file, int line);
void check_eq_impl(long long actual, long long expected, const char* text,
                   const char* file, int line);
void check_bytes_impl(const void* actual, const void* expected, size_t size,
                      const char* text, const char* file, int line);

// The macros need the call site and the text of the expression.
// NOLINTBEGIN(cppcoreguidelines-macro-usage)
#define CHECK(cond) check_impl((cond), #cond, __FILE__, __LINE__)
#define CHECK_EQ(actual, expected)                          \
  check_eq_impl((long long)(actual), (long long)(expected), \
                #actual " == " #expected, __FILE__, __LINE__)
#define CHECK_BYTES(actual, expected, size)                                \
  check_bytes_impl((actual), (expected), (size), #actual " == " #expected, \
                   __FILE__, __LINE__)
#define TEST_CASE(fn) {#fn, fn}
// NOLINTEND(cppcoreguidelines-macro-usage)

/* Number of checks that failed so far. */
int test_failures();

/* The program's own state, as the tests see it. The layout is that of
 * struct KbdState in the program. */
struct KbdView {
  __u8 fn_lock;
  __u8 backlight;
};

struct KbdView* kbd_view();

/* Sets a udev property variable as the loader does: the text and its
 * terminator, as far as the four bytes go. */
void set_property(char* property, const char* text);

/* A fresh start for each test: the fakes, the state, the udev properties. */
void test_reset();

// The program under test. Plain declarations: the program is a separate
// translation unit, and these are the names the BPF loader looks for.
int zenbook_duo_kbd_fix_event(struct hid_bpf_ctx* hctx);
int zenbook_duo_kbd_fix_rdesc(struct hid_bpf_ctx* hctx);
int probe(struct hid_bpf_probe_args* ctx);
// NOLINTBEGIN(readability-identifier-naming,cppcoreguidelines-avoid-non-const-global-variables)
extern char UDEV_PROP_ASUS_UX8406_KBD_BACKLIGHT[4];
extern char UDEV_PROP_ASUS_UX8406_KBD_FN_LOCK[4];
extern const char zenbook_duo_kbd_state;
extern const unsigned int hid_bpf_config_devices[][4];
extern struct hid_bpf_ops zenbook_duo_keyboard;
// NOLINTEND(readability-identifier-naming,cppcoreguidelines-avoid-non-const-global-variables)

/* The groups of tests. */
extern const struct TestGroup kEventTests;
extern const struct TestGroup kRdescTests;
extern const struct TestGroup kProbeTests;
extern const struct TestGroup kAsyncTests;

#endif  // UX8406_TESTS_TEST_SUPPORT_H_
