/*
 * bpf_wq_set_callback() on kernels before 7.0.
 *
 * From 6.10 to 6.19 the kernel exports this kfunc as
 * bpf_wq_set_callback_impl(), with one more argument that the verifier fills
 * in; 7.0 renamed it. hid_bpf_async.h calls the new name, which on an older
 * kernel stays unresolved and makes the whole program fail to load. Include
 * this after hid_bpf_helpers.h and before hid_bpf_async.h: the call then goes
 * to whichever of the two the running kernel has.
 *
 * Loaded and used on 7.2 only; the older path is untested.
 */
#ifndef __WQ_COMPAT_H__
#define __WQ_COMPAT_H__

extern int bpf_wq_set_callback_impl(struct bpf_wq *wq,
				    int (*callback_fn)(void *, int *, void *),
				    unsigned int flags,
				    void *aux__ign) __weak __ksym;

static __always_inline int
wq_set_callback(struct bpf_wq *wq, int (*callback_fn)(void *, int *, void *),
		unsigned int flags)
{
	/* The parentheses keep the macro below from rewriting this call. */
	if (bpf_ksym_exists(bpf_wq_set_callback))
		return (bpf_wq_set_callback)(wq, callback_fn, flags);

	return bpf_wq_set_callback_impl(wq, callback_fn, flags, NULL);
}

#define bpf_wq_set_callback(wq, callback_fn, flags) \
	wq_set_callback(wq, callback_fn, flags)

#endif /* __WQ_COMPAT_H__ */
