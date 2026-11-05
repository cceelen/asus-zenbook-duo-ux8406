// Runs the groups of tests against the HID-BPF program, one fresh start each.
#include <stdio.h>

#include "test_support.h"

static int run_group(const struct TestGroup* group) {
  int run = 0;

  for (size_t i = 0; i < group->count; i++) {
    const int before = test_failures();

    test_reset();
    group->cases[i].run();
    run++;
    if (test_failures() != before) {
      (void)fprintf(stderr, "FAIL %s\n", group->cases[i].name);
    }
  }
  return run;
}

int main() {
  const struct TestGroup* groups[] = {
      &kEventTests,
      &kAsyncTests,
      &kRdescTests,
      &kProbeTests,
  };
  int run = 0;

  for (size_t i = 0; i < sizeof(groups) / sizeof(groups[0]); i++) {
    run += run_group(groups[i]);
  }
  (void)printf("%d tests, %d failed checks\n", run, test_failures());
  return test_failures() == 0 ? 0 : 1;
}
