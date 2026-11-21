use crate::serial_print;
use crate::serial_println;

/// Maximum command line length.
const LINE_BUF_SIZE: usize = 256;
/// Maximum number of commands in history.
const HIST_SIZE: usize = 16;

/// Interactive serial shell.
pub struct Shell {
    /// Current line buffer.
    line: [u8; LINE_BUF_SIZE],
    /// Number of bytes in the line buffer.
    len: usize,
    /// Command history (ring buffer).
    history: [[u8; LINE_BUF_SIZE]; HIST_SIZE],
    /// Next history slot to write.
    hist_next: usize,
    /// Number of stored history entries (capped at HIST_SIZE).
    hist_count: usize,
    /// Logical index being browsed (0..=hist_count; hist_count = draft/empty).
    hist_browse: usize,
    /// Saved draft while browsing history.
    draft: [u8; LINE_BUF_SIZE],
    draft_len: usize,
    /// ANSI escape sequence state: 0 = normal, 1 = ESC, 2 = ESC '['.
    esc_state: u8,
}

impl Shell {
    pub const fn new() -> Self {
        Shell {
            line: [0; LINE_BUF_SIZE],
            len: 0,
            history: [[0; LINE_BUF_SIZE]; HIST_SIZE],
            hist_next: 0,
            hist_count: 0,
            hist_browse: 0,
            draft: [0; LINE_BUF_SIZE],
            draft_len: 0,
            esc_state: 0,
        }
    }

    /// Replace the current line contents and redraw it on the serial console.
    fn replace_line(&mut self, s: &[u8]) {
        for _ in 0..self.len {
            serial_print!("\x08 \x08");
        }
        self.len = s.len().min(LINE_BUF_SIZE - 1);
        self.line[..self.len].copy_from_slice(&s[..self.len]);
        for &b in &self.line[..self.len] {
            serial_print!("{}", b as char);
        }
    }

    /// Browse to the previous (older) command in history.
    fn history_up(&mut self) {
        if self.hist_count == 0 {
            return;
        }
        if self.hist_browse == self.hist_count {
            self.draft[..self.len].copy_from_slice(&self.line[..self.len]);
            self.draft_len = self.len;
        }
        if self.hist_browse > 0 {
            self.hist_browse -= 1;
            let idx = (self.hist_next + HIST_SIZE - self.hist_count + self.hist_browse) % HIST_SIZE;
            let cmd = self.history[idx];
            let cmd_len = cmd.iter().position(|&b| b == 0).unwrap_or(LINE_BUF_SIZE);
            self.replace_line(&cmd[..cmd_len]);
        }
    }

    /// Browse to the next (newer) command in history, or restore the draft.
    fn history_down(&mut self) {
        if self.hist_browse >= self.hist_count {
            return;
        }
        self.hist_browse += 1;
        if self.hist_browse == self.hist_count {
            let len = self.draft_len;
            let draft = self.draft;
            self.replace_line(&draft[..len]);
        } else {
            let idx = (self.hist_next + HIST_SIZE - self.hist_count + self.hist_browse) % HIST_SIZE;
            let cmd = self.history[idx];
            let cmd_len = cmd.iter().position(|&b| b == 0).unwrap_or(LINE_BUF_SIZE);
            self.replace_line(&cmd[..cmd_len]);
        }
    }

    /// Print the shell prompt.
    pub fn print_prompt(&self) {
        serial_print!("$ ");
    }

