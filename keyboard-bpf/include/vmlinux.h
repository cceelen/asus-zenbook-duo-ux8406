/*
 * The kernel types the HID-BPF program and its helper headers use.
 *
 * A build normally gets these from a vmlinux.h that bpftool generates from
 * the BTF of the running kernel. A package build has no particular kernel at
 * hand, so the handful of types needed is written down here. The kernel-side
 * structures carry preserve_access_index: the offsets of their members are
 * looked up in the BTF of the kernel the program is loaded into, and members
 * left out here do not matter.
 *
 * Everything else comes from the kernel's exported headers.
 */
#ifndef __VMLINUX_H__
#define __VMLINUX_H__

#include <linux/bpf.h>
#include <linux/types.h>
#include <stdbool.h>
#include <stddef.h>

typedef __u8 u8;
typedef __u16 u16;
typedef __u32 u32;
typedef __u64 u64;
typedef __s32 s32;

#pragma clang attribute push(__attribute__((preserve_access_index)), \
			     apply_to = record)

struct list_head {
	struct list_head *next;
	struct list_head *prev;
};

struct hid_device {
	char name[128];
	unsigned int id;
};

struct hid_bpf_ctx {
	struct hid_device *hid;
	__u32 allocated_size;
	union {
		__s32 retval;
		__s32 size;
	};
};

enum hid_report_type {
	HID_INPUT_REPORT = 0,
	HID_OUTPUT_REPORT = 1,
	HID_FEATURE_REPORT = 2,
	HID_REPORT_TYPES = 3,
};

enum hid_class_request {
	HID_REQ_GET_REPORT = 1,
	HID_REQ_GET_IDLE = 2,
	HID_REQ_GET_PROTOCOL = 3,
	HID_REQ_SET_REPORT = 9,
	HID_REQ_SET_IDLE = 10,
	HID_REQ_SET_PROTOCOL = 11,
};

struct hid_bpf_ops {
	int hid_id;
	u32 flags;
	struct list_head list;
	int (*hid_device_event)(struct hid_bpf_ctx *ctx,
				enum hid_report_type report_type, u64 source);
	int (*hid_rdesc_fixup)(struct hid_bpf_ctx *ctx);
	int (*hid_hw_request)(struct hid_bpf_ctx *ctx, unsigned char reportnum,
			      enum hid_report_type rtype,
			      enum hid_class_request reqtype, u64 source);
	int (*hid_hw_output_report)(struct hid_bpf_ctx *ctx, u64 source);
	struct hid_device *hdev;
};

#pragma clang attribute pop

#endif /* __VMLINUX_H__ */
