use std::io::{BufRead, Write};

pub type Cell = i16;

pub const SPAN_ADDR: usize = 1000;
pub const DP_ADDR: usize = 1002;
pub const BASE_ADDR: usize = 1004;
pub const INITIAL_DP: Cell = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForthError {
    pub message: String,
}

impl std::fmt::Display for ForthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ForthError {}

pub struct Forth<R: BufRead, W: Write> {
    pub ds: Vec<Cell>,
    pub rs: Vec<Cell>,
    pub memory: [u8; 65536],
    pub running: bool,
    pub state: bool,
    pub interrupted: bool,
    pub tib: String,
    pub in_ptr: usize,
    pub compile_list: Vec<Cell>,
    pub compiling_xt: isize,
    pub input: R,
    pub output: W,
}

impl<R: BufRead, W: Write> Forth<R, W> {
    pub fn new(input: R, output: W) -> Self {
        let mut f = Self {
            ds: Vec::new(),
            rs: Vec::new(),
            memory: [0; 65536],
            running: true,
            state: false,
            interrupted: false,
            tib: String::new(),
            in_ptr: 0,
            compile_list: Vec::new(),
            compiling_xt: -1,
            input,
            output,
        };
        f.store_cell(SPAN_ADDR as u32, 0).unwrap();
        f.store_cell(DP_ADDR as u32, INITIAL_DP).unwrap();
        f.store_cell(BASE_ADDR as u32, 10).unwrap();
        f.init_primitives();
        f.require_module("CORE");
        f
    }

    pub fn c_fetch(&self, addr: u32) -> Result<u8, ForthError> {
        if addr > 65535 {
            return Err(ForthError {
                message: "OUT OF MEMORY RANGE".to_string(),
            });
        }
        Ok(self.memory[addr as usize])
    }

    pub fn c_store(&mut self, addr: u32, val: u8) -> Result<(), ForthError> {
        if addr > 65535 {
            return Err(ForthError {
                message: "OUT OF MEMORY RANGE".to_string(),
            });
        }
        self.memory[addr as usize] = val;
        Ok(())
    }

    pub fn fetch_cell(&self, addr: u32) -> Result<Cell, ForthError> {
        if addr + 1 > 65535 {
            return Err(ForthError {
                message: "OUT OF MEMORY RANGE".to_string(),
            });
        }
        let low = self.memory[addr as usize] as u16;
        let high = self.memory[addr as usize + 1] as u16;
        Ok((low | (high << 8)) as Cell)
    }

    pub fn store_cell(&mut self, addr: u32, val: Cell) -> Result<(), ForthError> {
        if addr + 1 > 65535 {
            return Err(ForthError {
                message: "OUT OF MEMORY RANGE".to_string(),
            });
        }
        let u = val as u16;
        self.memory[addr as usize] = (u & 0xff) as u8;
        self.memory[addr as usize + 1] = ((u >> 8) & 0xff) as u8;
        Ok(())
    }

    fn init_primitives(&mut self) {
        // Kernel registration grows in later tasks.
    }

    fn require_module(&mut self, _name: &str) {
        // Module loading grows in later tasks.
    }

    pub fn push_ds(&mut self, v: Cell) {
        self.ds.push(v);
    }

    pub fn pop_ds(&mut self) -> Result<Cell, ForthError> {
        self.ds.pop().ok_or_else(|| ForthError {
            message: "DATA STACK UNDERFLOW".to_string(),
        })
    }

    pub fn peek_ds(&self) -> Result<Cell, ForthError> {
        self.ds.last().copied().ok_or_else(|| ForthError {
            message: "DATA STACK UNDERFLOW".to_string(),
        })
    }

    pub fn push_rs(&mut self, v: Cell) {
        self.rs.push(v);
    }

    pub fn pop_rs(&mut self) -> Result<Cell, ForthError> {
        self.rs.pop().ok_or_else(|| ForthError {
            message: "RETURN STACK UNDERFLOW".to_string(),
        })
    }

    pub fn peek_rs(&self) -> Result<Cell, ForthError> {
        self.rs.last().copied().ok_or_else(|| ForthError {
            message: "RETURN STACK UNDERFLOW".to_string(),
        })
    }

    pub fn depth(&mut self) -> Result<(), ForthError> {
        let n = self.ds.len() as Cell;
        self.push_ds(n);
        Ok(())
    }

