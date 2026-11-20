fn main() {
    println!("cargo:rerun-if-changed=src/arch/x86_64/boot.S");
    println!("cargo:rerun-if-changed=src/interrupts/interrupt.S");
    println!("cargo:rerun-if-changed=src/scheduler/switch.S");
    cc::Build::new()
        .file("src/arch/x86_64/boot.S")
        .file("src/interrupts/interrupt.S")
        .file("src/scheduler/switch.S")
        .compile("boot");
}
