.intel_syntax noprefix
.text
.code64

/* Exceptions that push an error code: 8, 10, 11, 12, 13, 14, 17, 21, 29, 30 */
/* All others: push a dummy 0 to keep the stack layout uniform. */

.extern isr_common_handler

/* Macro: generate a stub for vector `vec`.                                  */
/* If `has_error` is 1, the CPU already pushed an error code.                */
/* If `has_error` is 0, we push a dummy 0.                                   */
.macro isr_stub vec, has_error
.globl isr_stub_\vec
isr_stub_\vec:
    .if \has_error == 0
        push 0
    .endif
    push \vec
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    mov rdi, rsp

    call isr_common_handler

    /* Preemptive scheduling: if the timer requested a reschedule,
     * switch to the next task's interrupt frame. The generic pop/iretq
     * below will then restore the *new* task's context.
     * rdi = old_rsp (current frame), rax = new_rsp (0 = no switch). */
    mov rdi, rsp
    call switch_from_interrupt
    test rax, rax
    jz 1f
    mov rsp, rax
1:

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    add rsp, 16

    iretq
.endm

/* Vectors 0-31: CPU exceptions */
isr_stub 0, 0
isr_stub 1, 0
isr_stub 2, 0
isr_stub 3, 0
isr_stub 4, 0
isr_stub 5, 0
isr_stub 6, 0
isr_stub 7, 0
isr_stub 8, 1
isr_stub 9, 0
isr_stub 10, 1
isr_stub 11, 1
isr_stub 12, 1
isr_stub 13, 1
isr_stub 14, 1
isr_stub 15, 0
isr_stub 16, 0
isr_stub 17, 1
isr_stub 18, 0
isr_stub 19, 0
isr_stub 20, 0
isr_stub 21, 1
isr_stub 22, 0
isr_stub 23, 0
isr_stub 24, 0
isr_stub 25, 0
isr_stub 26, 0
isr_stub 27, 0
isr_stub 28, 0
isr_stub 29, 1
isr_stub 30, 1
isr_stub 31, 0

