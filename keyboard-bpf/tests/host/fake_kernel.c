// Host fakes of the kernel side of the HID-BPF program; see
// shim/fake_kernel.h for what each one models.
#include "fake_kernel.h"

#include <stddef.h>
#include <string.h>

struct FakeKernel* fake_kernel() {
  static struct FakeKernel state;

  return &state;
}

void fake_reset() {
  struct FakeKernel* fake = fake_kernel();

  memset(fake, 0, sizeof(*fake));
  fake->init_fail_at = -1;
}

// NOLINTNEXTLINE(bugprone-easily-swappable-parameters): the kernel's signature
void* bpf_map_lookup_elem(void* map, const void* key) {
  struct FakeKernel* fake = fake_kernel();

  fake->map_lookups++;
  if (fake->map_identity != nullptr && map != fake->map_identity) {
    fake->map_wrong_calls++;
    return nullptr;
  }
  if (*(const __u32*)key != 0) {
    fake->map_wrong_calls++;
    return nullptr;
  }
  if (fake->map_fail != 0) {
    return nullptr;
  }
  return fake->map_value;
}

__u8* hid_bpf_get_data(struct hid_bpf_ctx* ctx, unsigned int offset,
                       size_t size) {
  struct FakeKernel* fake = fake_kernel();

  fake->data_calls++;
  if (fake->data_fail != 0 || offset + size > ctx->allocated_size ||
      offset + size > sizeof(fake->data)) {
    return nullptr;
  }
  return fake->data + offset;
}

// NOLINTNEXTLINE(bugprone-easily-swappable-parameters): the kernel's signature
int hid_bpf_hw_request(struct hid_bpf_ctx* ctx, __u8* data, size_t size,
                       enum hid_report_type type,
                       enum hid_class_request reqtype) {
  struct FakeKernel* fake = fake_kernel();
  const int index = fake->hw_count;
  struct FakeHwCall* call = &fake->hw[index % kFakeMaxCalls];

  fake->hw_count++;
  call->ctx = ctx;
  call->size = size; /* as asked, whatever fits is copied */
  call->type = type;
  call->reqtype = reqtype;
  if (size > sizeof(call->report)) {
    fake->hw_oversize++;
    size = sizeof(call->report);
  }
  memset(call->report, 0, sizeof(call->report));
  memcpy(call->report, data, size);
  return index < kFakeMaxCalls ? fake->hw_results[index] : fake->hw_default;
}

struct hid_bpf_ctx* hid_bpf_allocate_context(unsigned int hid) {
  struct FakeKernel* fake = fake_kernel();

  if (fake->alloc_fail != 0) {
    return nullptr;
  }
  for (int i = 0; i < kFakePoolSize; i++) {
    if (fake->in_use[i] == 0) {
      fake->in_use[i] = 1;
      fake->allocs++;
      fake->alloc_hid[i] = hid;
      fake->pool[i] = (struct hid_bpf_ctx){
          .hid = hid,
          .allocated_size = kFakeDataSize,
          .size = 0,
      };
      return &fake->pool[i];
    }
  }
  return nullptr;
}

void hid_bpf_release_context(struct hid_bpf_ctx* ctx) {
  struct FakeKernel* fake = fake_kernel();
  const ptrdiff_t index = ctx - fake->pool;

  fake->releases++;
  if (index < 0 || index >= kFakePoolSize || fake->in_use[index] == 0) {
    fake->bad_releases++;
    return;
  }
  fake->in_use[index] = 0;
}

int fake_contexts_in_use() {
  const struct FakeKernel* fake = fake_kernel();
  int count = 0;

  for (int i = 0; i < kFakePoolSize; i++) {
    count += fake->in_use[i];
  }
  return count;
}

int fake_async_init(int* key) {
  struct FakeKernel* fake = fake_kernel();
  const int call = fake->init_calls++;

  if (call == fake->init_fail_at) {
    *key = fake->init_error;
    return fake->init_error;
  }
  *key = fake->next_key++;
  return 0;
}

int hid_bpf_async_delayed_call(struct hid_bpf_ctx* ctx, __u64 milliseconds,
                               int key, int (*callback)(struct hid_bpf_ctx*)) {
  struct FakeKernel* fake = fake_kernel();

  if (fake->queue_count >= kFakeMaxCalls) {
    return -ENOSPC;
  }
  fake->queue[fake->queue_count++] = (struct FakeDelayedCall){
      .hid = ctx->hid,
      .milliseconds = milliseconds,
      .key = key,
      .callback = callback,
  };
  return 0;
}

int fake_run_delayed() {
  struct FakeKernel* fake = fake_kernel();
  const int count = fake->queue_count;

  for (int i = 0; i < count; i++) {
    struct hid_bpf_ctx* ctx = hid_bpf_allocate_context(fake->queue[i].hid);

    fake->queue_results[i] =
        ctx == nullptr ? -ENOMEM : fake->queue[i].callback(ctx);
    if (ctx != nullptr) {
      hid_bpf_release_context(ctx);
    }
  }
  fake->queue_count = 0;
  return count;
}
