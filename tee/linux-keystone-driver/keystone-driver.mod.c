#include <linux/module.h>
#include <linux/export-internal.h>
#include <linux/compiler.h>

MODULE_INFO(name, KBUILD_MODNAME);

__visible struct module __this_module
__section(".gnu.linkonce.this_module") = {
	.name = KBUILD_MODNAME,
	.init = init_module,
#ifdef CONFIG_MODULE_UNLOAD
	.exit = cleanup_module,
#endif
	.arch = MODULE_ARCH_INIT,
};



static const struct modversion_info ____versions[]
__used __section("__versions") = {
	{ 0x88db9f48, "__check_object_size" },
	{ 0x67bd5f19, "misc_deregister" },
	{ 0x20978fb9, "idr_find" },
	{ 0x13c49cc2, "_copy_from_user" },
	{ 0xb0e602eb, "memmove" },
	{ 0xe422adc0, "pgtable_l5_enabled" },
	{ 0x222b1a5a, "remap_pfn_range" },
	{ 0x037a0cba, "kfree" },
	{ 0x4302d0eb, "free_pages" },
	{ 0x92997ed8, "_printk" },
	{ 0x73177675, "kernel_map" },
	{ 0xf0fdf6cb, "__stack_chk_fail" },
	{ 0x7665a95b, "idr_remove" },
	{ 0xb8f11603, "idr_alloc" },
	{ 0xf2c1cf1d, "__sbi_ecall" },
	{ 0x4c03a563, "random_kmalloc_seed" },
	{ 0x4dfa8d4b, "mutex_lock" },
	{ 0x6fb04d32, "dma_alloc_attrs" },
	{ 0x0be0c6fe, "pgtable_l4_enabled" },
	{ 0xfb578fc5, "memset" },
	{ 0x50123bf5, "misc_register" },
	{ 0x6b10bee1, "_copy_to_user" },
	{ 0x73c7bb3f, "dma_free_attrs" },
	{ 0x3213f038, "mutex_unlock" },
	{ 0x7141df95, "__kmalloc_cache_noprof" },
	{ 0x1c303cee, "validate_usercopy_range" },
	{ 0xccf69887, "get_free_pages_noprof" },
	{ 0xdd7ce51c, "kmalloc_caches" },
	{ 0x434ea330, "module_layout" },
};

MODULE_INFO(depends, "");


MODULE_INFO(srcversion, "5DBF7217ABD163BA2686D8B");