    pub fn word_fetch(&mut self) -> Result<(), ForthError> {
        let addr = self.pop_ds()? as u16 as u32;
        let v = self.fetch_cell(addr)?;
        self.push_ds(v);
        Ok(())
    }

    pub fn word_store(&mut self) -> Result<(), ForthError> {
        let addr = self.pop_ds()? as u16 as u32;
        let n = self.pop_ds()?;
        self.store_cell(addr, n)
    }

    pub fn word_c_fetch(&mut self) -> Result<(), ForthError> {
        let addr = self.pop_ds()? as u16 as u32;
        let b = self.c_fetch(addr)?;
        self.push_ds(b as Cell);
        Ok(())
    }

    pub fn word_c_store(&mut self) -> Result<(), ForthError> {
        let addr = self.pop_ds()? as u16 as u32;
        let n = self.pop_ds()?;
        self.c_store(addr, (n as u16 & 0xff) as u8)
    }

    pub fn plus_store(&mut self) -> Result<(), ForthError> {
        let addr = self.pop_ds()? as u16 as u32;
        let n = self.pop_ds()?;
        let cur = self.fetch_cell(addr)?;
        self.store_cell(addr, cur.wrapping_add(n))
    }

    pub fn fill(&mut self) -> Result<(), ForthError> {
        let byte = self.pop_ds()?;
        let len = self.pop_ds()?;
        let addr = self.pop_ds()? as u16 as u32;
        let b = (byte as u16 & 0xff) as u8;
        if len <= 0 {
            return Ok(());
        }
        for i in 0..len as u32 {
            self.c_store(addr + i, b)?;
        }
        Ok(())
    }

    pub fn erase(&mut self) -> Result<(), ForthError> {
        let len = self.pop_ds()?;
        let addr = self.pop_ds()? as u16 as u32;
        if len <= 0 {
            return Ok(());
        }
        for i in 0..len as u32 {
            self.c_store(addr + i, 0)?;
        }
        Ok(())
    }

    pub fn cmove(&mut self) -> Result<(), ForthError> {
        let len = self.pop_ds()?;
        let src = self.pop_ds()? as u16 as u32;
        let dest = self.pop_ds()? as u16 as u32;
        if len <= 0 {
            return Ok(());
        }
        for i in 0..len as u32 {
            let b = self.c_fetch(src + i)?;
            self.c_store(dest + i, b)?;
        }
        Ok(())
    }

    pub fn cmove_up(&mut self) -> Result<(), ForthError> {
        let len = self.pop_ds()?;
        let src = self.pop_ds()? as u16 as u32;
        let dest = self.pop_ds()? as u16 as u32;
        if len <= 0 {
            return Ok(());
        }
        for i in (0..len as u32).rev() {
            let b = self.c_fetch(src + i)?;
            self.c_store(dest + i, b)?;
        }
        Ok(())
    }

    pub fn dup(&mut self) -> Result<(), ForthError> {
        let v = self.peek_ds()?;
        self.push_ds(v);
        Ok(())
    }

    pub fn drop(&mut self) -> Result<(), ForthError> {
        self.pop_ds()?;
        Ok(())
    }

    pub fn swap(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(b);
        self.push_ds(a);
        Ok(())
    }

    pub fn over(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(a);
        self.push_ds(b);
        self.push_ds(a);
        Ok(())
    }

    pub fn qdup(&mut self) -> Result<(), ForthError> {
        let v = self.peek_ds()?;
        if v != 0 {
            self.push_ds(v);
        }
        Ok(())
    }

    pub fn rot(&mut self) -> Result<(), ForthError> {
        let c = self.pop_ds()?;
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(b);
        self.push_ds(c);
        self.push_ds(a);
        Ok(())
    }

    pub fn nrot(&mut self) -> Result<(), ForthError> {
        let c = self.pop_ds()?;
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(c);
        self.push_ds(a);
        self.push_ds(b);
        Ok(())
    }

    pub fn nip(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        self.pop_ds()?;
        self.push_ds(b);
        Ok(())
    }

    pub fn tuck(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(b);
        self.push_ds(a);
        self.push_ds(b);
        Ok(())
    }

    pub fn two_dup(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(a);
        self.push_ds(b);
        self.push_ds(a);
        self.push_ds(b);
        Ok(())
    }

    pub fn two_drop(&mut self) -> Result<(), ForthError> {
        self.pop_ds()?;
        self.pop_ds()?;
        Ok(())
    }

