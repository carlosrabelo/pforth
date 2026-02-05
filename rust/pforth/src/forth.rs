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
}