    /// Handle a single keypress from the keyboard.
    pub fn handle_key(&mut self, ch: u8) {
        // ANSI escape sequences (arrow keys) arrive as ESC '[' A/B.
        // Handle them before lowercasing so 'A'/'B' survive.
        if self.esc_state != 0 {
            match (self.esc_state, ch) {
                (1, b'[') => {
                    self.esc_state = 2;
                    return;
                }
                (2, b'A') => {
                    self.esc_state = 0;
                    self.history_up();
                    return;
                }
                (2, b'B') => {
                    self.esc_state = 0;
                    self.history_down();
                    return;
                }
                _ => {
                    self.esc_state = 0;
                }
            }
        }
        if ch == 0x1B {
            self.esc_state = 1;
            return;
        }

        // Normalize to lowercase for command matching
        let ch = if ch >= b'A' && ch <= b'Z' { ch + 32 } else { ch };
        match ch {
            b'\n' | b'\r' => {
                // Execute command
                serial_print!("\n");
                // Copy command string to avoid borrow conflict with self.execute()
                let cmd_str = {
                    let s = core::str::from_utf8(&self.line[..self.len]).unwrap_or("");
                    let mut buf = [0u8; LINE_BUF_SIZE];
                    let len = s.len().min(LINE_BUF_SIZE - 1);
                    buf[..len].copy_from_slice(s.as_bytes());
                    buf
                };
                let cmd_len = core::str::from_utf8(&self.line[..self.len]).unwrap_or("").len();
                let cmd = core::str::from_utf8(&cmd_str[..cmd_len]).unwrap_or("");
                self.execute(cmd);
                // Save to history (don't save empty or duplicate of last)
                if self.len > 0 {
                    let mut duplicate = false;
                    if self.hist_count > 0 {
                        let last_idx = (self.hist_next + HIST_SIZE - 1) % HIST_SIZE;
                        duplicate = self.history[last_idx][..self.len] == self.line[..self.len];
                    }
                    if !duplicate {
                        for i in 0..LINE_BUF_SIZE {
                            self.history[self.hist_next][i] = 0;
                        }
                        self.history[self.hist_next][..self.len]
                            .copy_from_slice(&self.line[..self.len]);
                        self.hist_next = (self.hist_next + 1) % HIST_SIZE;
                        self.hist_count = (self.hist_count + 1).min(HIST_SIZE);
                    }
                }
                self.len = 0;
                self.hist_browse = self.hist_count;
                self.draft_len = 0;
                self.print_prompt();
            }
            b'\x08' | 0x7F => {
                // Backspace
                if self.len > 0 {
                    self.len -= 1;
                    serial_print!("\x08 \x08");
                }
            }
            0x09 => {
                // Tab — print available commands (simple)
                serial_print!("\n");
                self.list_commands();
                self.print_prompt();
                let _ = core::fmt::Write::write_str(
                    &mut *crate::serial::SERIAL.lock(),
                    core::str::from_utf8(&self.line[..self.len]).unwrap_or(""),
                );
                crate::serial::SERIAL.unlock();
            }
            _ => {
                // Printable character
                if self.len < LINE_BUF_SIZE - 1 && ch >= 0x20 && ch <= 0x7E {
                    self.line[self.len] = ch;
                    self.len += 1;
                    serial_print!("{}", ch as char);
                }
            }
        }
    }

    /// Execute a command line.
    fn execute(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }

        // Split into command and args
        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let args: core::str::SplitWhitespace = parts;

