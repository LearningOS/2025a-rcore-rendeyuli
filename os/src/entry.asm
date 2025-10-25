    .section .text.entry
    .globl _start
_start:
    la sp, boot_stack_top
    call rust_main //应用入口

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 16  //64KiB操作系统栈空间
    .globl boot_stack_top
boot_stack_top: