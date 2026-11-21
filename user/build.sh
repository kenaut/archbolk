#!/bin/sh
# Build external user programs as static ELF64 EXECs at the common link base
# (0x1000), then copy them into the initrd.
set -eu
cd "$(dirname "$0")"

# Assembly demo.
cc -nostdlib -static -no-pie -Wl,-T,linker.ld -o elf_hello.elf elf_hello.S
cp elf_hello.elf ../initrd/elf_hello.elf

# Rust freestanding program.
cd rust_app
. "$HOME/.cargo/env"
CARGO_ENCODED_RUSTFLAGS="$(printf -- '-C\x1flink-arg=-T../linker.ld\x1f-C\x1flink-arg=-static\x1f-C\x1frelocation-model=pic')" \
    cargo +nightly build --release 2>/dev/null
cp target/x86_64-unknown-none/release/archbolk-user ../../initrd/rust_hello.elf