/* Vectors 32-255: hardware IRQs and unused */
isr_stub 32, 0
isr_stub 33, 0
isr_stub 34, 0
isr_stub 35, 0
isr_stub 36, 0
isr_stub 37, 0
isr_stub 38, 0
isr_stub 39, 0
isr_stub 40, 0
isr_stub 41, 0
isr_stub 42, 0
isr_stub 43, 0
isr_stub 44, 0
isr_stub 45, 0
isr_stub 46, 0
isr_stub 47, 0
isr_stub 48, 0
isr_stub 49, 0
isr_stub 50, 0
isr_stub 51, 0
isr_stub 52, 0
isr_stub 53, 0
isr_stub 54, 0
isr_stub 55, 0
isr_stub 56, 0
isr_stub 57, 0
isr_stub 58, 0
isr_stub 59, 0
isr_stub 60, 0
isr_stub 61, 0
isr_stub 62, 0
isr_stub 63, 0
isr_stub 64, 0
isr_stub 65, 0
isr_stub 66, 0
isr_stub 67, 0
isr_stub 68, 0
isr_stub 69, 0
isr_stub 70, 0
isr_stub 71, 0
isr_stub 72, 0
isr_stub 73, 0
isr_stub 74, 0
isr_stub 75, 0
isr_stub 76, 0
isr_stub 77, 0
isr_stub 78, 0
isr_stub 79, 0
isr_stub 80, 0
isr_stub 81, 0
isr_stub 82, 0
isr_stub 83, 0
isr_stub 84, 0
isr_stub 85, 0
isr_stub 86, 0
isr_stub 87, 0
isr_stub 88, 0
isr_stub 89, 0
isr_stub 90, 0
isr_stub 91, 0
isr_stub 92, 0
isr_stub 93, 0
isr_stub 94, 0
isr_stub 95, 0
isr_stub 96, 0
isr_stub 97, 0
isr_stub 98, 0
isr_stub 99, 0
isr_stub 100, 0
isr_stub 101, 0
isr_stub 102, 0
isr_stub 103, 0
isr_stub 104, 0
isr_stub 105, 0
isr_stub 106, 0
isr_stub 107, 0
isr_stub 108, 0
isr_stub 109, 0
isr_stub 110, 0
isr_stub 111, 0
isr_stub 112, 0
isr_stub 113, 0
isr_stub 114, 0
isr_stub 115, 0
isr_stub 116, 0
isr_stub 117, 0
isr_stub 118, 0
isr_stub 119, 0
isr_stub 120, 0
isr_stub 121, 0
isr_stub 122, 0
isr_stub 123, 0
isr_stub 124, 0
isr_stub 125, 0
isr_stub 126, 0
isr_stub 127, 0
isr_stub 128, 0
isr_stub 129, 0
isr_stub 130, 0
isr_stub 131, 0
isr_stub 132, 0
isr_stub 133, 0
isr_stub 134, 0
isr_stub 135, 0
isr_stub 136, 0
isr_stub 137, 0
isr_stub 138, 0
isr_stub 139, 0
isr_stub 140, 0
isr_stub 141, 0
isr_stub 142, 0
isr_stub 143, 0
isr_stub 144, 0
isr_stub 145, 0
isr_stub 146, 0
isr_stub 147, 0
isr_stub 148, 0
isr_stub 149, 0
isr_stub 150, 0
isr_stub 151, 0
isr_stub 152, 0
isr_stub 153, 0
isr_stub 154, 0
isr_stub 155, 0
isr_stub 156, 0
isr_stub 157, 0
isr_stub 158, 0
isr_stub 159, 0
isr_stub 160, 0
isr_stub 161, 0
isr_stub 162, 0
isr_stub 163, 0
isr_stub 164, 0
isr_stub 165, 0
isr_stub 166, 0
isr_stub 167, 0
isr_stub 168, 0
isr_stub 169, 0
isr_stub 170, 0
isr_stub 171, 0
isr_stub 172, 0
isr_stub 173, 0
isr_stub 174, 0
isr_stub 175, 0
isr_stub 176, 0
isr_stub 177, 0
isr_stub 178, 0
isr_stub 179, 0
isr_stub 180, 0
isr_stub 181, 0
isr_stub 182, 0
isr_stub 183, 0
isr_stub 184, 0
isr_stub 185, 0
isr_stub 186, 0
isr_stub 187, 0
isr_stub 188, 0
isr_stub 189, 0
isr_stub 190, 0
isr_stub 191, 0
isr_stub 192, 0
isr_stub 193, 0
isr_stub 194, 0
isr_stub 195, 0
isr_stub 196, 0
isr_stub 197, 0
isr_stub 198, 0
isr_stub 199, 0
isr_stub 200, 0
isr_stub 201, 0
isr_stub 202, 0
isr_stub 203, 0
isr_stub 204, 0
isr_stub 205, 0
isr_stub 206, 0
isr_stub 207, 0
isr_stub 208, 0
isr_stub 209, 0
isr_stub 210, 0
isr_stub 211, 0
isr_stub 212, 0
isr_stub 213, 0
isr_stub 214, 0
isr_stub 215, 0
isr_stub 216, 0
isr_stub 217, 0
isr_stub 218, 0
isr_stub 219, 0
isr_stub 220, 0
isr_stub 221, 0
isr_stub 222, 0
isr_stub 223, 0
isr_stub 224, 0
isr_stub 225, 0
isr_stub 226, 0
isr_stub 227, 0
isr_stub 228, 0
isr_stub 229, 0
isr_stub 230, 0
isr_stub 231, 0
isr_stub 232, 0
isr_stub 233, 0
isr_stub 234, 0
isr_stub 235, 0
isr_stub 236, 0
isr_stub 237, 0
isr_stub 238, 0
isr_stub 239, 0
isr_stub 240, 0
isr_stub 241, 0
isr_stub 242, 0
isr_stub 243, 0
isr_stub 244, 0
isr_stub 245, 0
isr_stub 246, 0
isr_stub 247, 0
isr_stub 248, 0
isr_stub 249, 0
isr_stub 250, 0
isr_stub 251, 0
isr_stub 252, 0
isr_stub 253, 0
isr_stub 254, 0
isr_stub 255, 0

/* ── Syscall entry point (SYSCALL instruction target) ────────────────────
 *
 * On entry:
 *   RCX = return RIP (set by SYSCALL)
 *   R11 = original RFLAGS (set by SYSCALL)
 *   RAX = syscall number
 *   RDI, RSI, RDX, R10, R8, R9 = arguments 1-6
 *
 * We must save all registers that the C ABI expects the callee to preserve
 * (rbx, rbp, r12-r15), plus rcx and r11 which hold the return address and
 * flags.  We also save rax (return value slot) and the argument registers
 * so the handler can inspect them.
 *
 * Stack layout (top to bottom, RSP after all pushes):
 *   [original RAX]   <- offset 0
 *   [original RCX]   <- offset 8  (return RIP)
 *   [original R11]   <- offset 16 (return RFLAGS)
 *   [original RDI]   <- offset 24
 *   [original RSI]   <- offset 32
 *   [original RDX]   <- offset 40
 *   [original R10]   <- offset 48
 *   [original R8]    <- offset 56
 *   [original R9]    <- offset 64
 *   [RBX]            <- offset 72
 *   [RBP]            <- offset 80
 *   [R12]            <- offset 88
 *   [R13]            <- offset 96
 *   [R14]            <- offset 104
 *   [R15]            <- offset 112  <- RSP
 *
 * After the handler returns, RAX holds the return value.
 */
