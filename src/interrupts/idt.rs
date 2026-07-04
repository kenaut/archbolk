use core::arch::asm;
use alloc::boxed::Box;

/// An IDT entry (16 bytes, x86_64 format).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct IdtEntry {
    /// Bits 0-15 of the handler offset.
    offset_low: u16,
    /// Code segment selector (kernel code = 0x08).
    selector: u16,
    /// Interrupt Stack Table (bits 0-2), reserved (bits 3-7).
    ist: u8,
    /// Gate type, DPL, present bit.
    flags: u8,
    /// Bits 16-31 of the handler offset.
    offset_mid: u16,
    /// Bits 32-63 of the handler offset.
    offset_high: u32,
    /// Reserved (must be 0).
    reserved: u32,
}

impl IdtEntry {
    /// Create a new IDT entry pointing to the given handler address.
    pub const fn new(handler: u64) -> Self {
        IdtEntry {
            offset_low: (handler & 0xFFFF) as u16,
            selector: 0x08, // kernel code segment
            ist: 0,
            // 0x8E = present, DPL=0, interrupt gate (type=1110)
            flags: 0x8E,
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }
}

/// The Interrupt Descriptor Table: 256 entries.
#[repr(C, align(16))]
pub struct Idt {
    entries: [IdtEntry; 256],
}

impl Idt {
    pub const fn new() -> Self {
        Idt {
            entries: [IdtEntry::new(0); 256],
        }
    }

