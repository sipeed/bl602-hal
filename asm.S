#define STORE    sw
#define LOAD     lw
#define LOG_REGBYTES 2
#define REGBYTES (1 << LOG_REGBYTES)

/*
    Trap entry point (_start_trap)

    Saves caller saved registers ra, t0..6, a0..7, calls _start_trap_rust,
    restores caller saved registers and then returns.
*/
.section .trap, "ax"
.global _start_trap_hal
.option norelax
.align 6

_start_trap_hal:
    addi sp, sp, -16*REGBYTES

    STORE ra, 0*REGBYTES(sp)
    STORE t0, 1*REGBYTES(sp)
    STORE t1, 2*REGBYTES(sp)
    STORE t2, 3*REGBYTES(sp)
    STORE t3, 4*REGBYTES(sp)
    STORE t4, 5*REGBYTES(sp)
    STORE t5, 6*REGBYTES(sp)
    STORE t6, 7*REGBYTES(sp)
    STORE a0, 8*REGBYTES(sp)
    STORE a1, 9*REGBYTES(sp)
    STORE a2, 10*REGBYTES(sp)
    STORE a3, 11*REGBYTES(sp)
    STORE a4, 12*REGBYTES(sp)
    STORE a5, 13*REGBYTES(sp)
    STORE a6, 14*REGBYTES(sp)
    STORE a7, 15*REGBYTES(sp)

    add a0, sp, zero
    jal ra, _start_trap_rust_hal

    LOAD ra, 0*REGBYTES(sp)
    LOAD t0, 1*REGBYTES(sp)
    LOAD t1, 2*REGBYTES(sp)
    LOAD t2, 3*REGBYTES(sp)
    LOAD t3, 4*REGBYTES(sp)
    LOAD t4, 5*REGBYTES(sp)
    LOAD t5, 6*REGBYTES(sp)
    LOAD t6, 7*REGBYTES(sp)
    LOAD a0, 8*REGBYTES(sp)
    LOAD a1, 9*REGBYTES(sp)
    LOAD a2, 10*REGBYTES(sp)
    LOAD a3, 11*REGBYTES(sp)
    LOAD a4, 12*REGBYTES(sp)
    LOAD a5, 13*REGBYTES(sp)
    LOAD a6, 14*REGBYTES(sp)
    LOAD a7, 15*REGBYTES(sp)

    addi sp, sp, 16*REGBYTES
    mret
