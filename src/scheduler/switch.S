.intel_syntax noprefix
.text
.code64

/* void swtch(usize *old_rsp, usize new_rsp);
 *
 * Save callee-saved registers of the current task onto its stack,
 * then restore the callee-saved registers of the next task.
 *
 * The saved RSP points to the pushed rbx (lowest address).
 * When switching back, RSP is restored to this same point,
 * so the pops and ret return to the instruction after the
 * original swtch call.
 *
 * Stack layout (low to high address):
 *   [rbx]            <- RSP after all pushes (popped 1st)
 *   [rbp]            <- popped 2nd
 *   [r12]            <- popped 3rd
 *   [r13]            <- popped 4th
 *   [r14]            <- popped 5th
 *   [r15]            <- popped 6th
 *   [return_address] <- ret reads this (highest address)
 */
.globl swtch
swtch:
    /* Save current task's registers */
    push    r15
    push    r14
    push    r13
    push    r12
    push    rbp
    push    rbx

    /* Save current RSP (pointing to pushed rbx) into *old_rsp (rdi) */
    mov     qword ptr [rdi], rsp

    /* Load new RSP from rsi */
    mov     rsp, rsi

    /* Restore new task's registers */
    pop     rbx
    pop     rbp
    pop     r12
    pop     r13
    pop     r14
    pop     r15

    /* Return to the new task's execution point */
    ret