    pub fn set_entry(&mut self, index: u8, handler: u64) {
        self.entries[index as usize] = IdtEntry::new(handler);
    }
}

/// 6-byte descriptor table pointer for `lidt`.
#[repr(C, packed)]
struct DescriptorTablePointer {
    /// Size of the IDT in bytes minus 1.
    limit: u16,
    /// Linear address of the IDT.
    base: u64,
}

/// Initialize the IDT: set all 256 entries to point to the assembly stubs,
/// then load the IDT with `lidt`.
pub fn init() {
    // IDT must live forever — use a leaked box to put it on the heap.
    let idt = Box::leak(Box::new(Idt::new()));

    // Fill all 256 entries with the address of each stub.
    // The stubs are exported as `isr_stub_0`, `isr_stub_1`, etc.
    unsafe extern "C" {
        fn isr_stub_0();
        fn isr_stub_1();
        fn isr_stub_2();
        fn isr_stub_3();
        fn isr_stub_4();
        fn isr_stub_5();
        fn isr_stub_6();
        fn isr_stub_7();
        fn isr_stub_8();
        fn isr_stub_9();
        fn isr_stub_10();
        fn isr_stub_11();
        fn isr_stub_12();
        fn isr_stub_13();
        fn isr_stub_14();
        fn isr_stub_15();
        fn isr_stub_16();
        fn isr_stub_17();
        fn isr_stub_18();
        fn isr_stub_19();
        fn isr_stub_20();
        fn isr_stub_21();
        fn isr_stub_22();
        fn isr_stub_23();
        fn isr_stub_24();
        fn isr_stub_25();
        fn isr_stub_26();
        fn isr_stub_27();
        fn isr_stub_28();
        fn isr_stub_29();
        fn isr_stub_30();
        fn isr_stub_31();
        fn isr_stub_32();
        fn isr_stub_33();
        fn isr_stub_34();
        fn isr_stub_35();
        fn isr_stub_36();
        fn isr_stub_37();
        fn isr_stub_38();
        fn isr_stub_39();
        fn isr_stub_40();
        fn isr_stub_41();
        fn isr_stub_42();
        fn isr_stub_43();
        fn isr_stub_44();
        fn isr_stub_45();
        fn isr_stub_46();
        fn isr_stub_47();
        fn isr_stub_48();
        fn isr_stub_49();
        fn isr_stub_50();
        fn isr_stub_51();
        fn isr_stub_52();
        fn isr_stub_53();
        fn isr_stub_54();
        fn isr_stub_55();
        fn isr_stub_56();
        fn isr_stub_57();
        fn isr_stub_58();
        fn isr_stub_59();
        fn isr_stub_60();
        fn isr_stub_61();
        fn isr_stub_62();
        fn isr_stub_63();
        fn isr_stub_64();
        fn isr_stub_65();
        fn isr_stub_66();
        fn isr_stub_67();
        fn isr_stub_68();
        fn isr_stub_69();
        fn isr_stub_70();
        fn isr_stub_71();
        fn isr_stub_72();
        fn isr_stub_73();
        fn isr_stub_74();
        fn isr_stub_75();
        fn isr_stub_76();
        fn isr_stub_77();
        fn isr_stub_78();
        fn isr_stub_79();
        fn isr_stub_80();
        fn isr_stub_81();
        fn isr_stub_82();
        fn isr_stub_83();
        fn isr_stub_84();
        fn isr_stub_85();
        fn isr_stub_86();
        fn isr_stub_87();
        fn isr_stub_88();
        fn isr_stub_89();
        fn isr_stub_90();
        fn isr_stub_91();
        fn isr_stub_92();
        fn isr_stub_93();
        fn isr_stub_94();
        fn isr_stub_95();
        fn isr_stub_96();
        fn isr_stub_97();
        fn isr_stub_98();
        fn isr_stub_99();
        fn isr_stub_100();
        fn isr_stub_101();
        fn isr_stub_102();
        fn isr_stub_103();
        fn isr_stub_104();
        fn isr_stub_105();
        fn isr_stub_106();
        fn isr_stub_107();
        fn isr_stub_108();
        fn isr_stub_109();
        fn isr_stub_110();
        fn isr_stub_111();
        fn isr_stub_112();
        fn isr_stub_113();
        fn isr_stub_114();
        fn isr_stub_115();
        fn isr_stub_116();
        fn isr_stub_117();
        fn isr_stub_118();
        fn isr_stub_119();
        fn isr_stub_120();
        fn isr_stub_121();
        fn isr_stub_122();
        fn isr_stub_123();
        fn isr_stub_124();
        fn isr_stub_125();
        fn isr_stub_126();
        fn isr_stub_127();
        fn isr_stub_128();
        fn isr_stub_129();
        fn isr_stub_130();
        fn isr_stub_131();
        fn isr_stub_132();
        fn isr_stub_133();
        fn isr_stub_134();
        fn isr_stub_135();
        fn isr_stub_136();
        fn isr_stub_137();
        fn isr_stub_138();
        fn isr_stub_139();
        fn isr_stub_140();
        fn isr_stub_141();
        fn isr_stub_142();
        fn isr_stub_143();
        fn isr_stub_144();
        fn isr_stub_145();
        fn isr_stub_146();
        fn isr_stub_147();
        fn isr_stub_148();
        fn isr_stub_149();
        fn isr_stub_150();
        fn isr_stub_151();
        fn isr_stub_152();
        fn isr_stub_153();
        fn isr_stub_154();
        fn isr_stub_155();
        fn isr_stub_156();
        fn isr_stub_157();
        fn isr_stub_158();
        fn isr_stub_159();
        fn isr_stub_160();
        fn isr_stub_161();
        fn isr_stub_162();
        fn isr_stub_163();
        fn isr_stub_164();
        fn isr_stub_165();
        fn isr_stub_166();
        fn isr_stub_167();
        fn isr_stub_168();
        fn isr_stub_169();
        fn isr_stub_170();
        fn isr_stub_171();
        fn isr_stub_172();
        fn isr_stub_173();
        fn isr_stub_174();
        fn isr_stub_175();
        fn isr_stub_176();
        fn isr_stub_177();
        fn isr_stub_178();
        fn isr_stub_179();
        fn isr_stub_180();
        fn isr_stub_181();
        fn isr_stub_182();
        fn isr_stub_183();
        fn isr_stub_184();
        fn isr_stub_185();
        fn isr_stub_186();
        fn isr_stub_187();
        fn isr_stub_188();
        fn isr_stub_189();
        fn isr_stub_190();
        fn isr_stub_191();
        fn isr_stub_192();
        fn isr_stub_193();
        fn isr_stub_194();
        fn isr_stub_195();
        fn isr_stub_196();
        fn isr_stub_197();
        fn isr_stub_198();
        fn isr_stub_199();
        fn isr_stub_200();
        fn isr_stub_201();
        fn isr_stub_202();
        fn isr_stub_203();
        fn isr_stub_204();
        fn isr_stub_205();
        fn isr_stub_206();
        fn isr_stub_207();
        fn isr_stub_208();
        fn isr_stub_209();
        fn isr_stub_210();
        fn isr_stub_211();
        fn isr_stub_212();
        fn isr_stub_213();
        fn isr_stub_214();
        fn isr_stub_215();
        fn isr_stub_216();
        fn isr_stub_217();
        fn isr_stub_218();
        fn isr_stub_219();
        fn isr_stub_220();
        fn isr_stub_221();
        fn isr_stub_222();
        fn isr_stub_223();
        fn isr_stub_224();
        fn isr_stub_225();
        fn isr_stub_226();
        fn isr_stub_227();
        fn isr_stub_228();
        fn isr_stub_229();
        fn isr_stub_230();
        fn isr_stub_231();
        fn isr_stub_232();
        fn isr_stub_233();
        fn isr_stub_234();
        fn isr_stub_235();
        fn isr_stub_236();
        fn isr_stub_237();
        fn isr_stub_238();
        fn isr_stub_239();
        fn isr_stub_240();
        fn isr_stub_241();
        fn isr_stub_242();
        fn isr_stub_243();
        fn isr_stub_244();
        fn isr_stub_245();
        fn isr_stub_246();
        fn isr_stub_247();
        fn isr_stub_248();
        fn isr_stub_249();
        fn isr_stub_250();
        fn isr_stub_251();
        fn isr_stub_252();
        fn isr_stub_253();
        fn isr_stub_254();
        fn isr_stub_255();
    }

    let stubs: [unsafe extern "C" fn(); 256] = [
        isr_stub_0, isr_stub_1, isr_stub_2, isr_stub_3, isr_stub_4, isr_stub_5, isr_stub_6,
        isr_stub_7, isr_stub_8, isr_stub_9, isr_stub_10, isr_stub_11, isr_stub_12, isr_stub_13,
        isr_stub_14, isr_stub_15, isr_stub_16, isr_stub_17, isr_stub_18, isr_stub_19, isr_stub_20,
        isr_stub_21, isr_stub_22, isr_stub_23, isr_stub_24, isr_stub_25, isr_stub_26, isr_stub_27,
        isr_stub_28, isr_stub_29, isr_stub_30, isr_stub_31, isr_stub_32, isr_stub_33, isr_stub_34,
        isr_stub_35, isr_stub_36, isr_stub_37, isr_stub_38, isr_stub_39, isr_stub_40, isr_stub_41,
        isr_stub_42, isr_stub_43, isr_stub_44, isr_stub_45, isr_stub_46, isr_stub_47, isr_stub_48,
        isr_stub_49, isr_stub_50, isr_stub_51, isr_stub_52, isr_stub_53, isr_stub_54, isr_stub_55,
        isr_stub_56, isr_stub_57, isr_stub_58, isr_stub_59, isr_stub_60, isr_stub_61, isr_stub_62,
        isr_stub_63, isr_stub_64, isr_stub_65, isr_stub_66, isr_stub_67, isr_stub_68, isr_stub_69,
        isr_stub_70, isr_stub_71, isr_stub_72, isr_stub_73, isr_stub_74, isr_stub_75, isr_stub_76,
        isr_stub_77, isr_stub_78, isr_stub_79, isr_stub_80, isr_stub_81, isr_stub_82, isr_stub_83,
        isr_stub_84, isr_stub_85, isr_stub_86, isr_stub_87, isr_stub_88, isr_stub_89, isr_stub_90,
        isr_stub_91, isr_stub_92, isr_stub_93, isr_stub_94, isr_stub_95, isr_stub_96, isr_stub_97,
        isr_stub_98, isr_stub_99, isr_stub_100, isr_stub_101, isr_stub_102, isr_stub_103,
        isr_stub_104, isr_stub_105, isr_stub_106, isr_stub_107, isr_stub_108, isr_stub_109,
        isr_stub_110, isr_stub_111, isr_stub_112, isr_stub_113, isr_stub_114, isr_stub_115,
        isr_stub_116, isr_stub_117, isr_stub_118, isr_stub_119, isr_stub_120, isr_stub_121,
        isr_stub_122, isr_stub_123, isr_stub_124, isr_stub_125, isr_stub_126, isr_stub_127,
        isr_stub_128, isr_stub_129, isr_stub_130, isr_stub_131, isr_stub_132, isr_stub_133,
        isr_stub_134, isr_stub_135, isr_stub_136, isr_stub_137, isr_stub_138, isr_stub_139,
        isr_stub_140, isr_stub_141, isr_stub_142, isr_stub_143, isr_stub_144, isr_stub_145,
        isr_stub_146, isr_stub_147, isr_stub_148, isr_stub_149, isr_stub_150, isr_stub_151,
        isr_stub_152, isr_stub_153, isr_stub_154, isr_stub_155, isr_stub_156, isr_stub_157,
        isr_stub_158, isr_stub_159, isr_stub_160, isr_stub_161, isr_stub_162, isr_stub_163,
        isr_stub_164, isr_stub_165, isr_stub_166, isr_stub_167, isr_stub_168, isr_stub_169,
        isr_stub_170, isr_stub_171, isr_stub_172, isr_stub_173, isr_stub_174, isr_stub_175,
        isr_stub_176, isr_stub_177, isr_stub_178, isr_stub_179, isr_stub_180, isr_stub_181,
        isr_stub_182, isr_stub_183, isr_stub_184, isr_stub_185, isr_stub_186, isr_stub_187,
        isr_stub_188, isr_stub_189, isr_stub_190, isr_stub_191, isr_stub_192, isr_stub_193,
        isr_stub_194, isr_stub_195, isr_stub_196, isr_stub_197, isr_stub_198, isr_stub_199,
        isr_stub_200, isr_stub_201, isr_stub_202, isr_stub_203, isr_stub_204, isr_stub_205,
        isr_stub_206, isr_stub_207, isr_stub_208, isr_stub_209, isr_stub_210, isr_stub_211,
        isr_stub_212, isr_stub_213, isr_stub_214, isr_stub_215, isr_stub_216, isr_stub_217,
        isr_stub_218, isr_stub_219, isr_stub_220, isr_stub_221, isr_stub_222, isr_stub_223,
        isr_stub_224, isr_stub_225, isr_stub_226, isr_stub_227, isr_stub_228, isr_stub_229,
        isr_stub_230, isr_stub_231, isr_stub_232, isr_stub_233, isr_stub_234, isr_stub_235,
        isr_stub_236, isr_stub_237, isr_stub_238, isr_stub_239, isr_stub_240, isr_stub_241,
        isr_stub_242, isr_stub_243, isr_stub_244, isr_stub_245, isr_stub_246, isr_stub_247,
        isr_stub_248, isr_stub_249, isr_stub_250, isr_stub_251, isr_stub_252, isr_stub_253,
        isr_stub_254, isr_stub_255,
    ];

    for (i, stub) in stubs.iter().enumerate() {
        idt.set_entry(i as u8, *stub as u64);
    }

    // Load the IDT
    let ptr = DescriptorTablePointer {
        limit: (core::mem::size_of::<Idt>() - 1) as u16,
        base: idt as *const Idt as u64,
    };

    unsafe {
        asm!("lidt [{}]", in(reg) &ptr, options(readonly, nostack, preserves_flags));
    }
}
