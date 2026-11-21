ENTRY(_start)
SECTIONS
{
    /* ELF user slot 7 base (MAX_USER_PROGRAMS - 1). */
    . = 0x20700000000;
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
