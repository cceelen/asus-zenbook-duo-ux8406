// zenbook_duo_kbd_fix_rdesc, and the declarations the loader reads (the
// device table and the struct_ops).
#include <stdint.h>
#include <string.h>

#include "captures.h"
#include "fake_kernel.h"
#include "test_support.h"

enum {
  kUsbSize = 90,
  kUsbOffset = 57,
  kBtSize = 257,
  kBtOffset = 167,
  kFill = 0xa5,
};

/* Start the descriptor buffer as the kernel gives it: the capture, then
 * filler up to the end of the buffer. */
static struct hid_bpf_ctx load(const unsigned char* capture, size_t size) {
  struct FakeKernel* fake = fake_kernel();

  memset(fake->data, kFill, sizeof(fake->data));
  memcpy(fake->data, capture, size);
  return (struct hid_bpf_ctx){
      .hid = 1,
      .allocated_size = kFakeDataSize,
      .size = (__s32)size,
  };
}

static void usb_descriptor() {
  struct hid_bpf_ctx hctx = load(kUsbCapture, sizeof(kUsbCapture));
  const unsigned char* data = fake_kernel()->data;

  CHECK_EQ(sizeof(kUsbCapture), kUsbSize);
  CHECK_EQ(zenbook_duo_kbd_fix_rdesc(&hctx),
           kUsbOffset + sizeof(kExpectedHotkeys));
  CHECK_BYTES(data, kUsbCapture, kUsbOffset);
  CHECK_BYTES(data + kUsbOffset, kExpectedHotkeys, sizeof(kExpectedHotkeys));
  CHECK_EQ(data[kUsbOffset + sizeof(kExpectedHotkeys)], kFill);
}

static void bluetooth_descriptor() {
  struct hid_bpf_ctx hctx = load(kBtCapture, sizeof(kBtCapture));
  const unsigned char* data = fake_kernel()->data;
  const size_t hotkeys_end = kBtOffset + sizeof(kExpectedHotkeys);
  /* The vendor collection after the hotkeys is the captured one. */
  const size_t capture_tail = kBtOffset + 33;

  CHECK_EQ(sizeof(kBtCapture), kBtSize);
  CHECK_EQ(zenbook_duo_kbd_fix_rdesc(&hctx),
           hotkeys_end + sizeof(kBtCapture) - capture_tail);
  CHECK_BYTES(data, kBtCapture, kBtOffset);
  CHECK_BYTES(data + kBtOffset, kExpectedHotkeys, sizeof(kExpectedHotkeys));
  CHECK_BYTES(data + hotkeys_end, kBtCapture + capture_tail,
              sizeof(kBtCapture) - capture_tail);
  CHECK_EQ(data[sizeof(kBtCapture) + sizeof(kExpectedHotkeys) - 33], kFill);
}

static void other_sizes_are_left_alone() {
  const int sizes[] = {
      0, 1, kUsbSize - 1, kUsbSize + 1, kBtSize - 1, kBtSize + 1, kFakeDataSize,
  };
  unsigned char before[kFakeDataSize];

  for (size_t i = 0; i < sizeof(sizes) / sizeof(sizes[0]); i++) {
    struct hid_bpf_ctx hctx = load(kBtCapture, sizeof(kBtCapture));

    hctx.size = sizes[i];
    memcpy(before, fake_kernel()->data, sizeof(before));
    CHECK_EQ(zenbook_duo_kbd_fix_rdesc(&hctx), 0);
    CHECK_BYTES(fake_kernel()->data, before, sizeof(before));
  }
}

static void no_data() {
  struct hid_bpf_ctx hctx = load(kUsbCapture, sizeof(kUsbCapture));
  unsigned char before[kFakeDataSize];

  fake_kernel()->data_fail = 1;
  memcpy(before, fake_kernel()->data, sizeof(before));
  CHECK_EQ(zenbook_duo_kbd_fix_rdesc(&hctx), 0);
  CHECK_BYTES(fake_kernel()->data, before, sizeof(before));
}

static void buffer_smaller_than_a_descriptor() {
  struct hid_bpf_ctx hctx = load(kUsbCapture, sizeof(kUsbCapture));

  hctx.allocated_size = kFakeDataSize - 1;
  CHECK_EQ(zenbook_duo_kbd_fix_rdesc(&hctx), 0);
  CHECK_BYTES(fake_kernel()->data, kUsbCapture, sizeof(kUsbCapture));
}

static void device_table() {
  enum { kBusUsb = 3, kBusBluetooth = 5, kGroupGeneric = 1 };
  const unsigned int want[][4] = {
      {kBusUsb, kGroupGeneric, 0x0b05, 0x1bf2},
      {kBusBluetooth, kGroupGeneric, 0x0b05, 0x1bf3},
      {kBusUsb, kGroupGeneric, 0x0b05, 0x1b2c},
      {kBusBluetooth, kGroupGeneric, 0x0b05, 0x1b2d},
  };

  CHECK_BYTES(hid_bpf_config_devices, want, sizeof(want));
}

static void struct_ops() {
  /* The slots have the kernel's signatures, the programs the BPF_PROG ones:
   * compare the addresses. */
  CHECK((uintptr_t)zenbook_duo_keyboard.hid_device_event ==
        (uintptr_t)zenbook_duo_kbd_fix_event);
  CHECK((uintptr_t)zenbook_duo_keyboard.hid_rdesc_fixup ==
        (uintptr_t)zenbook_duo_kbd_fix_rdesc);
}

static const struct TestCase kCases[] = {
    TEST_CASE(usb_descriptor),
    TEST_CASE(bluetooth_descriptor),
    TEST_CASE(other_sizes_are_left_alone),
    TEST_CASE(no_data),
    TEST_CASE(buffer_smaller_than_a_descriptor),
    TEST_CASE(device_table),
    TEST_CASE(struct_ops),
};

const struct TestGroup kRdescTests = {
    kCases,
    sizeof(kCases) / sizeof(kCases[0]),
};
