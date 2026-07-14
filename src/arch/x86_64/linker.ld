ENTRY(_start)

SECTIONS {
    . = 1M;

    .multiboot2_header : ALIGN(8) {
        KEEP(*(.multiboot2_header))
    }

    .text.boot : ALIGN(4K) {
        KEEP(*(.text.boot))
    }

    .text.boot64 : ALIGN(4K) {
        KEEP(*(.text.boot64))
    }

    .text : ALIGN(4K) {
        *(.text .text.*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }

    .note.Xen : ALIGN(4) {
        KEEP(*(.note.Xen))
    }

    .got : ALIGN(4K) {
        *(.got)
    }

    .data : ALIGN(4K) {
        *(.data .data.*)
        KEEP(*(.data.gdt))
    }

    .bss : ALIGN(4K) {
        *(COMMON)
        *(.bss .bss.*)
    }

    _kernel_end = .;

    /DISCARD/ : {
        *(.comment)
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.note.gnu.build-id)
    }
}
