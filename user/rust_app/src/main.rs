#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

const SYS_WRITE: u64 = 1;
const SYS_EXIT: u64 = 60;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    exit(1);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let msg = b"Hello from a Rust user program!\n";
    sys_write(1, msg.as_ptr(), msg.len());
    exit(0);
}

fn sys_write(fd: usize, buf: *const u8, len: usize) {
    let ret: i64;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") SYS_WRITE as i64 => ret,
            in("rdi") fd,
            in("rsi") buf,
            in("rdx") len,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    let _ = ret;
}

fn exit(code: i32) -> ! {
    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_EXIT as i64,
            in("rdi") code as i64,
            options(noreturn),
        );
    }
}
