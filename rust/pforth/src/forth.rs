use std::collections::HashMap;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordKind {
    Primitive,
    Colon,
    Constant,
    Variable,
    Create,
}

pub type PrimFn<R, W> = fn(&mut Forth<R, W>) -> Result<(), ForthError>;

pub struct Word<R: BufRead, W: Write> {
    pub name: String,
    pub immediate: bool,
    pub kind: WordKind,
    pub code: Option<PrimFn<R, W>>,
    pub body: Vec<Cell>,
    pub pfa: i32,
}

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
    pub current_body: Vec<Cell>,
    pub ip: usize,
    pub words: Vec<Word<R, W>>,
    pub words_by_name: HashMap<String, usize>,
    pub latest: isize,
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
            current_body: Vec::new(),
            ip: 0,
            words: Vec::new(),
            words_by_name: HashMap::new(),
            latest: -1,
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
        self.define_primitive("DUP", Self::dup);
        self.define_primitive("OVER", Self::over);
        self.define_primitive("SWAP", Self::swap);
        self.define_primitive("+", Self::plus);
        self.define_primitive("*", Self::star);
        self.define_primitive("EXIT", Self::exit);
        self.define_primitive("LIT", Self::lit);
        self.define_primitive("STATE", Self::word_state);
        self.define_immediate("[", Self::left_bracket);
        self.define_primitive("]", Self::right_bracket);
        self.define_immediate("LITERAL", Self::literal);
        self.define_primitive("COMPILE", Self::compile);
        self.define_immediate("[COMPILE]", Self::bracket_compile);
        self.define_primitive(":", Self::colon);
        self.define_immediate(";", Self::semicolon);
        self.define_immediate("IMMEDIATE", Self::immediate);
        self.define_immediate("RECURSE", Self::recurse);
    }

    fn require_module(&mut self, _name: &str) {
        // Module loading grows in later tasks.
    }

    fn uppercase_name(name: &str) -> String {
        name.chars().map(|c| c.to_ascii_uppercase()).collect()
    }

    pub fn define_word(
        &mut self,
        name: &str,
        kind: WordKind,
        code: Option<PrimFn<R, W>>,
        body: Vec<Cell>,
        immediate: bool,
    ) -> usize {
        let upper = Self::uppercase_name(name);
        let xt = self.words.len();
        self.words.push(Word {
            name: upper.clone(),
            immediate,
            kind,
            code,
            body,
            pfa: 0,
        });
        self.words_by_name.insert(upper, xt);
        self.latest = xt as isize;
        xt
    }

    pub fn define_primitive(&mut self, name: &str, code: PrimFn<R, W>) -> usize {
        self.define_word(name, WordKind::Primitive, Some(code), Vec::new(), false)
    }

    pub fn define_immediate(&mut self, name: &str, code: PrimFn<R, W>) -> usize {
        self.define_word(name, WordKind::Primitive, Some(code), Vec::new(), true)
    }

    pub fn lookup(&self, name: &str) -> Option<usize> {
        let upper = Self::uppercase_name(name);
        self.words_by_name.get(&upper).copied()
    }

    pub fn find_xt(&self, name: &str) -> Option<usize> {
        self.lookup(name)
    }

    pub fn parse_word(&mut self) -> Option<String> {
        let bytes = self.tib.as_bytes();
        while self.in_ptr < bytes.len() {
            let b = bytes[self.in_ptr];
            if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
                self.in_ptr += 1;
            } else {
                break;
            }
        }
        if self.in_ptr >= bytes.len() {
            return None;
        }
        let start = self.in_ptr;
        while self.in_ptr < bytes.len() {
            let b = bytes[self.in_ptr];
            if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
                break;
            }
            self.in_ptr += 1;
        }
        Some(self.tib[start..self.in_ptr].to_string())
    }

    pub fn find(&mut self) -> Result<(), ForthError> {
        let addr = self.pop_ds()? as u16 as u32;
        let len = self.c_fetch(addr)? as usize;
        let mut name = String::new();
        for i in 0..len {
            let ch = self.c_fetch(addr + 1 + i as u32)? as char;
            name.push(ch.to_ascii_uppercase());
        }
        match self.lookup(&name) {
            None => {
                self.push_ds(addr as Cell);
                self.push_ds(0);
            }
            Some(xt) => {
                let imm = self.words[xt].immediate;
                self.push_ds(xt as Cell);
                self.push_ds(if imm { 1 } else { -1 });
            }
        }
        Ok(())
    }

    pub fn tick(&mut self) -> Result<(), ForthError> {
        let token = self.parse_word().ok_or_else(|| ForthError {
            message: "WORD NAME EXPECTED".to_string(),
        })?;
        let xt = self.find_xt(&token).ok_or_else(|| ForthError {
            message: format!("WORD NOT FOUND: {token}"),
        })?;
        self.push_ds(xt as Cell);
        Ok(())
    }

    pub fn to_body(&mut self) -> Result<(), ForthError> {
        let xt = self.pop_ds()?;
        if xt >= 0 {
            let idx = xt as usize;
            if idx < self.words.len() && self.words[idx].pfa != 0 {
                self.push_ds(self.words[idx].pfa as Cell);
                return Ok(());
            }
        }
        self.push_ds(xt.wrapping_add(2));
        Ok(())
    }

    pub fn dp(&mut self) -> Result<(), ForthError> {
        self.push_ds(DP_ADDR as Cell);
        Ok(())
    }

    pub fn here(&mut self) -> Result<(), ForthError> {
        let v = self.fetch_cell(DP_ADDR as u32)?;
        self.push_ds(v);
        Ok(())
    }

    pub fn here_value(&self) -> Result<Cell, ForthError> {
        self.fetch_cell(DP_ADDR as u32)
    }

    pub fn pad(&mut self) -> Result<(), ForthError> {
        let h = self.here_value()?;
        self.push_ds(h.wrapping_add(68));
        Ok(())
    }

    pub fn allot(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        let cur = self.here_value()?;
        self.store_cell(DP_ADDR as u32, cur.wrapping_add(n))
    }

    pub fn comma(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        let addr = self.here_value()? as u16 as u32;
        self.store_cell(addr, n)?;
        self.push_ds(2);
        self.allot()
    }

    pub fn c_comma(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        let addr = self.here_value()? as u16 as u32;
        self.c_store(addr, (n as u16 & 0xff) as u8)?;
        self.push_ds(1);
        self.allot()
    }

    pub fn exit(&mut self) -> Result<(), ForthError> {
        self.ip = self.current_body.len();
        Ok(())
    }

    pub fn execute(&mut self) -> Result<(), ForthError> {
        let xt = self.pop_ds()?;
        self.execute_xt(xt)
    }

    pub fn execute_xt(&mut self, xt: Cell) -> Result<(), ForthError> {
        if xt < 0 || xt as usize >= self.words.len() {
            return Err(ForthError {
                message: format!("INVALID EXECUTION TOKEN: {xt}"),
            });
        }
        let idx = xt as usize;
        let kind = self.words[idx].kind;
        match kind {
            WordKind::Primitive => {
                if let Some(code) = self.words[idx].code {
                    code(self)?;
                }
            }
            WordKind::Constant => {
                if let Some(&v) = self.words[idx].body.first() {
                    self.push_ds(v);
                }
            }
            WordKind::Variable | WordKind::Create => {
                self.push_ds(self.words[idx].pfa as Cell);
            }
            WordKind::Colon => {
                let body = self.words[idx].body.clone();
                let saved_body = std::mem::replace(&mut self.current_body, body);
                let saved_ip = self.ip;
                self.ip = 0;
                let result = (|| {
                    while self.ip < self.current_body.len() {
                        self.check_interrupt()?;
                        let nxt = self.current_body[self.ip];
                        self.ip += 1;
                        self.execute_xt(nxt)?;
                    }
                    Ok(())
                })();
                self.current_body = saved_body;
                self.ip = saved_ip;
                result?;
            }
        }
        Ok(())
    }

    pub fn create(&mut self) -> Result<(), ForthError> {
        let token = self.parse_word().ok_or_else(|| ForthError {
            message: "WORD NAME EXPECTED".to_string(),
        })?;
        let pfa = self.here_value()?;
        let xt = self.define_word(&token, WordKind::Create, None, Vec::new(), false);
        self.words[xt].pfa = pfa as i32;
        Ok(())
    }

    pub fn constant(&mut self) -> Result<(), ForthError> {
        let val = self.pop_ds()?;
        let token = self.parse_word().ok_or_else(|| ForthError {
            message: "WORD NAME EXPECTED".to_string(),
        })?;
        self.define_word(&token, WordKind::Constant, None, vec![val], false);
        Ok(())
    }

    pub fn variable(&mut self) -> Result<(), ForthError> {
        let token = self.parse_word().ok_or_else(|| ForthError {
            message: "WORD NAME EXPECTED".to_string(),
        })?;
        let pfa = self.here_value()?;
        let xt = self.define_word(&token, WordKind::Variable, None, Vec::new(), false);
        self.words[xt].pfa = pfa as i32;
        self.store_cell(pfa as u16 as u32, 0)?;
        self.push_ds(2);
        self.allot()
    }

    pub fn base(&mut self) -> Result<(), ForthError> {
        self.push_ds(BASE_ADDR as Cell);
        Ok(())
    }

    pub fn get_base(&self) -> Result<i32, ForthError> {
        let b = self.fetch_cell(BASE_ADDR as u32)? as i32;
        if b < 1 {
            Ok(10)
        } else {
            Ok(b)
        }
    }

    pub fn parse_number(&self, token: &str) -> Option<Cell> {
        if token.is_empty() {
            return None;
        }
        let cur_base = self.get_base().ok()? as u32;
        let bytes = token.as_bytes();
        let mut start = 0;
        let mut neg = false;
        if bytes[0] == b'-' {
            neg = true;
            start = 1;
        } else if bytes[0] == b'+' {
            start = 1;
        }
        if start >= bytes.len() {
            return None;
        }
        let mut parsed_base = cur_base;
        if bytes[start] == b'$' {
            parsed_base = 16;
            start += 1;
        } else if bytes[start] == b'%' {
            parsed_base = 2;
            start += 1;
        } else if bytes[start] == b'#' {
            start += 1;
            if start < bytes.len() && bytes[start] == b'#' {
                start += 1;
            }
        }
        if start >= bytes.len() {
            return None;
        }
        let mut val: u32 = 0;
        for &c in &bytes[start..] {
            let digit = match c {
                b'0'..=b'9' => (c - b'0') as u32,
                b'A'..=b'F' => (c - b'A' + 10) as u32,
                b'a'..=b'f' => (c - b'a' + 10) as u32,
                _ => return None,
            };
            if digit >= parsed_base {
                return None;
            }
            val = val * parsed_base + digit;
        }
        let result = (val & 0xffff) as Cell;
        Some(if neg { result.wrapping_neg() } else { result })
    }

    pub fn word(&mut self) -> Result<(), ForthError> {
        let delim = (self.pop_ds()? as u16 & 0xff) as u8 as char;
        while self.in_ptr < self.tib.len() {
            let ch = self.tib.as_bytes()[self.in_ptr] as char;
            if ch == delim {
                self.in_ptr += 1;
            } else {
                break;
            }
        }
        let dest = self.here_value()? as u16 as u32;
        if self.in_ptr >= self.tib.len() {
            self.c_store(dest, 0)?;
            self.push_ds(dest as Cell);
            return Ok(());
        }
        let start = dest + 1;
        let mut count: u8 = 0;
        while self.in_ptr < self.tib.len() {
            let b = self.tib.as_bytes()[self.in_ptr];
            let ch = b as char;
            if ch == delim || ch == '\r' || ch == '\n' {
                break;
            }
            self.c_store(start + count as u32, b)?;
            count = count.wrapping_add(1);
            self.in_ptr += 1;
        }
        self.c_store(dest, count)?;
        self.c_store(start + count as u32, b' ')?;
        self.push_ds(dest as Cell);
        Ok(())
    }

    pub fn number(&mut self) -> Result<(), ForthError> {
        let addr = self.pop_ds()? as u16 as u32;
        let len = self.c_fetch(addr)? as usize;
        let mut token = String::new();
        for i in 0..len {
            token.push(self.c_fetch(addr + 1 + i as u32)? as char);
        }
        if let Some(n) = self.parse_number(&token) {
            self.push_ds(n);
            self.push_ds(-1);
        } else {
            self.push_ds(0);
            self.push_ds(0);
        }
        Ok(())
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

    fn flag(v: bool) -> Cell {
        if v {
            -1
        } else {
            0
        }
    }

    pub fn equals(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(Self::flag(a == b));
        Ok(())
    }

    pub fn not_equals(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(Self::flag(a != b));
        Ok(())
    }

    pub fn less(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(Self::flag(a < b));
        Ok(())
    }

    pub fn greater(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(Self::flag(a > b));
        Ok(())
    }

    pub fn less_equal(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(Self::flag(a <= b));
        Ok(())
    }

    pub fn greater_equal(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()?;
        let a = self.pop_ds()?;
        self.push_ds(Self::flag(a >= b));
        Ok(())
    }

    pub fn zero_equals(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        self.push_ds(Self::flag(n == 0));
        Ok(())
    }

    pub fn zero_not_equals(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        self.push_ds(Self::flag(n != 0));
        Ok(())
    }

    pub fn zero_less(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        self.push_ds(Self::flag(n < 0));
        Ok(())
    }

    pub fn zero_greater(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        self.push_ds(Self::flag(n > 0));
        Ok(())
    }

    pub fn u_less(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()? as u16;
        let a = self.pop_ds()? as u16;
        self.push_ds(Self::flag(a < b));
        Ok(())
    }

    pub fn u_greater(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()? as u16;
        let a = self.pop_ds()? as u16;
        self.push_ds(Self::flag(a > b));
        Ok(())
    }

    pub fn u_less_equal(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()? as u16;
        let a = self.pop_ds()? as u16;
        self.push_ds(Self::flag(a <= b));
        Ok(())
    }

    pub fn u_greater_equal(&mut self) -> Result<(), ForthError> {
        let b = self.pop_ds()? as u16;
        let a = self.pop_ds()? as u16;
        self.push_ds(Self::flag(a >= b));
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

    pub fn lit(&mut self) -> Result<(), ForthError> {
        if self.ip < self.current_body.len() {
            let v = self.current_body[self.ip];
            self.ip += 1;
            self.push_ds(v);
        }
        Ok(())
    }

    pub fn interpret(&mut self) -> Result<(), ForthError> {
        while self.in_ptr < self.tib.len() {
            self.check_interrupt()?;
            let token = match self.parse_word() {
                Some(t) => t,
                None => break,
            };
            if let Some(xt) = self.find_xt(&token) {
                let immediate = self.words[xt].immediate;
                if self.state && !immediate {
                    self.compile_list.push(xt as Cell);
                } else {
                    self.execute_xt(xt as Cell)?;
                }
            } else if let Some(num) = self.parse_number(&token) {
                if self.state {
                    let lit_xt = self.find_xt("LIT").unwrap() as Cell;
                    self.compile_list.push(lit_xt);
                    self.compile_list.push(num);
                } else {
                    self.push_ds(num);
                }
            } else {
                self.abort();
                return Err(ForthError {
                    message: format!("UNKNOWN WORD: {token}"),
                });
            }
        }
        Ok(())
    }

    pub fn interpret_line(&mut self, line: &str) -> Result<(), ForthError> {
        self.tib = line.to_string();
        self.in_ptr = 0;
        self.interpret()
    }

    pub fn word_state(&mut self) -> Result<(), ForthError> {
        self.push_ds(if self.state { -1 } else { 0 });
        Ok(())
    }

    pub fn left_bracket(&mut self) -> Result<(), ForthError> {
        self.state = false;
        Ok(())
    }

    pub fn right_bracket(&mut self) -> Result<(), ForthError> {
        self.state = true;
        Ok(())
    }

    pub fn literal(&mut self) -> Result<(), ForthError> {
        let val = self.pop_ds()?;
        let lit_xt = self.find_xt("LIT").unwrap() as Cell;
        self.compile_list.push(lit_xt);
        self.compile_list.push(val);
        Ok(())
    }

    pub fn compile(&mut self) -> Result<(), ForthError> {
        let token = self.parse_word().ok_or_else(|| ForthError {
            message: "WORD NAME EXPECTED".to_string(),
        })?;
        let xt = self.find_xt(&token).ok_or_else(|| ForthError {
            message: format!("WORD NOT FOUND: {token}"),
        })?;
        self.compile_list.push(xt as Cell);
        Ok(())
    }

    pub fn bracket_compile(&mut self) -> Result<(), ForthError> {
        self.compile()
    }

    pub fn colon(&mut self) -> Result<(), ForthError> {
        if self.state {
            return Err(ForthError {
                message: "ALREADY COMPILING".to_string(),
            });
        }
        let token = self.parse_word().ok_or_else(|| ForthError {
            message: "WORD NAME EXPECTED".to_string(),
        })?;
        self.compile_list.clear();
        let xt = self.define_word(&token, WordKind::Colon, None, Vec::new(), false);
        self.compiling_xt = xt as isize;
        self.state = true;
        Ok(())
    }

    pub fn semicolon(&mut self) -> Result<(), ForthError> {
        if !self.state {
            return Err(ForthError {
                message: "NOT IN COMPILATION STATE".to_string(),
            });
        }
        let exit_xt = self.find_xt("EXIT").unwrap() as Cell;
        self.compile_list.push(exit_xt);
        self.state = false;
        if self.compiling_xt >= 0 {
            let xt = self.compiling_xt as usize;
            if xt < self.words.len() {
                self.words[xt].body = self.compile_list.clone();
            }
        }
        self.compile_list.clear();
        self.compiling_xt = -1;
        Ok(())
    }

    pub fn immediate(&mut self) -> Result<(), ForthError> {
        if self.latest >= 0 {
            let xt = self.latest as usize;
            if xt < self.words.len() {
                self.words[xt].immediate = true;
            }
        }
        Ok(())
    }

    pub fn recurse(&mut self) -> Result<(), ForthError> {
        if !self.state {
            return Err(ForthError {
                message: "RECURSE ONLY IN COMPILATION".to_string(),
            });
        }
        if self.compiling_xt < 0 {
            return Err(ForthError {
                message: "NO COMPILATION IN PROGRESS".to_string(),
            });
        }
        self.compile_list.push(self.compiling_xt as Cell);
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

    #[test]
    fn signed_zero_and_unsigned_comparisons() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.push_ds(42);
        f.push_ds(42);
        f.equals().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);
        f.push_ds(42);
        f.push_ds(43);
        f.equals().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 0);

        f.push_ds(42);
        f.push_ds(43);
        f.not_equals().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);

        f.push_ds(-1);
        f.push_ds(1);
        f.less().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);
        f.push_ds(-1);
        f.push_ds(1);
        f.greater().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 0);

        f.push_ds(5);
        f.push_ds(5);
        f.less_equal().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);
        f.push_ds(5);
        f.push_ds(5);
        f.greater_equal().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);

        f.push_ds(0);
        f.zero_equals().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);
        f.push_ds(5);
        f.zero_not_equals().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);
        f.push_ds(-10);
        f.zero_less().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);
        f.push_ds(10);
        f.zero_greater().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);

        f.push_ds(-1);
        f.push_ds(1);
        f.u_greater().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);
        f.push_ds(-1);
        f.push_ds(1);
        f.u_less().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 0);
        f.push_ds(100);
        f.push_ds(100);
        f.u_less_equal().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);
        f.push_ds(100);
        f.push_ds(100);
        f.u_greater_equal().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);
    }

    #[test]
    fn dictionary_lookup_find_tick_and_to_body() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        assert!(f.lookup("DUP").is_some());
        assert!(f.lookup("dup").is_some());
        assert!(f.lookup("Dup").is_some());
        assert!(f.lookup("NONEXISTENT").is_none());
        let swap_xt = f.find_xt("swap").unwrap();
        assert_eq!(f.words[swap_xt].name, "SWAP");
        let over_xt = f.find_xt("over").unwrap();
        assert_eq!(f.words[over_xt].name, "OVER");

        f.c_store(3000, 3).unwrap();
        f.c_store(3001, b'd').unwrap();
        f.c_store(3002, b'u').unwrap();
        f.c_store(3003, b'p').unwrap();
        f.push_ds(3000);
        f.find().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);
        assert_eq!(f.pop_ds().unwrap(), f.find_xt("DUP").unwrap() as Cell);

        f.define_immediate("TESTIMM", |_f| Ok(()));
        f.c_store(3010, 7).unwrap();
        for (i, b) in b"TESTIMM".iter().enumerate() {
            f.c_store(3011 + i as u32, *b).unwrap();
        }
        f.push_ds(3010);
        f.find().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 1);
        assert_eq!(f.pop_ds().unwrap(), f.find_xt("TESTIMM").unwrap() as Cell);

        f.c_store(3020, 4).unwrap();
        f.c_store(3021, b'N').unwrap();
        f.c_store(3022, b'O').unwrap();
        f.c_store(3023, b'P').unwrap();
        f.c_store(3024, b'E').unwrap();
        f.push_ds(3020);
        f.find().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 0);
        assert_eq!(f.pop_ds().unwrap(), 3020);

        f.tib = "  over  ".to_string();
        f.in_ptr = 0;
        f.tick().unwrap();
        assert_eq!(f.pop_ds().unwrap(), f.find_xt("OVER").unwrap() as Cell);

        f.tib = "missingword".to_string();
        f.in_ptr = 0;
        assert_eq!(f.tick().unwrap_err().message, "WORD NOT FOUND: missingword");

        f.tib = "   ".to_string();
        f.in_ptr = 0;
        assert_eq!(f.tick().unwrap_err().message, "WORD NAME EXPECTED");

        let var_xt = f.define_word("MYVAR", WordKind::Variable, None, Vec::new(), false);
        f.words[var_xt].pfa = 5432;
        f.push_ds(var_xt as Cell);
        f.to_body().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 5432);

        f.push_ds(9999);
        f.to_body().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 10001);
    }

    #[test]
    fn here_allot_and_comma() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.dp().unwrap();
        assert_eq!(f.pop_ds().unwrap(), DP_ADDR as Cell);
        f.here().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 1024);
        f.pad().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 1024 + 68);

        f.push_ds(10);
        f.allot().unwrap();
        f.here().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 1034);

        f.push_ds(0x1234);
        f.comma().unwrap();
        f.here().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 1036);
        assert_eq!(f.fetch_cell(1034).unwrap(), 0x1234);

        f.push_ds(0xAB);
        f.c_comma().unwrap();
        f.here().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 1037);
        assert_eq!(f.c_fetch(1036).unwrap(), 0xAB);
    }

    #[test]
    fn execute_constant_and_invalid_token() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        let plus_xt = f.find_xt("+").unwrap();
        f.push_ds(12);
        f.push_ds(18);
        f.push_ds(plus_xt as Cell);
        f.execute().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 30);

        let const_xt = f.define_word("MYCONST", WordKind::Constant, None, vec![999], false);
        f.push_ds(const_xt as Cell);
        f.execute().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 999);

        let var_xt = f.define_word("MYVAR", WordKind::Variable, None, Vec::new(), false);
        f.words[var_xt].pfa = 2048;
        f.push_ds(var_xt as Cell);
        f.execute().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 2048);

        let cr_xt = f.define_word("MYBUF", WordKind::Create, None, Vec::new(), false);
        f.words[cr_xt].pfa = 4096;
        f.push_ds(cr_xt as Cell);
        f.execute().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 4096);

        f.push_ds(9999);
        assert_eq!(
            f.execute().unwrap_err().message,
            "INVALID EXECUTION TOKEN: 9999"
        );

        let exit_xt = f.find_xt("EXIT").unwrap() as Cell;
        let dup_xt = f.find_xt("DUP").unwrap() as Cell;
        let colon_xt = f.define_word(
            "TWICE",
            WordKind::Colon,
            None,
            vec![dup_xt, exit_xt, dup_xt],
            false,
        );
        f.push_ds(5);
        f.push_ds(colon_xt as Cell);
        f.execute().unwrap();
        assert_eq!(f.ds, vec![5, 5]);
    }

    #[test]
    fn create_constant_and_variable() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        let before_create = f.here_value().unwrap();
        f.tib = "MYBUF".to_string();
        f.in_ptr = 0;
        f.create().unwrap();
        let buf_xt = f.find_xt("MYBUF").unwrap();
        assert_eq!(f.words[buf_xt].kind, WordKind::Create);
        assert_eq!(f.words[buf_xt].pfa, before_create as i32);
        f.push_ds(buf_xt as Cell);
        f.execute().unwrap();
        assert_eq!(f.pop_ds().unwrap(), before_create);

        let before_var = f.here_value().unwrap();
        f.tib = "COUNTER".to_string();
        f.in_ptr = 0;
        f.variable().unwrap();
        let var_xt = f.find_xt("COUNTER").unwrap();
        assert_eq!(f.words[var_xt].kind, WordKind::Variable);
        assert_eq!(f.words[var_xt].pfa, before_var as i32);
        assert_eq!(f.here_value().unwrap(), before_var + 2);
        assert_eq!(f.fetch_cell(before_var as u32).unwrap(), 0);

        f.push_ds(var_xt as Cell);
        f.execute().unwrap();
        assert_eq!(f.peek_ds().unwrap(), before_var);
        f.word_fetch().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 0);

        f.push_ds(77);
        f.push_ds(var_xt as Cell);
        f.execute().unwrap();
        f.word_store().unwrap();
        assert_eq!(f.fetch_cell(before_var as u32).unwrap(), 77);

        f.push_ds(9876);
        f.tib = "LIMIT".to_string();
        f.in_ptr = 0;
        f.constant().unwrap();
        let const_xt = f.find_xt("LIMIT").unwrap();
        assert_eq!(f.words[const_xt].kind, WordKind::Constant);
        f.push_ds(const_xt as Cell);
        f.execute().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 9876);

        f.tib = "   ".to_string();
        f.in_ptr = 0;
        assert_eq!(f.create().unwrap_err().message, "WORD NAME EXPECTED");
        f.tib = "   ".to_string();
        f.in_ptr = 0;
        assert_eq!(f.variable().unwrap_err().message, "WORD NAME EXPECTED");
        f.push_ds(1);
        f.tib = "   ".to_string();
        f.in_ptr = 0;
        assert_eq!(f.constant().unwrap_err().message, "WORD NAME EXPECTED");
    }

    #[test]
    fn base_and_prefixed_numbers() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.base().unwrap();
        assert_eq!(f.pop_ds().unwrap(), BASE_ADDR as Cell);
        assert_eq!(f.get_base().unwrap(), 10);

        f.push_ds(16);
        f.push_ds(BASE_ADDR as Cell);
        f.word_store().unwrap();
        assert_eq!(f.get_base().unwrap(), 16);

        f.store_cell(BASE_ADDR as u32, 10).unwrap();
        assert_eq!(f.parse_number("123"), Some(123));
        assert_eq!(f.parse_number("-456"), Some(-456));
        assert_eq!(f.parse_number("+789"), Some(789));
        assert_eq!(f.parse_number("$1F"), Some(31));
        assert_eq!(f.parse_number("-$10"), Some(-16));
        assert_eq!(f.parse_number("%1010"), Some(10));

        f.store_cell(BASE_ADDR as u32, 16).unwrap();
        assert_eq!(f.parse_number("FF"), Some(255));
        f.store_cell(BASE_ADDR as u32, 8).unwrap();
        assert_eq!(f.parse_number("77"), Some(63));
        f.store_cell(BASE_ADDR as u32, 10).unwrap();
        assert_eq!(f.parse_number("ABC"), None);
        assert_eq!(f.parse_number(""), None);
        assert_eq!(f.parse_number("-"), None);
    }

    #[test]
    fn word_and_number() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.tib = "   HELLO   WORLD".to_string();
        f.in_ptr = 0;
        f.push_ds(b' ' as Cell);
        f.word().unwrap();
        let addr = f.pop_ds().unwrap();
        assert_eq!(addr, f.here_value().unwrap());
        assert_eq!(f.c_fetch(addr as u32).unwrap(), 5);
        let mut word_read = String::new();
        for i in 0..5 {
            word_read.push(f.c_fetch(addr as u32 + 1 + i).unwrap() as char);
        }
        assert_eq!(word_read, "HELLO");

        f.c_store(4000, 4).unwrap();
        f.c_store(4001, b'1').unwrap();
        f.c_store(4002, b'2').unwrap();
        f.c_store(4003, b'3').unwrap();
        f.c_store(4004, b'4').unwrap();
        f.push_ds(4000);
        f.number().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);
        assert_eq!(f.pop_ds().unwrap(), 1234);

        f.c_store(4010, 3).unwrap();
        f.c_store(4011, b'X').unwrap();
        f.c_store(4012, b'Y').unwrap();
        f.c_store(4013, b'Z').unwrap();
        f.push_ds(4010);
        f.number().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 0);
        assert_eq!(f.pop_ds().unwrap(), 0);
    }

    #[test]
    fn interpret_and_unknown_word() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.interpret_line("10 20 +").unwrap();
        assert_eq!(f.pop_ds().unwrap(), 30);

        f.interpret_line("5 DUP *").unwrap();
        assert_eq!(f.pop_ds().unwrap(), 25);

        f.interpret_line("1 2 3 4 + + +").unwrap();
        assert_eq!(f.pop_ds().unwrap(), 10);

        f.push_ds(100);
        assert_eq!(
            f.interpret_line("UNKNOWNWORD 50 +").unwrap_err().message,
            "UNKNOWN WORD: UNKNOWNWORD"
        );
        assert!(f.ds.is_empty());
        assert!(!f.state);
    }

    #[test]
    fn state_brackets_literal_and_compile() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.word_state().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 0);

        f.right_bracket().unwrap();
        f.word_state().unwrap();
        assert_eq!(f.pop_ds().unwrap(), -1);
        assert!(f.state);

        f.left_bracket().unwrap();
        f.word_state().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 0);
        assert!(!f.state);

        let lit_xt = f.find_xt("LIT").unwrap() as Cell;
        let test_xt = f.define_word("TESTLIT", WordKind::Colon, None, vec![lit_xt, 555], false);
        f.push_ds(test_xt as Cell);
        f.execute().unwrap();
        assert_eq!(f.pop_ds().unwrap(), 555);

        f.compile_list.clear();
        f.push_ds(888);
        f.literal().unwrap();
        assert_eq!(f.compile_list, vec![lit_xt, 888]);

        f.compile_list.clear();
        f.tib = "SWAP".to_string();
        f.in_ptr = 0;
        f.compile().unwrap();
        assert_eq!(f.compile_list, vec![f.find_xt("SWAP").unwrap() as Cell]);

        f.compile_list.clear();
        f.tib = "[".to_string();
        f.in_ptr = 0;
        f.bracket_compile().unwrap();
        assert_eq!(f.compile_list, vec![f.find_xt("[").unwrap() as Cell]);
    }

    #[test]
    fn colon_exit_immediate_and_recurse() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.interpret_line(": SQUARE DUP * ;").unwrap();
        assert!(!f.state);
        f.interpret_line("5 SQUARE").unwrap();
        assert_eq!(f.pop_ds().unwrap(), 25);

        f.interpret_line(": DOUBLE DUP + ;").unwrap();
        f.interpret_line(": QUAD DOUBLE DOUBLE ;").unwrap();
        f.interpret_line("3 QUAD").unwrap();
        assert_eq!(f.pop_ds().unwrap(), 12);

        f.interpret_line(": CONST_VAL [ 10 20 + ] LITERAL ;")
            .unwrap();
        f.interpret_line("CONST_VAL").unwrap();
        assert_eq!(f.pop_ds().unwrap(), 30);

        f.interpret_line(": EARLY_EXIT 10 EXIT 20 ;").unwrap();
        f.interpret_line("EARLY_EXIT").unwrap();
        assert_eq!(f.pop_ds().unwrap(), 10);

        f.interpret_line(": PUSH77 77 ; IMMEDIATE").unwrap();
        let push77_xt = f.find_xt("PUSH77").unwrap();
        assert!(f.words[push77_xt].immediate);
        f.interpret_line(": CALL_PUSH77 PUSH77 ;").unwrap();
        assert_eq!(f.pop_ds().unwrap(), 77);

        f.interpret_line(": FOOREC RECURSE ;").unwrap();
        let foorec_xt = f.find_xt("FOOREC").unwrap();
        let body = &f.words[foorec_xt].body;
        assert_eq!(body.len(), 2);
        assert_eq!(body[0], foorec_xt as Cell);
        assert_eq!(body[1], f.find_xt("EXIT").unwrap() as Cell);

        assert_eq!(
            f.interpret_line(";").unwrap_err().message,
            "NOT IN COMPILATION STATE"
        );
        assert_eq!(
            f.interpret_line("RECURSE").unwrap_err().message,
            "RECURSE ONLY IN COMPILATION"
        );
        assert_eq!(
            f.interpret_line(":").unwrap_err().message,
            "WORD NAME EXPECTED"
        );
        assert!(!f.state);
    }
}
