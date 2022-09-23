ENTRY(_start);

/* RAM on opensbi starts at 0x80000000 */
.  = 0x80000000;
/* Open SBI reserves 2^19 bytes at start of ram */
. +=    0x80000;
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
    }
    .data : ALIGN(4K) {
        *(.data);
    }
    PROVIDE(_end_of_data = .);
}
