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
