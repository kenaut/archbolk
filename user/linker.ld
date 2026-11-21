ENTRY(_start)
SECTIONS
{
    /* Link base: programs are loaded at a dynamic user slot base + this
     * offset. RIP-relative addressing makes the code position-independent. */
    . = 0x1000;
    .text : ALIGN(4096) {
        *(.text*)
    }
    .rodata : ALIGN(4096) {
        *(.rodata*)
    }
    .data : ALIGN(4096) {
        *(.data*)
    }
    .bss : ALIGN(4096) {
        *(.bss*)
    }
}