        match cmd {
            "help" => self.cmd_help(),
            "echo" => self.cmd_echo(args),
            "clear" => self.cmd_clear(),
            "uptime" => self.cmd_uptime(),
            "info" => self.cmd_info(),
            "reboot" => self.cmd_reboot(),
            "ticks" => self.cmd_ticks(),
            "mem" => self.cmd_mem(),
            "history" => self.cmd_history(),
            "syscall" => self.cmd_syscall(),
            "ps" => self.cmd_ps(),
            "kill" => self.cmd_kill(args),
            "spawn" => self.cmd_spawn(),
            "ls" => self.cmd_ls(),
            "cat" => self.cmd_cat(args),
            _ => {
                serial_println!("shell: unknown command '{}'", cmd);
                serial_println!("Try 'help'.");
            }
        }
    }

    fn list_commands(&self) {
        serial_println!("available commands:");
        serial_println!("  help     — show this help");
        serial_println!("  echo     — echo arguments");
        serial_println!("  clear    — clear screen (VGA)");
        serial_println!("  uptime   — show system uptime");
        serial_println!("  info     — show system info");
        serial_println!("  reboot   — reboot the system");
        serial_println!("  ticks    — show PIT tick count");
        serial_println!("  mem      — show memory info");
        serial_println!("  history  — show command history");
        serial_println!("  syscall  — test syscall mechanism");
        serial_println!("  ps       — list tasks");
        serial_println!("  kill <id>— terminate a task");
        serial_println!("  spawn    — create a test task");
        serial_println!("  ls       — list initrd files");
        serial_println!("  cat <f>  — print a file from initrd");
    }

    fn cmd_help(&self) {
        serial_println!("archbolk shell — built-in commands:");
        self.list_commands();
    }

    fn cmd_echo(&self, args: core::str::SplitWhitespace) {
        for arg in args {
            serial_print!("{} ", arg);
        }
        serial_print!("\n");
    }

    fn cmd_clear(&self) {
        // VGA clear: fill screen with spaces
        use core::ptr::write_volatile;
        let buf = 0xb8000 as *mut u16;
        for i in 0..80 * 25 {
            unsafe { write_volatile(buf.add(i), 0x0720); }
        }
        // Reset column position via the writer
        {
            let w = crate::vga_buffer::WRITER.lock();
            w.column_position = 0;
            crate::vga_buffer::WRITER.unlock();
        }
    }

    fn cmd_uptime(&self) {
        let ticks = crate::interrupts::get_tick_count();
        let secs = ticks / 100;
        let ms = (ticks % 100) * 10;
        serial_println!("uptime: {}.{:03}s", secs, ms);
    }

    fn cmd_info(&self) {
        serial_println!("archbolk kernel v0.1.0");
        serial_println!("arch: x86_64, freestanding");
        serial_println!("boot: Multiboot2 via GRUB2");
        serial_println!("heap: bump allocator at 0x4444_0000_0000");
        serial_println!("PIT: ~100 Hz");
        serial_println!("keyboard: PS/2, Set 1 scancodes");
    }

    fn cmd_reboot(&self) {
        serial_println!("Rebooting...");
        // Use the PS/2 controller reset method (triple fault)
        unsafe {
            // Try keyboard controller reset
            core::arch::asm!("mov dx, 0x64; mov al, 0xFE; out dx, al");
            // If that doesn't work, cause a triple fault via IDT
            core::arch::asm!("lidt [0]");
            core::arch::asm!("int 3");
        }
        loop {}
    }

    fn cmd_ticks(&self) {
        let ticks = crate::interrupts::get_tick_count();
        serial_println!("PIT ticks: {}", ticks);
    }

    fn cmd_mem(&self) {
        let frames = crate::memory::pfa::frame_count();
        serial_println!("free frames: {} ({} KiB)", frames, frames * 4);
    }

    fn cmd_history(&self) {
        serial_println!("command history:");
        for i in 0..HIST_SIZE {
            let idx = (self.hist_next + HIST_SIZE - 1 - i) % HIST_SIZE;
            let entry = self.history[idx];
            if entry[0] != 0 {
                let s = core::str::from_utf8(&entry).unwrap_or("");
                let s = s.trim_end_matches('\0');
                if !s.is_empty() {
                    serial_println!("  {}: {}", i + 1, s);
                }
            }
        }
    }

    fn cmd_syscall(&self) {
        serial_println!("syscall test:");
        serial_println!("  Calling sys_getpid (nr=39)...");

        let pid: u64;
        unsafe {
            core::arch::asm!(
                "movq ${0}, %rax",
                "syscall",
                const 39u64,
                out("rax") pid,
                out("rcx") _,
                out("r11") _,
                options(att_syntax),
            );
        }
        serial_println!("  getpid() = {}", pid);

        serial_println!("  Calling sys_write (nr=1, fd=1, buf='Hello from syscall!\\n')...");
        let msg = b"Hello from syscall!\n";
        let ret: u64;
        unsafe {
            core::arch::asm!(
                "movq ${0}, %rax",
                "movq ${1}, %rdi",
                "syscall",
                const 1u64,
                const 1u64,
                in("rsi") msg.as_ptr(),
                in("rdx") msg.len(),
                out("rax") ret,
                out("rcx") _,
                out("r11") _,
                options(att_syntax),
            );
        }
        serial_println!("  write() returned {}", ret);

        serial_println!("  Calling unknown syscall (nr=999)...");
        let unknown_ret: u64;
        unsafe {
            core::arch::asm!(
                "movq ${0}, %rax",
                "syscall",
                const 999u64,
                out("rax") unknown_ret,
                out("rcx") _,
                out("r11") _,
                options(att_syntax),
            );
        }
        serial_println!("  unknown syscall returned {}", unknown_ret as isize);
    }

    fn cmd_ps(&self) {
        use crate::scheduler::TaskInfo;
        let mut infos = [TaskInfo {
            id: 0,
            name: "",
            state: crate::scheduler::task::TaskState::Ready,
            terminated: false,
            rsp: 0,
        }; 16];
        let sched = crate::scheduler::SCHEDULER.lock();
        let n = sched.list(&mut infos);
        serial_println!("  ID  NAME        STATE      RSP");
        for info in infos.iter().take(n) {
            let state = match info.state {
                crate::scheduler::task::TaskState::Ready => "Ready",
                crate::scheduler::task::TaskState::Running => "Running",
                crate::scheduler::task::TaskState::Blocked => "Blocked",
            };
            let marker = if info.id == sched.current_id() { "*" } else { " " };
            let term = if info.terminated { " (dead)" } else { "" };
            serial_println!(
                "{} {:>2}  {:<10} {:<9} 0x{:x}{}",
                marker, info.id, info.name, state, info.rsp, term
            );
        }
        drop(sched);
    }

    fn cmd_kill(&self, mut args: core::str::SplitWhitespace) {
        let Some(id_str) = args.next() else {
            serial_println!("usage: kill <id>");
            return;
        };
        let Ok(id) = id_str.parse::<usize>() else {
            serial_println!("kill: invalid id '{}'", id_str);
            return;
        };
        let ok = crate::scheduler::SCHEDULER.lock().kill(id);
        if ok {
            serial_println!("kill: task {} terminated", id);
        } else {
            serial_println!("kill: cannot terminate task {}", id);
        }
    }

    fn cmd_spawn(&self) {
        let name = "worker";
        let id = crate::scheduler::spawn_dynamic(name, crate::spawned_task);
        match id {
            Some(id) => serial_println!("spawn: created task {} ('{}')", id, name),
            None => serial_println!("spawn: no stacks or task slots available"),
        }
    }

    fn cmd_ls(&self) {
        crate::fs::tarfs::list();
    }

    fn cmd_cat(&self, mut args: core::str::SplitWhitespace) {
        let name = match args.next() {
            Some(n) => n,
            None => {
                serial_println!("usage: cat <filename>");
                return;
            }
        };
        match crate::fs::tarfs::read(name) {
            Some(data) => {
                let s = core::str::from_utf8(data).unwrap_or("<binary data>");
                serial_print!("{}", s);
                if !s.ends_with('\n') {
                    serial_print!("\n");
                }
            }
            None => serial_println!("cat: {}: not found", name),
        }
    }
}
