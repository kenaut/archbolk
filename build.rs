fn main() {
    cc::Build::new()
        .file("src/arch/x86_64/boot.S")
        .file("src/interrupts/interrupt.S")
        .compile("boot");
}
