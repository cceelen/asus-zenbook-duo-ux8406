#include "test_support.h"

#include <stdio.h>
#include <string.h>

#include "fake_kernel.h"

static int* failure_counter() {
  static int failures;

  return &failures;
}

int test_failures() { return *failure_counter(); }

void check_impl(bool passed, const char* text, const char* file, int line) {
  if (!passed) {
    (void)fprintf(stderr, "%s:%d: check failed: %s\n", file, line, text);
    (*failure_counter())++;
  }
}

void check_eq_impl(long long actual, long long expected, const char* text,
                   const char* file, int line) {
  if (actual != expected) {
    (void)fprintf(stderr, "%s:%d: check failed: %s (got %lld, want %lld)\n",
                  file, line, text, actual, expected);
    (*failure_counter())++;
  }
}

void check_bytes_impl(const void* actual, const void* expected, size_t size,
                      const char* text, const char* file, int line) {
  if (memcmp(actual, expected, size) != 0) {
    (void)fprintf(stderr, "%s:%d: check failed: %s (bytes differ)\n", file,
                  line, text);
    (*failure_counter())++;
  }
}

struct KbdView* kbd_view() { return (struct KbdView*)fake_kernel()->map_value; }

void test_reset() {
  fake_reset();
  fake_kernel()->map_identity = &zenbook_duo_kbd_state;
  memset(UDEV_PROP_ASUS_UX8406_KBD_BACKLIGHT, 0,
         sizeof(UDEV_PROP_ASUS_UX8406_KBD_BACKLIGHT));
  memset(UDEV_PROP_ASUS_UX8406_KBD_FN_LOCK, 0,
         sizeof(UDEV_PROP_ASUS_UX8406_KBD_FN_LOCK));
}

void set_property(char* property, const char* text) {
  const size_t size = sizeof(UDEV_PROP_ASUS_UX8406_KBD_FN_LOCK);
  size_t length = strlen(text) + 1;

  length = length < size ? length : size;
  for (size_t i = 0; i < length; i++) {
    property[i] = text[i];
  }
}