    pub fn two_swap(&mut self) -> Result<(), ForthError> {
        let d = self.pop_ds()?;
        let c = self.pop_ds()?;
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(c);
        self.push_ds(d);
        self.push_ds(a);
        self.push_ds(b);
        Ok(())
    }

    pub fn two_over(&mut self) -> Result<(), ForthError> {
        let d = self.pop_ds()?;
        let c = self.pop_ds()?;
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(a);
        self.push_ds(b);
        self.push_ds(c);
        self.push_ds(d);
        self.push_ds(a);
        self.push_ds(b);
        Ok(())
    }

    pub fn to_r(&mut self) -> Result<(), ForthError> {
        let v = self.pop_ds()?;
        self.push_rs(v);
        Ok(())
    }

    pub fn r_from(&mut self) -> Result<(), ForthError> {
        let v = self.pop_rs()?;
        self.push_ds(v);
        Ok(())
    }

    pub fn r_fetch(&mut self) -> Result<(), ForthError> {
        let v = self.peek_rs()?;
        self.push_ds(v);
        Ok(())
    }

    pub fn plus(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(a.wrapping_add(b));
        Ok(())
    }

    pub fn minus(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(a.wrapping_sub(b));
        Ok(())
    }

    pub fn star(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(a.wrapping_mul(b));
        Ok(())
    }

    pub fn slash_mod(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        if b == 0 {
            return Err(ForthError {
                message: "DIVISION BY ZERO".to_string(),
            });
        }
        self.push_ds(a.wrapping_rem(b));
        self.push_ds(a.wrapping_div(b));
        Ok(())
    }

    pub fn slash(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        if b == 0 {
            return Err(ForthError {
                message: "DIVISION BY ZERO".to_string(),
            });
        }
        self.push_ds(a.wrapping_div(b));
        Ok(())
    }

