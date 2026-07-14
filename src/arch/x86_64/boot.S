/* ── PVH ELF Note (for QEMU -kernel direct loading) ────────────────────
 *
 * QEMU 10.2+ requires a PVH ELF note to load a kernel with -kernel.
 * This note tells QEMU the 32-bit physical entry point.
 *
 * Layout:
 *   namesz = 4 ("Xen\0")
 *   descsz = 4 (32-bit physical address)
 *   type   = 18 (XEN_ELFNOTE_PHYS32_ENTRY)
 *   name   = "Xen\0"
 *   desc   = pvh_start absolute physical address
 *
 * We use .pushsection with @note and manually emit the note structure.
 * The .note type tells the linker to create a PT_NOTE program header.
 *
 * IMPORTANT: The desc value must be the ABSOLUTE physical address of
 * pvh_start, not relative to the load address. QEMU's read_pvh_start_addr()
 * reads this value and writes it to FW_CFG_KERNEL_ENTRY. pvh.bin then
 * jumps directly to this physical address. Since our kernel is loaded at
 * physical address 0x100000 (from the first LOAD segment), and pvh_start
 * is at virtual address 0x101016, the physical address is 0x101016.
 */
.pushsection .note.Xen, "a", @note
.balign 4
.long 2f - 1f          /* namesz = 4 ("Xen\0") */
.long 4f - 3f          /* descsz = 4 (32-bit phys addr) */
.long 18               /* type = XEN_ELFNOTE_PHYS32_ENTRY */
1: .asciz "Xen"
2: .balign 4
3:
.long pvh_start        /* desc: absolute physical address of pvh_start */
4: .balign 4
.popsection

.section .text.boot, "ax"

.code32

/* ── Multiboot2 entry point ────────────────────────────────────────────
 *
 * Entered from a Multiboot2-compatible bootloader (GRUB, Limine, etc.).
 * Register state:
 *   %eax = 0x36d76289 (Multiboot2 magic)
 *   %ebx = physical address of Multiboot2 info structure
 */
.globl _start
_start:
    movl    %eax, %edi
    movl    %ebx, %esi

    # Set up stack at 0x80000 (512 KiB stack, grows down to 0x00000)
    movl    $0x80000, %esp

    # Check multiboot magic
    cmpl    $0x36d76289, %eax
    jne     .Lfail

    # Fall through to common boot path
    jmp     .Lcommon_boot

/* ── PVH entry point ───────────────────────────────────────────────────
 *
 * Entered from QEMU -kernel (PVH ELF note). Register state per Xen ABI:
 *   %ebx = physical address of hvm_start_info structure
 *   %cr0: PE bit set, all other writeable bits cleared
 *   %cr4: all bits cleared
 *   %cs:  32-bit read/execute code segment, base=0, limit=0xFFFFFFFF
 *   %ds, %es: 32-bit read/write data segment, base=0, limit=0xFFFFFFFF
 *   %eflags: IF=0, TF=0, VM=0
 *
 * IMPORTANT: The hvm_start_info structure is at the address in %ebx,
 * which is typically in the range 0x1000-0x3000 (on the stack of the
 * pvh_main.c caller). Our page table setup below writes to 0x1000-0x3FFF,
 * which would OVERWRITE the hvm_start_info. So we must copy it to a
 * safe location (above our stack at 0x80000) before setting up page tables.
 *
 * We copy the structure to 0x90000 (above the stack at 0x80000).
 * The hvm_start_info is at most 56 bytes (version 1), so we copy 64 bytes.
 */
.globl pvh_start
pvh_start:
    # Save hvm_start_info address in %esi (will be passed to _start_64)
    movl    %ebx, %esi

    # Set up stack at 0x80000
    movl    $0x80000, %esp

    # Disable interrupts (should already be off per PVH ABI)
    cli

    # Copy hvm_start_info from %ebx to safe location at 0x90000
    # The page table setup below writes to 0x1000-0x3FFF, which would
    # overwrite the hvm_start_info if it's in that range.
    movl    $0x90000, %edi       # destination
    movl    %ebx, %esi           # source (hvm_start_info address)
    movl    $16, %ecx            # 16 dwords = 64 bytes
    rep     movsl                # copy

    # Copy memory map entries from memmap_paddr to 0x90040
    # The memmap entries are also in the 0x1000-0x3FFF range and will be
    # overwritten by page table setup. Copy them to 0x90040, right after
    # the hvm_start_info copy at 0x90000.
    # memmap_paddr is at offset 0x28 in hvm_start_info (8 bytes).
    # memmap_entries is at offset 0x30 (4 bytes).
    # Each HvmMemmapEntry is 24 bytes: addr(8) + size(8) + type(4) + reserved(4).
    movl    0x28(%ebx), %eax     # low 32 bits of memmap_paddr
    movl    0x30(%ebx), %ecx     # memmap_entries count
    testl   %ecx, %ecx
    jz      .Lskip_memmap_copy
    # Calculate total bytes: entries * 24 = entries * (16 + 8)
    movl    %ecx, %edx
    shll    $4, %ecx             # entries * 16
    shll    $3, %edx             # entries * 8
    addl    %edx, %ecx           # entries * 24 = total bytes
    movl    %eax, %esi           # source = memmap_paddr
    movl    $0x90040, %edi       # destination
    rep     movsb                # copy memmap entries

    # Update memmap_paddr in the copied hvm_start_info at 0x90000
    movl    $0x90040, 0x28 + 0x90000