.globl syscall_entry
syscall_entry:
    /* Check if we came from ring 0 (kernel) or ring 3 (user).
     * SYSCALL from ring 0: CS stays ring 0, we can ret.
     * SYSCALL from ring 3: CS becomes ring 0, we must sysretq.
     * We read the saved CS from the stack... actually SYSCALL doesn't
     * push CS.  Instead, check the original CS by reading it now
     * (before we modify anything).  But SYSCALL doesn't save CS.
     *
     * Simpler approach: always use a kernel-compatible return.
     * We'll use ret for now (works from ring 0).
     * When user mode is implemented, we'll switch to sysretq.
     */
    /* Save all registers on the caller's stack.
     * No stack switch needed — we're called from ring 0 (kernel),
     * so the caller's stack is valid and we must preserve it.
     * (A stack switch would only be needed for ring-3 callers.) */

    /* Save all registers */
    push    r15
    push    r14
    push    r13
    push    r12
    push    rbp
    push    rbx
    push    r9
    push    r8
    push    r10
    push    rdx
    push    rsi
    push    rdi
    push    r11
    push    rcx
    push    rax

    /* First argument: pointer to the saved register block (RSP) */
    mov     rdi, rsp

    /* Call the Rust syscall handler */
    call    syscall_handler

    /* Restore registers.
     * After call returns, RSP points at frame.rax (the first pushed value).
     * The call's return address was above the saved block and was consumed
     * by ret, so RSP is back at the top of the saved register block. */
    pop     rax             /* load return value from frame.rax */
    pop     rcx             /* return RIP */
    pop     r11             /* return RFLAGS */
    pop     rdi
    pop     rsi
    pop     rdx
    pop     r10
    pop     r8
    pop     r9
    pop     rbx
    pop     rbp
    pop     r12
    pop     r13
    pop     r14
    pop     r15

    /* Decide return path by bit 37 of the saved return RIP in rcx.
     * User code lives at 0x200_0000_0000 (bit 37 set); kernel addresses
     * are far below. `bt` does not clobber rcx or the return value rax. */
    bt      rcx, 37
    jc      1f

    /* Ring 0: return with ret. */
    push    rcx
    ret

1:
    /* Ring 3: sysretq. rcx = return RIP, r11 = return RFLAGS (restored above).
     * STAR MSR supplies user CS (0x20|3=0x23) and SS (0x18|3=0x1b).
     * RSP is unchanged. */
    sysretq

/* ── Ring-3 demo program ────────────────────────────────────────────────
 * Runs in user mode via iretq. Writes a message with sys_write, then exits.
 * All code/data must be within one 4 KiB page so map_user_page can expose it.
 */
.globl user_demo
user_demo:
    /* sys_write("Hello from ring 3!\n") */
    mov rax, 1
    mov rdi, 1
    lea rsi, [rip + user_msg]
    mov rdx, 19
    syscall

    /* sys_getpid() — result in rax (not printed, verified via kernel log) */
    mov rax, 39
    syscall

    /* sys_sched_yield() */
    mov rax, 24
    syscall
    /* (sched_yield is a no-op in this kernel; timer IRQ handles preemption) */

    /* sys_nanosleep(20 ms) */
    mov rax, 35
    mov rdi, 20
    syscall

    /* sys_brk(0) query, then extend by 0x1000 */
    mov rax, 12
    xor rdi, rdi
    syscall
    mov rdi, rax
    add rdi, 4096
    mov rax, 12
    syscall

    mov rax, 1
    mov rdi, 1
    lea rsi, [rip + done_msg]
    mov rdx, 24
    syscall

    mov rax, 60                /* SYS_EXIT */
    xor rdi, rdi
    syscall

.user_loop:
    jmp .user_loop

user_msg:
    .ascii "Hello from ring 3!\n"
done_msg:
    .ascii "syscalls from ring3 OK!\n"
