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

        mv      a0,s0
        mv      a1,s1
        
        /* Jump to kernel! */
        tail    kmain

        /* Should never get here */
        ebreak
        .cfi_endproc

        .end

