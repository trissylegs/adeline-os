ENTRY(_start);

/* Include entry point at start of binary */
/* . = 0x80000000; */
. = 0x80200000;
SECTIONS {
    PROVIDE(_start_of_data = .);
    .text.init : ALIGN(4K) {
        *(.text.init);         
    }
    .text.trap_entry : ALIGN(4K) {
        *(.text.trap_entry)
    }
    .text : ALIGN(4K) {        
        *(.text);
    }
    .bss : ALIGN(4K) {
        PROVIDE(bss_start = .);
        *(.bss);
        PROVIDE(stack_limit = .);
        . += 16384;
        PROVIDE(stack_top = .);
        . += 4096;
        PROVIDE(__global_pointer = .);
        PROVIDE(bss_end = .);
    }
    .rodata : ALIGN(4K) {
        *(.rodata);
        PROVIDE(__uart_base_addr = 0x10000000);
    }
    .data : ALIGN(4K) {
        *(.data);
    }
    PROVIDE(_end_of_data = .);
}
