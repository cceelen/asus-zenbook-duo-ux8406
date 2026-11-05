/*
 * Host fakes of the kernel side of the HID-BPF program: the macros of the BPF
 * headers and the kfuncs and helpers it calls. Written from what the API
 * means, not taken from the vendored headers. Every header the program
 * includes is a stub that includes this file, and this directory comes first
 * on the include path, so the program is compiled unchanged as host C.
 *
 * What a kfunc does here:
 *  - bpf_map_lookup_elem: the one-entry array map; the pointer to its value,
 *    or nullptr for a key other than 0, another map, or on demand.
 *  - hid_bpf_get_data: a window into a 4096 byte report buffer; nullptr when
 * the window is larger than the context's allocation, or on demand.
 *  - hid_bpf_hw_request: records the report; the result is programmable.
 *  - hid_bpf_allocate_context / hid_bpf_release_context: a pool of contexts;
 *    nullptr on demand; a second release or a foreign pointer is counted.
 *  - the HID_BPF_ASYNC_* macros: init numbers the work items and can fail;
 *    a delayed call is queued and fake_run_delayed() runs it.
 */
#ifndef UX8406_SHIM_FAKE_KERNEL_H_
#define UX8406_SHIM_FAKE_KERNEL_H_

#include <errno.h>
#include <stddef.h>
#include <stdint.h>

#define UX8406_HOST_SHIM 1

typedef uint8_t __u8;
typedef uint16_t __u16;
typedef uint32_t __u32;
typedef uint64_t __u64;
typedef int32_t __s32;
typedef uint64_t u64;

#undef __always_inline
#define __always_inline inline __attribute__((always_inline))
#define SEC(name)
/* The map declaration as byte arrays of the sizes it states: the symbol's
 * size (checked by run_host.py) shows the type, the entries, the key and the
 * value. */
#define __uint(name, val) char name[val]
#define __type(name, val) char name[sizeof(val)]

enum {
  BPF_MAP_TYPE_ARRAY = 2,
  BUS_USB = 3,
  BUS_BLUETOOTH = 5,
  HID_GROUP_GENERIC = 1,
  HID_MAX_DESCRIPTOR_SIZE = 4096,
  HID_IGNORE_EVENT = -1,
};

enum hid_report_type {
  HID_INPUT_REPORT = 0,
  HID_OUTPUT_REPORT = 1,
  HID_FEATURE_REPORT = 2,
};

enum hid_class_request {
  HID_REQ_GET_REPORT = 0x01,
  HID_REQ_SET_REPORT = 0x09,
};

#define HID_BPF_DEVICE_EVENT "struct_ops/hid_device_event"
#define HID_BPF_RDESC_FIXUP "struct_ops/hid_rdesc_fixup"

struct hid_bpf_ctx {
  unsigned int hid;
  __u32 allocated_size;
  union {
    __s32 retval;
    __s32 size;
  };
};

struct hid_bpf_probe_args {
  unsigned int hid;
  unsigned int rdesc_size;
  unsigned char rdesc[HID_MAX_DESCRIPTOR_SIZE];
  int retval;
};

struct hid_bpf_ops {
  int (*hid_device_event)(struct hid_bpf_ctx* ctx,
                          enum hid_report_type report_type, u64 source);
  int (*hid_rdesc_fixup)(struct hid_bpf_ctx* ctx);
};

/* A program is a plain function: the arguments come as written. */
#define BPF_PROG(name, ...) name(__VA_ARGS__)
#define HID_BPF_OPS(name) struct hid_bpf_ops name

/* The device table is kept as rows of bus, group, vendor, product. */
#define HID_DEVICE(bus, group, vendor, product) \
  {(bus), (group), (vendor), (product)}
#define HID_BPF_CONFIG(...) \
  const unsigned int hid_bpf_config_devices[][4] = {__VA_ARGS__}

/* The helpers and kfuncs. */
void* bpf_map_lookup_elem(void* map, const void* key);
__u8* hid_bpf_get_data(struct hid_bpf_ctx* ctx, unsigned int offset,
                       size_t size);
int hid_bpf_hw_request(struct hid_bpf_ctx* ctx, __u8* data, size_t size,
                       enum hid_report_type type,
                       enum hid_class_request reqtype);
struct hid_bpf_ctx* hid_bpf_allocate_context(unsigned int hid);
void hid_bpf_release_context(struct hid_bpf_ctx* ctx);
int hid_bpf_async_delayed_call(struct hid_bpf_ctx* ctx, __u64 milliseconds,
                               int key, int (*callback)(struct hid_bpf_ctx*));
/* Gives the next work item number, or fails; sets *key either way. */
int fake_async_init(int* key);

/*
 * The macros of hid_bpf_async.h: the work function, its init and the callback
 * that the queue runs. No branch is written in them: whatever they would
 * decide is in the fakes above.
 */
#define HID_BPF_ASYNC_FUN(fun)                                \
  fun(struct hid_bpf_ctx* ctx);                               \
  int ____key__##fun;                                         \
  __attribute__((unused)) static int ____async_init_##fun() { \
    return fake_async_init(&____key__##fun);                  \
  }                                                           \
  __attribute__((unused)) static int ____##fun##_cb(          \
      struct hid_bpf_ctx* hctx) {                             \
    return fun(hctx);                                         \
  }                                                           \
  __typeof__(fun(0)) fun
#define HID_BPF_ASYNC_INIT(fun) ____async_init_##fun()
#define HID_BPF_ASYNC_DELAYED_CALL(fun, ctx, delay) \
  hid_bpf_async_delayed_call(ctx, delay, ____key__##fun, ____##fun##_cb)

enum {
  kFakeMaxCalls = 16,
  kFakeReportBytes = 16,
  kFakePoolSize = 8,
  kFakeDataSize = HID_MAX_DESCRIPTOR_SIZE,
  kFakeMapValueSize = 16,
};

struct FakeHwCall {
  const struct hid_bpf_ctx* ctx;
  __u8 report[kFakeReportBytes];
  size_t size;
  enum hid_report_type type;
  enum hid_class_request reqtype;
};

struct FakeDelayedCall {
  unsigned int hid; /* of the context given; the context itself may be gone */
  __u64 milliseconds;
  int key;
  int (*callback)(struct hid_bpf_ctx*);
};

struct FakeKernel {
  /* The map. */
  const void* map_identity; /* the only map that exists; nullptr: any */
  _Alignas(8) __u8 map_value[kFakeMapValueSize];
  int map_lookups;
  int map_wrong_calls; /* lookups of another map or a key other than 0 */
  int map_fail;        /* make the lookup return nullptr */
  /* Report data. */
  __u8 data[kFakeDataSize];
  int data_fail; /* make hid_bpf_get_data return nullptr */
  int data_calls;
  /* Feature requests. */
  struct FakeHwCall hw[kFakeMaxCalls];
  int hw_count;
  int hw_results[kFakeMaxCalls]; /* result of call i, then hw_default */
  int hw_default;
  int hw_oversize; /* requests of more than a report buffer */
  /* Contexts. */
  struct hid_bpf_ctx pool[kFakePoolSize];
  int in_use[kFakePoolSize];
  int allocs;
  int releases;
  int bad_releases; /* a pointer that was not allocated, or released twice */
  int alloc_fail;   /* make hid_bpf_allocate_context return nullptr */
  unsigned int alloc_hid[kFakePoolSize];
  /* Work items. */
  int init_calls;
  int init_fail_at; /* the number of the call that fails; -1 for none */
  int init_error;
  int next_key;
  struct FakeDelayedCall queue[kFakeMaxCalls];
  int queue_count;
  int queue_results[kFakeMaxCalls]; /* what each callback returned */
};

/* The one state of the fakes. */
struct FakeKernel* fake_kernel();
/* Back to the start: everything works, nothing was called. */
void fake_reset();
/* Runs the queued callbacks as the work queue does: a fresh context for
 * each, released afterwards. Returns how many ran. */
int fake_run_delayed();
/* Contexts that were allocated and not released. */
int fake_contexts_in_use();

#endif /* UX8406_SHIM_FAKE_KERNEL_H_ */
