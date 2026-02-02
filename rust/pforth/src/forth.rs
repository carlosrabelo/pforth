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
            input,
            output,
        };
        f.store_cell_unchecked(SPAN_ADDR, 0);
        f.store_cell_unchecked(DP_ADDR, INITIAL_DP);
        f.store_cell_unchecked(BASE_ADDR, 10);
        f.init_primitives();
        f.require_module("CORE");
        f
    }

    fn store_cell_unchecked(&mut self, addr: usize, val: Cell) {
        let u = val as u16;
        self.memory[addr] = (u & 0xff) as u8;
        self.memory[addr + 1] = (u >> 8) as u8;
    }

    fn init_primitives(&mut self) {
        // Kernel registration grows in later tasks.
    }

    fn require_module(&mut self, _name: &str) {
        // Module loading grows in later tasks.
    }

    pub fn ds_push(&mut self, v: Cell) {
        self.ds.push(v);
    }

    pub fn ds_pop(&mut self) -> Result<Cell, ForthError> {
        self.ds.pop().ok_or_else(|| ForthError {
            message: "DATA STACK UNDERFLOW".to_string(),
        })
    }

    pub fn key(&mut self) -> Result<(), ForthError> {
        let mut buf = [0u8; 1];
        match std::io::Read::read(&mut self.input, &mut buf) {
            Ok(0) => self.ds_push(0),
            Ok(_) => self.ds_push(buf[0] as Cell),
            Err(_) => self.ds_push(0),
        }
        Ok(())
    }

    pub fn emit(&mut self) -> Result<(), ForthError> {
        let v = self.ds_pop()?;
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
        let n = self.ds_pop()?;
        if n > 0 {
            for _ in 0..n {
                self.space()?;
            }
        }
        Ok(())
    }

    pub fn type_word(&mut self) -> Result<(), ForthError> {
        let len = self.ds_pop()?;
        let addr = self.ds_pop()? as u16 as usize;
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
            f.ds_push(3);
            f.spaces().unwrap();

            let hello = b"HELLO";
            for (i, b) in hello.iter().enumerate() {
                f.memory[1024 + i] = *b;
            }
            f.ds_push(1024);
            f.ds_push(5);
            f.type_word().unwrap();
            assert!(f.ds.is_empty());

            f.bye().unwrap();
            assert!(!f.running);
        }
        assert_eq!(output, b"\n    HELLO");
    }
}
