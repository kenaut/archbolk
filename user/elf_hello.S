/* Freestanding ring-3 user program: prints a message via sys_write, then exits.
 * Linked at 0x207_0000_0000 (ELF slot base in the kernel). */
.intel_syntax noprefix
.text
.globl _start
_start:
    mov rax, 1                 /* SYS_WRITE */
    mov rdi, 1                 /* stdout */
    lea rsi, [rip + msg]
    mov rdx, 24                /* length */
    syscall

    mov rax, 60                /* SYS_EXIT */
    xor rdi, rdi
    syscall

.hang:
    jmp .hang

msg:
    .ascii "Hello from external ELF!\n"