    pub fn r#mod(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        if b == 0 {
            return Err(ForthError {
                message: "DIVISION BY ZERO".to_string(),
            });
        }
        self.push_ds(a.wrapping_rem(b));
        Ok(())
    }

    pub fn negate(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        self.push_ds(n.wrapping_neg());
        Ok(())
    }

    pub fn one_plus(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        self.push_ds(n.wrapping_add(1));
        Ok(())
    }

    pub fn one_minus(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        self.push_ds(n.wrapping_sub(1));
        Ok(())
    }

    pub fn two_plus(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        self.push_ds(n.wrapping_add(2));
        Ok(())
    }

    pub fn two_minus(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        self.push_ds(n.wrapping_sub(2));
        Ok(())
    }

    pub fn two_star(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        self.push_ds(n.wrapping_shl(1));
        Ok(())
    }

    pub fn two_slash(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        self.push_ds(n.wrapping_shr(1));
        Ok(())
    }

    pub fn abs(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        self.push_ds(if n < 0 { n.wrapping_neg() } else { n });
        Ok(())
    }

    pub fn min(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(if a < b { a } else { b });
        Ok(())
    }

    pub fn max(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(if a > b { a } else { b });
        Ok(())
    }

    pub fn and(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(a & b);
        Ok(())
    }

    pub fn or(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(a | b);
        Ok(())
    }

    pub fn xor(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(a ^ b);
        Ok(())
    }

    pub fn invert(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        self.push_ds(!n);
        Ok(())
    }

    pub fn key(&mut self) -> Result<(), ForthError> {
        let mut buf = [0u8; 1];
        match std::io::Read::read(&mut self.input, &mut buf) {
            Ok(0) => self.push_ds(0),
            Ok(_) => self.push_ds(buf[0] as Cell),
            Err(_) => self.push_ds(0),
        }
        Ok(())
    }

    pub fn emit(&mut self) -> Result<(), ForthError> {
        let v = self.pop_ds()?;
        let b = (v as u16 & 0xff) as u8;
        self.output.write_all(&[b]).map_err(|e| ForthError {
            message: e.to_string(),
        })?;
        Ok(())
    }

    pub fn cr(&mut self) -> Result<(), ForthError> {
        self.output.write_all(b"\n").map_err(|e| ForthError {
            message: e.to_string(),
        })?;
        Ok(())
    }

    pub fn space(&mut self) -> Result<(), ForthError> {
        self.output.write_all(b" ").map_err(|e| ForthError {
            message: e.to_string(),
        })?;
        Ok(())
    }

    pub fn spaces(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        if n > 0 {
            for _ in 0..n {
                self.space()?;
            }
        }
        Ok(())
    }

    pub fn type_word(&mut self) -> Result<(), ForthError> {
        let len = self.pop_ds()?;
        let addr = self.pop_ds()? as u16 as usize;
        if len > 0 {
            for i in 0..len as usize {
                let a = addr + i;
                if a <= 65535 {
                    let b = self.memory[a];
                    self.output.write_all(&[b]).map_err(|e| ForthError {
                        message: e.to_string(),
                    })?;
                }
            }
        }
        Ok(())
    }

    pub fn bye(&mut self) -> Result<(), ForthError> {
        self.running = false;
        Ok(())
    }

    pub fn read_line(&mut self) -> Result<Option<String>, ForthError> {
        let mut line = String::new();
        match self.input.read_line(&mut line) {
            Ok(0) => {
                self.running = false;
                Ok(None)
            }
            Ok(_) => {
                if line.ends_with('\n') {
                    line.pop();
                }
                if line.ends_with('\r') {
                    line.pop();
                }
                Ok(Some(line))
            }
            Err(e) => Err(ForthError {
                message: e.to_string(),
            }),
        }
    }

    pub fn expect(&mut self) -> Result<(), ForthError> {
        let maxlen = self.pop_ds()?;
        let addr = self.pop_ds()? as u16 as usize;
        let line: String = self.read_line()?.unwrap_or_default();
        let mut count = line.len();
        if maxlen > 0 && count > maxlen as usize {
            count = maxlen as usize;
        }
        for (i, b) in line.as_bytes().iter().enumerate().take(count) {
            let a = addr + i;
            if a <= 65535 {
                self.memory[a] = *b;
            }
        }
        let after = addr + count;
        if after <= 65535 {
            self.memory[after] = 0;
        }
        self.store_cell(SPAN_ADDR as u32, count as Cell)?;
        Ok(())
    }

    pub fn span(&mut self) -> Result<(), ForthError> {
        self.push_ds(SPAN_ADDR as Cell);
        Ok(())
    }

    pub fn abort(&mut self) {
        self.ds.clear();
        self.rs.clear();
        self.compile_list.clear();
        self.compiling_xt = -1;
        self.state = false;
        self.in_ptr = self.tib.len();
    }

    pub fn check_interrupt(&mut self) -> Result<(), ForthError> {
        if !self.interrupted {
            return Ok(());
        }
        self.interrupted = false;
        self.abort();
        Err(ForthError {
            message: "INTERRUPTED".to_string(),
        })
    }

    pub fn interpret_line(&mut self, line: &str) -> Result<(), ForthError> {
        self.tib = line.to_string();
        self.in_ptr = 0;
        // Outer interpreter is filled in a later spec.
        Ok(())
    }

    pub fn repl_step(&mut self) -> Result<(), ForthError> {
        if !self.state {
            self.output.write_all(b" ok\n").map_err(|e| ForthError {
                message: e.to_string(),
            })?;
        }
        let line = match self.read_line()? {
            None => return Ok(()),
            Some(s) => s,
        };
        if let Err(e) = self
            .check_interrupt()
            .and_then(|_| self.interpret_line(&line))
        {
            let _ = self.output.write_all(b"\n");
            let _ = writeln!(self.output, "{}", e.message);
            self.abort();
        }
        Ok(())
    }

    pub fn quit(&mut self) -> Result<(), ForthError> {
        self.running = true;
        self.abort();
        let flag = install_sigint_flag();
        while self.running {
            if flag.load(std::sync::atomic::Ordering::SeqCst) {
                self.interrupted = true;
                flag.store(false, std::sync::atomic::Ordering::SeqCst);
            }
            self.repl_step()?;
        }
        remove_sigint_flag();
        Ok(())
    }
}

static SIGINT_FLAG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SIGINT_INSTALLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn install_sigint_flag() -> &'static std::sync::atomic::AtomicBool {
    #[cfg(unix)]
    {
        if !SIGINT_INSTALLED.swap(true, std::sync::atomic::Ordering::SeqCst) {
            unsafe extern "C" {
                fn signal(sig: i32, handler: usize) -> usize;
            }
            const SIGINT: i32 = 2;
            unsafe extern "C" fn handle_sigint(_: i32) {
                SIGINT_FLAG.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            unsafe {
                signal(SIGINT, handle_sigint as *const () as usize);
            }
        }
    }
    &SIGINT_FLAG
}

