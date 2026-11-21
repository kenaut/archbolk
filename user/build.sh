#!/bin/sh
# Build the external user program as a static ELF64 EXEC at the fixed slot 7 base.
set -eu
cd "$(dirname "$0")"
cc -nostdlib -static -no-pie -Wl,-T,linker.ld -o elf_hello.elf elf_hello.S
cp elf_hello.elf ../initrd/elf_hello.elf