.Lskip_memmap_copy:
    movl    $0x90000, %esi       # update %esi to point to the copy

    # Fall through to common boot path (same page table + GDT setup)
    # %esi now contains the safe copy address (0x90000)

/* ── Common 32-bit boot path ───────────────────────────────────────────
 *
 * Shared between Multiboot2 and PVH entry points.
 * On entry:
 *   %esi = boot info pointer (Multiboot2 info or hvm_start_info)
 *   %esp = valid stack
 *   interrupts disabled
 */
.Lcommon_boot:
    # Enable A20 gate
    inb     $0x92, %al
    orb     $2, %al
    outb    %al, $0x92

    # Load page table at 0x1000 (identity map first 2 MiB)
    movl    $0x1000, %edi
    movl    %edi, %cr3

    # Clear 12 pages (PML4 + PDP + PD = 3 × 4 KiB)
    xorl    %eax, %eax
    movl    $12288, %ecx
    rep     stosb

    # PML4[0] -> PDP at 0x2000
    movl    $0x1000, %edi
    movl    $0x2003, (%edi)

    # PDP[0] -> PD at 0x3000
    movl    $0x2000, %edi
    movl    $0x3003, (%edi)

    # PD[0] -> 2 MiB huge page (0x00000083) — covers 0x000000-0x1FFFFF
    movl    $0x3000, %edi
    movl    $0x00000083, (%edi)

    # PD[1] -> 2 MiB huge page (0x00200083) — covers 0x200000-0x3FFFFF (VGA buffer at 0xB8000)
    movl    $0x3000, %edi
    movl    $0x00200083, 8(%edi)

    # Enable PAE
    movl    %cr4, %eax
    orl     $0x20, %eax
    movl    %eax, %cr4

    # Build GDT at a fixed location (0x800, below stack at 0x80000)
    # This avoids linker moving .quad constants to .rodata
    movl    $0x800, %edi
    movl    $0, (%edi)              # null descriptor (bytes 0-3)
    movl    $0, 4(%edi)             # null descriptor (bytes 4-7)
    movl    $0x00000000, 8(%edi)    # code descriptor (bytes 0-3): 0x00000000
    movl    $0x00209800, 12(%edi)   # code descriptor (bytes 4-7): 0x00209800
    movl    $0x00000000, 16(%edi)   # data descriptor (bytes 0-3): 0x00000000
    movl    $0x00209200, 20(%edi)   # data descriptor (bytes 4-7): 0x00209200

    # Load GDT descriptor pointing to 0x800
    subl    $8, %esp                # make room on stack for GDTR (6 bytes)
    movw    $23, 0(%esp)            # limit = 24 - 1 = 23
    movl    $0x800, 2(%esp)         # base = 0x800
    lgdt    (%esp)
    addl    $8, %esp                # restore stack

    # Enable long mode
    movl    $0xC0000080, %ecx
    rdmsr
    orl     $0x100, %eax
    wrmsr

    # Enable paging
    movl    %cr0, %eax
    orl     $0x80000001, %eax
    movl    %eax, %cr0

    # Far jump to 64-bit code
    pushl   $0x08
    pushl   $_start_64
    lret

.Lfail:
    cli
    hlt
    jmp     .Lfail

.section .text.boot64, "ax"
.code64
.globl _start_64
_start_64:
    # Set up segment registers
    movw    $0x10, %ax
    movw    %ax, %ds
    movw    %ax, %es
    movw    %ax, %fs
    movw    %ax, %gs
    movw    %ax, %ss

    # Set up stack at 0x80000 (512 KiB stack, grows down to 0x00000)
    movq    $0x80000, %rsp

    # Pass boot info pointer (saved in esi) as first argument (rdi)
    # For Multiboot2: esi = Multiboot2 info structure address
    # For PVH: esi = hvm_start_info structure address
    # mov %esi, %edi zero-extends esi into rdi (x86_64 implicit zero-extend)
    mov     %esi, %edi

    # Call Rust entry with boot_info_addr in rdi
    call    _start_rust

    # Should never return
    cli
    hlt
    jmp     .