fn remove_sigint_flag() {
    #[cfg(unix)]
    {
        if SIGINT_INSTALLED.swap(false, std::sync::atomic::Ordering::SeqCst) {
            unsafe extern "C" {
                fn signal(sig: i32, handler: usize) -> usize;
            }
            const SIGINT: i32 = 2;
            const SIG_DFL: usize = 0;
            unsafe {
                signal(SIGINT, SIG_DFL);
            }
        }
    }
    SIGINT_FLAG.store(false, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn key_and_emit() {
        let input = Cursor::new(b"X".to_vec());
        let mut output = Vec::new();
        {
            let mut f = Forth::new(input, &mut output);

            f.key().unwrap();
            assert_eq!(f.ds.len(), 1);
            assert_eq!(f.ds[0], b'X' as Cell);

            f.emit().unwrap();
            assert!(f.ds.is_empty());

            f.key().unwrap();
            assert_eq!(f.ds.pop(), Some(0));
        }
        assert_eq!(output, b"X");
    }

    #[test]
    fn cr_space_spaces_type_bye() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        {
            let mut f = Forth::new(input, &mut output);
            assert!(f.running);

            f.cr().unwrap();
            f.space().unwrap();
            f.push_ds(3);
            f.spaces().unwrap();

            let hello = b"HELLO";
            for (i, b) in hello.iter().enumerate() {
                f.memory[1024 + i] = *b;
            }
            f.push_ds(1024);
            f.push_ds(5);
            f.type_word().unwrap();
            assert!(f.ds.is_empty());

            f.bye().unwrap();
            assert!(!f.running);
        }
        assert_eq!(output, b"\n    HELLO");
    }

    fn fetch_cell(mem: &[u8; 65536], addr: usize) -> Cell {
        let lo = mem[addr] as u16;
        let hi = mem[addr + 1] as u16;
        (lo | (hi << 8)) as Cell
    }

    #[test]
    fn expect_stores_line_and_span() {
        let input = Cursor::new(b"TEST LINE\n".to_vec());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);
        f.push_ds(2000);
        f.push_ds(80);
        f.expect().unwrap();
        assert_eq!(&f.memory[2000..2009], b"TEST LINE");
        assert_eq!(f.memory[2009], 0);
        f.span().unwrap();
        assert_eq!(f.pop_ds().unwrap(), SPAN_ADDR as Cell);
        assert_eq!(fetch_cell(&f.memory, SPAN_ADDR), 9);
    }

    #[test]
    fn expect_truncates_to_maxlen() {
        let input = Cursor::new(b"A VERY LONG STRING\n".to_vec());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);
        f.push_ds(2000);
        f.push_ds(5);
        f.expect().unwrap();
        assert_eq!(&f.memory[2000..2005], b"A VER");
        assert_eq!(f.memory[2005], 0);
        assert_eq!(fetch_cell(&f.memory, SPAN_ADDR), 5);
    }

    #[test]
    fn repl_prints_ok_and_stops_on_eof() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        {
            let mut f = Forth::new(input, &mut output);
            assert_eq!(fetch_cell(&f.memory, DP_ADDR), INITIAL_DP);
            assert_eq!(fetch_cell(&f.memory, BASE_ADDR), 10);
            f.quit().unwrap();
            assert!(!f.running);
        }
        assert_eq!(output, b" ok\n");
    }

    #[test]
    fn interrupt_clears_both_stacks() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);
        f.push_ds(1);
        f.rs.push(2);
        f.interrupted = true;
        let err = f.check_interrupt().unwrap_err();
        assert_eq!(err.message, "INTERRUPTED");
        assert!(f.ds.is_empty());
        assert!(f.rs.is_empty());
    }

    #[test]
    fn stacks_push_pop_peek_and_underflow() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);
        f.push_ds(10);
        f.push_ds(20);
        assert_eq!(f.peek_ds().unwrap(), 20);
        assert_eq!(f.pop_ds().unwrap(), 20);
        assert_eq!(f.pop_ds().unwrap(), 10);
        assert_eq!(f.pop_ds().unwrap_err().message, "DATA STACK UNDERFLOW");
        assert_eq!(f.peek_ds().unwrap_err().message, "DATA STACK UNDERFLOW");

        f.push_rs(3);
        assert_eq!(f.peek_rs().unwrap(), 3);
        assert_eq!(f.pop_rs().unwrap(), 3);
        assert_eq!(f.pop_rs().unwrap_err().message, "RETURN STACK UNDERFLOW");
        assert_eq!(f.peek_rs().unwrap_err().message, "RETURN STACK UNDERFLOW");
    }

    #[test]
    fn depth_pushes_count_before_push() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);
        f.depth().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 0);
        f.push_ds(7);
        f.push_ds(8);
        f.depth().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 2);
        assert_eq!(f.ds, vec![7, 8]);
    }

    #[test]
    fn memory_little_endian_and_bounds() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.c_store(0, 0xAB).unwrap();
        assert_eq!(f.c_fetch(0).unwrap(), 0xAB);
        f.c_store(65535, 0xFE).unwrap();
        assert_eq!(f.c_fetch(65535).unwrap(), 0xFE);
        assert_eq!(f.c_fetch(65536).unwrap_err().message, "OUT OF MEMORY RANGE");
        assert_eq!(
            f.c_store(65536, 1).unwrap_err().message,
            "OUT OF MEMORY RANGE"
        );

        f.store_cell(1024, 0x1234).unwrap();
        assert_eq!(f.c_fetch(1024).unwrap(), 0x34);
        assert_eq!(f.c_fetch(1025).unwrap(), 0x12);
        assert_eq!(f.fetch_cell(1024).unwrap(), 0x1234);

        f.store_cell(2048, -1).unwrap();
        assert_eq!(f.c_fetch(2048).unwrap(), 0xFF);
        assert_eq!(f.c_fetch(2049).unwrap(), 0xFF);
        assert_eq!(f.fetch_cell(2048).unwrap(), -1);

        assert_eq!(
            f.fetch_cell(65535).unwrap_err().message,
            "OUT OF MEMORY RANGE"
        );
        assert_eq!(
            f.store_cell(65535, 10).unwrap_err().message,
            "OUT OF MEMORY RANGE"
        );
    }

    #[test]
    fn fetch_store_and_plus_store() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.push_ds(12345);
        f.push_ds(4000);
        f.word_store().unwrap();
        assert!(f.ds.is_empty());
        assert_eq!(f.fetch_cell(4000).unwrap(), 12345);

        f.push_ds(4000);
        f.word_fetch().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 12345);

        assert_eq!(f.word_fetch().unwrap_err().message, "DATA STACK UNDERFLOW");
        f.push_ds(100);
        assert_eq!(f.word_store().unwrap_err().message, "DATA STACK UNDERFLOW");

        f.push_ds(0x42);
        f.push_ds(5000);
        f.word_c_store().unwrap();
        assert!(f.ds.is_empty());
        assert_eq!(f.c_fetch(5000).unwrap(), 0x42);

        f.push_ds(5000);
        f.word_c_fetch().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 0x42);

        assert_eq!(
            f.word_c_fetch().unwrap_err().message,
            "DATA STACK UNDERFLOW"
        );
        f.push_ds(100);
        assert_eq!(
            f.word_c_store().unwrap_err().message,
            "DATA STACK UNDERFLOW"
        );

        f.push_ds(10);
        f.push_ds(6000);
        f.word_store().unwrap();
        f.push_ds(25);
        f.push_ds(6000);
        f.plus_store().unwrap();
        assert!(f.ds.is_empty());
        assert_eq!(f.fetch_cell(6000).unwrap(), 35);

        f.push_ds(-5);
        f.push_ds(6000);
        f.plus_store().unwrap();
        assert_eq!(f.fetch_cell(6000).unwrap(), 30);
    }

    #[test]
    fn fill_erase_cmove() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.push_ds(8000);
        f.push_ds(10);
        f.push_ds(0xEE);
        f.fill().unwrap();
        assert!(f.ds.is_empty());
        for i in 0..10 {
            assert_eq!(f.c_fetch(8000 + i).unwrap(), 0xEE);
        }

        f.push_ds(8000);
        f.push_ds(5);
        f.erase().unwrap();
        assert!(f.ds.is_empty());
        for i in 0..5 {
            assert_eq!(f.c_fetch(8000 + i).unwrap(), 0);
        }
        for i in 5..10 {
            assert_eq!(f.c_fetch(8000 + i).unwrap(), 0xEE);
        }

        for i in 0..5 {
            f.c_store(9000 + i, (i + 10) as u8).unwrap();
        }
        f.push_ds(9010);
        f.push_ds(9000);
        f.push_ds(5);
        f.cmove().unwrap();
        assert!(f.ds.is_empty());
        for i in 0..5 {
            assert_eq!(f.c_fetch(9010 + i).unwrap(), (i + 10) as u8);
        }

        for i in 0..5 {
            f.c_store(9020 + i, (i + 20) as u8).unwrap();
        }
        f.push_ds(9021);
        f.push_ds(9020);
        f.push_ds(4);
        f.cmove().unwrap();
        assert_eq!(f.c_fetch(9021).unwrap(), 20);
        assert_eq!(f.c_fetch(9022).unwrap(), 20);
        assert_eq!(f.c_fetch(9023).unwrap(), 20);
        assert_eq!(f.c_fetch(9024).unwrap(), 20);

        for i in 0..5 {
            f.c_store(9030 + i, (i + 30) as u8).unwrap();
        }
        f.push_ds(9031);
        f.push_ds(9030);
        f.push_ds(4);
        f.cmove_up().unwrap();
        assert_eq!(f.c_fetch(9031).unwrap(), 30);
        assert_eq!(f.c_fetch(9032).unwrap(), 31);
        assert_eq!(f.c_fetch(9033).unwrap(), 32);
        assert_eq!(f.c_fetch(9034).unwrap(), 33);
    }

    #[test]
    fn basic_stack_words() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.push_ds(42);
        f.dup().unwrap();
        assert_eq!(f.ds, vec![42, 42]);
        f.ds.clear();
        assert_eq!(f.dup().unwrap_err().message, "DATA STACK UNDERFLOW");

        f.push_ds(10);
        f.push_ds(20);
        f.drop().unwrap();
        assert_eq!(f.ds, vec![10]);
        f.drop().unwrap();
        assert_eq!(f.drop().unwrap_err().message, "DATA STACK UNDERFLOW");

        f.push_ds(1);
        f.push_ds(2);
        f.swap().unwrap();
        assert_eq!(f.ds, vec![2, 1]);
        f.ds.clear();
        f.push_ds(1);
        assert_eq!(f.swap().unwrap_err().message, "DATA STACK UNDERFLOW");

        f.ds.clear();
        f.push_ds(100);
        f.push_ds(200);
        f.over().unwrap();
        assert_eq!(f.ds, vec![100, 200, 100]);
        f.ds.clear();
        f.push_ds(1);
        assert_eq!(f.over().unwrap_err().message, "DATA STACK UNDERFLOW");
    }

    #[test]
    fn extended_stack_words() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.push_ds(123);
        f.qdup().unwrap();
        assert_eq!(f.ds, vec![123, 123]);
        f.ds.clear();
        f.push_ds(0);
        f.qdup().unwrap();
        assert_eq!(f.ds, vec![0]);
        f.ds.clear();
        assert_eq!(f.qdup().unwrap_err().message, "DATA STACK UNDERFLOW");

        f.push_ds(10);
        f.push_ds(20);
        f.push_ds(30);
        f.rot().unwrap();
        assert_eq!(f.ds, vec![20, 30, 10]);
        f.ds.clear();
        f.push_ds(1);
        f.push_ds(2);
        assert_eq!(f.rot().unwrap_err().message, "DATA STACK UNDERFLOW");

        f.ds.clear();
        f.push_ds(10);
        f.push_ds(20);
        f.push_ds(30);
        f.nrot().unwrap();
        assert_eq!(f.ds, vec![30, 10, 20]);
        f.ds.clear();
        f.push_ds(1);
        f.push_ds(2);
        assert_eq!(f.nrot().unwrap_err().message, "DATA STACK UNDERFLOW");

        f.ds.clear();
        f.push_ds(11);
        f.push_ds(22);
        f.nip().unwrap();
        assert_eq!(f.ds, vec![22]);
        f.ds.clear();
        f.push_ds(1);
        assert_eq!(f.nip().unwrap_err().message, "DATA STACK UNDERFLOW");

        f.ds.clear();
        f.push_ds(11);
        f.push_ds(22);
        f.tuck().unwrap();
        assert_eq!(f.ds, vec![22, 11, 22]);
        f.ds.clear();
        f.push_ds(1);
        assert_eq!(f.tuck().unwrap_err().message, "DATA STACK UNDERFLOW");
    }

    #[test]
    fn double_stack_words() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.push_ds(10);
        f.push_ds(20);
        f.two_dup().unwrap();
        assert_eq!(f.ds, vec![10, 20, 10, 20]);
        f.ds.clear();
        f.push_ds(1);
        assert_eq!(f.two_dup().unwrap_err().message, "DATA STACK UNDERFLOW");

        f.ds.clear();
        f.push_ds(1);
        f.push_ds(2);
        f.push_ds(3);
        f.push_ds(4);
        f.two_drop().unwrap();
        assert_eq!(f.ds, vec![1, 2]);
        f.ds.clear();
        f.push_ds(1);
        assert_eq!(f.two_drop().unwrap_err().message, "DATA STACK UNDERFLOW");

        f.ds.clear();
        f.push_ds(1);
        f.push_ds(2);
        f.push_ds(3);
        f.push_ds(4);
        f.two_swap().unwrap();
        assert_eq!(f.ds, vec![3, 4, 1, 2]);
        f.ds.clear();
        f.push_ds(1);
        f.push_ds(2);
        f.push_ds(3);
        assert_eq!(f.two_swap().unwrap_err().message, "DATA STACK UNDERFLOW");

        f.ds.clear();
        f.push_ds(1);
        f.push_ds(2);
        f.push_ds(3);
        f.push_ds(4);
        f.two_over().unwrap();
        assert_eq!(f.ds, vec![1, 2, 3, 4, 1, 2]);
        f.ds.clear();
        f.push_ds(1);
        f.push_ds(2);
        f.push_ds(3);
        assert_eq!(f.two_over().unwrap_err().message, "DATA STACK UNDERFLOW");
    }

    #[test]
    fn return_stack_transfers() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.push_ds(777);
        f.to_r().unwrap();
        assert!(f.ds.is_empty());
        assert_eq!(f.peek_rs().unwrap(), 777);

        f.r_fetch().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 777);
        assert_eq!(f.rs.len(), 1);

        f.r_from().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 777);
        assert!(f.rs.is_empty());

        assert_eq!(f.to_r().unwrap_err().message, "DATA STACK UNDERFLOW");
        assert_eq!(f.r_from().unwrap_err().message, "RETURN STACK UNDERFLOW");
        assert_eq!(f.r_fetch().unwrap_err().message, "RETURN STACK UNDERFLOW");
    }

    #[test]
    fn arithmetic_and_division() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.push_ds(3);
        f.push_ds(4);
        f.plus().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 7);

        f.push_ds(10);
        f.push_ds(3);
        f.minus().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 7);

        f.push_ds(6);
        f.push_ds(7);
        f.star().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 42);

        f.push_ds(14);
        f.push_ds(3);
        f.slash_mod().unwrap();
        assert_eq!(f.ds, vec![2, 4]);
        f.ds.clear();

        f.push_ds(14);
        f.push_ds(3);
        f.slash().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 4);

        f.push_ds(14);
        f.push_ds(3);
        f.r#mod().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 2);

        f.push_ds(14);
        f.push_ds(0);
        assert_eq!(f.slash_mod().unwrap_err().message, "DIVISION BY ZERO");
        assert!(f.ds.is_empty());

        f.push_ds(5);
        f.negate().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -5);
    }

    #[test]
    fn scaling_abs_min_max() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.push_ds(5);
        f.one_plus().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 6);
        f.push_ds(5);
        f.one_minus().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 4);
        f.push_ds(5);
        f.two_plus().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 7);
        f.push_ds(5);
        f.two_minus().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 3);
        f.push_ds(5);
        f.two_star().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 10);
        f.push_ds(-8);
        f.two_slash().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -4);

        f.push_ds(-7);
        f.abs().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 7);
        f.push_ds(7);
        f.abs().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 7);

        f.push_ds(3);
        f.push_ds(8);
        f.min().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 3);
        f.push_ds(3);
        f.push_ds(8);
        f.max().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 8);
    }

    #[test]
    fn bitwise_logic() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.push_ds(0b1100);
        f.push_ds(0b1010);
        f.and().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 0b1000);

        f.push_ds(0b1100);
        f.push_ds(0b1010);
        f.or().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 0b1110);

        f.push_ds(0b1100);
        f.push_ds(0b1010);
        f.xor().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 0b0110);

        f.push_ds(0);
        f.invert().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);
    }
}
