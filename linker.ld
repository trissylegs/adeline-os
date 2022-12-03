ENTRY(_start);

/* RAM on opensbi starts at 0x80000000 */
.  = 0x80000000;
/* Open SBI reserves 2^19 bytes at start of ram */
. +=    0x80000;
SECTIONS {
    .text.init : ALIGN(4K) {
        __image_start = .;
        __text_start = .;
        *(.text.init);
        . = ALIGN(4096);
        *(.text);
        . = ALIGN(4096);
        __text_end = .;
    } 

    .rodata : ALIGN(4K) {
        __rodata_start = .;
        *(.rodata);
        . = ALIGN(4096);
        __rodata_end = .;
    }

    .data : ALIGN(4K) {
        __data_start = .;
        *(.data);
        . = ALIGN(4096);
        __data_end = .;
    }

    .bss : ALIGN(4K) {
        __bss_start = .;  
        *(.bss);
        *(.stack_guard);
        __stack_limit = .;
        . += 0x8000;
        PROVIDE(__stack_top = .);
        PROVIDE(__bss_end = .);

        . += 0x1000;
        PROVIDE(__global_pointer = .);
    }

    .tdata : ALIGN(4K) {
        __tdata_start = .;
        *(.tdata);
        . = ALIGN(4096);
        __tdata_end = .;
        __tbss_start = .;
        *(.tbss);
        . = ALIGN(4096);
        __tbss_end = .;
    }

    __image_end = .;

    /DISCARD/ : {
        *(.comment*)
        *(.eh_frame*)
        *(.gcc_except_table*)
        *(.note*)
        *(.rel.eh_frame*)
    }

}

