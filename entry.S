        .section .text.init

        .option norvc
        
        .type _start, @function
        .global _start
_start:
        .cfi_startproc

.option push
.option norelax
	la gp, __global_pointer
.option pop
	la sp, stack_top
        addi    sp,sp,-32
        sd      ra,24(sp)        
        
        mv      s0,a0
        mv      s1,a1

        // Call memset
        // memset(bss_start, 0, bss_end)
        la      a0,bss_start        

        li      a1,0

        la      a2,bss_end
        sub     a2,a2,a0
        
        call    memset

        mv      a0,s0                  // HeartID: i32
        mv      a1,s1                  // devtree address: *const u8
        la      a2,_start_of_data      // start_of_data: *const ()
        la      a3,_end_of_data        // end_of_data: *const ()
        /* Jump to kernel! */
        tail    kmain

        /* Should never get here */
        ebreak
        .cfi_endproc

        .end

