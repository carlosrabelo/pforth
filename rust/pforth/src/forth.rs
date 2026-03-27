use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Write};
use std::path::PathBuf;

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
    pub loaded_modules: HashSet<String>,
    pub last_save_path: Option<String>,
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
            loaded_modules: HashSet::new(),
            last_save_path: None,
            input,
            output,
        };
        f.store_cell(SPAN_ADDR as u32, 0).unwrap();
        f.store_cell(DP_ADDR as u32, INITIAL_DP).unwrap();
        f.store_cell(BASE_ADDR as u32, 10).unwrap();
        f.init_primitives();
        let _ = f.require_module("CORE");
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
        self.define_primitive("BRANCH", Self::branch);
        self.define_primitive("0BRANCH", Self::zero_branch);
        self.define_immediate("IF", Self::word_if);
        self.define_immediate("ELSE", Self::word_else);
        self.define_immediate("THEN", Self::word_then);
        self.define_immediate("BEGIN", Self::word_begin);
        self.define_immediate("UNTIL", Self::word_until);
        self.define_immediate("AGAIN", Self::word_again);
        self.define_immediate("WHILE", Self::word_while);
        self.define_immediate("REPEAT", Self::word_repeat);
        self.define_primitive("(DO)", Self::do_run);
        self.define_primitive("(LOOP)", Self::loop_run);
        self.define_primitive("(+LOOP)", Self::plus_loop_run);
        self.define_primitive("(LEAVE)", Self::leave_run);
        self.define_primitive("I", Self::word_i);
        self.define_primitive("J", Self::word_j);
        self.define_primitive("UNLOOP", Self::unloop);
        self.define_immediate("DO", Self::word_do);
        self.define_immediate("LOOP", Self::word_loop);
        self.define_immediate("+LOOP", Self::word_plus_loop);
        self.define_immediate("LEAVE", Self::word_leave);
        self.define_primitive(".", Self::dot);
        self.define_primitive("U.", Self::u_dot);
        self.define_primitive(".S", Self::dot_s);
        self.define_primitive("TYPE", Self::type_word);
        self.define_immediate(".\"", Self::dot_quote);
        self.define_immediate("S\"", Self::s_quote);
        self.define_primitive("DROP", Self::drop);
        self.define_primitive(">R", Self::to_r);
        self.define_primitive("R>", Self::r_from);
        self.define_primitive("-", Self::minus);
        self.define_primitive("/MOD", Self::slash_mod);
        self.define_primitive("OR", Self::or);
        self.define_primitive("=", Self::equals);
        self.define_primitive("<", Self::less);
        self.define_primitive(">", Self::greater);
        self.define_primitive("@", Self::word_fetch);
        self.define_primitive("!", Self::word_store);
        self.define_primitive("FILL", Self::fill);
        self.define_primitive("EMIT", Self::emit);
        self.define_primitive("CR", Self::cr);
        self.define_primitive("RUNNING", Self::running_word);
        self.define_primitive("LOAD-FILE", Self::load_file_word);
        self.define_primitive("SAVE-FILE", Self::save_file_word);
        self.define_primitive("WORDS", Self::list_words);
        self.define_primitive("DUMP", Self::dump);
        self.define_primitive("C@", Self::word_c_fetch);
        self.define_primitive("C!", Self::word_c_store);
        self.define_primitive("C,", Self::c_comma);
        self.define_primitive("ALLOT", Self::allot);
        self.define_primitive("CREATE", Self::create);
        self.define_primitive("CONSTANT", Self::constant);
        self.define_primitive("VARIABLE", Self::variable);
        self.define_primitive("#REQUIRE", Self::word_require);
        self.define_primitive("EVALUATE", Self::evaluate);
        self.define_primitive("INCLUDED", Self::included);
        self.define_primitive("INCLUDE", Self::include);
        self.define_immediate("(", Self::paren);
        self.define_immediate("\\", Self::backslash);
    }

    pub fn require_module(&mut self, name: &str) -> Result<(), ForthError> {
        let upper = Self::uppercase_name(name);
        if self.loaded_modules.contains(&upper) {
            return Ok(());
        }
        if let Some(path) = Self::find_lib_path(name) {
            self.loaded_modules.insert(upper);
            return self.load_file(&path);
        }
        Err(ForthError {
            message: format!("MODULE NOT FOUND: {upper}"),
        })
    }

    fn find_lib_path(name: &str) -> Option<String> {
        let mut file = name.to_ascii_lowercase();
        if !file.ends_with(".fs") {
            file.push_str(".fs");
        }
        if let Ok(lib) = std::env::var("PFORTH_LIB") {
            let candidate = PathBuf::from(lib).join(&file);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            let candidate = cwd.join("pforth").join(&file);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
        let mut dir = std::env::current_dir().ok()?;
        loop {
            for sub in ["libs", "demos"] {
                let candidate = dir.join(sub).join(&file);
                if candidate.is_file() {
                    return Some(candidate.to_string_lossy().into_owned());
                }
            }
            if !dir.pop() {
                break;
            }
        }
        None
    }

    pub fn word_require(&mut self) -> Result<(), ForthError> {
        let name = self.parse_word().ok_or_else(|| ForthError {
            message: "MODULE NAME EXPECTED".to_string(),
        })?;
        self.require_module(&name)
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

    fn branch_by(&mut self, offset: Cell) -> Result<(), ForthError> {
        let target = self.ip as isize + offset as isize;
        if target < 0 {
            return Err(ForthError {
                message: "INVALID BRANCH TARGET".to_string(),
            });
        }
        self.ip = target as usize;
        Ok(())
    }

    pub fn branch(&mut self) -> Result<(), ForthError> {
        if self.ip < self.current_body.len() {
            let offset = self.current_body[self.ip];
            self.branch_by(offset)?;
        }
        Ok(())
    }

    pub fn zero_branch(&mut self) -> Result<(), ForthError> {
        let flag = self.pop_ds()?;
        if self.ip < self.current_body.len() {
            if flag == 0 {
                let offset = self.current_body[self.ip];
                self.branch_by(offset)?;
            } else {
                self.ip += 1;
            }
        }
        Ok(())
    }

    fn require_compiling(&self, name: &str) -> Result<(), ForthError> {
        if self.state {
            Ok(())
        } else {
            Err(ForthError {
                message: format!("{name} ONLY IN COMPILATION"),
            })
        }
    }

    fn pop_branch_origin(&mut self) -> Result<usize, ForthError> {
        let index = self.pop_ds()?;
        if index < 0 || index as usize >= self.compile_list.len() {
            return Err(ForthError {
                message: "INVALID BRANCH TARGET".to_string(),
            });
        }
        Ok(index as usize)
    }

    pub fn word_if(&mut self) -> Result<(), ForthError> {
        self.require_compiling("IF")?;
        let zbranch = self.find_xt("0BRANCH").unwrap() as Cell;
        self.compile_list.push(zbranch);
        self.compile_list.push(0);
        self.push_ds((self.compile_list.len() - 1) as Cell);
        Ok(())
    }

    pub fn word_then(&mut self) -> Result<(), ForthError> {
        self.require_compiling("THEN")?;
        let index = self.pop_branch_origin()?;
        self.compile_list[index] = (self.compile_list.len() - index) as Cell;
        Ok(())
    }

    pub fn word_else(&mut self) -> Result<(), ForthError> {
        self.require_compiling("ELSE")?;
        let branch = self.find_xt("BRANCH").unwrap() as Cell;
        self.compile_list.push(branch);
        self.compile_list.push(0);
        let new_origin = self.compile_list.len() - 1;
        let if_origin = self.pop_branch_origin()?;
        self.compile_list[if_origin] = (new_origin + 1 - if_origin) as Cell;
        self.push_ds(new_origin as Cell);
        Ok(())
    }

    fn compile_backward_branch(&mut self, branch_name: &str, dest: Cell) {
        let xt = self.find_xt(branch_name).unwrap() as Cell;
        self.compile_list.push(xt);
        let offset = dest as isize - self.compile_list.len() as isize;
        self.compile_list.push(offset as Cell);
    }

    pub fn word_begin(&mut self) -> Result<(), ForthError> {
        self.require_compiling("BEGIN")?;
        self.push_ds(self.compile_list.len() as Cell);
        Ok(())
    }

    pub fn word_until(&mut self) -> Result<(), ForthError> {
        self.require_compiling("UNTIL")?;
        let dest = self.pop_ds()?;
        self.compile_backward_branch("0BRANCH", dest);
        Ok(())
    }

    pub fn word_again(&mut self) -> Result<(), ForthError> {
        self.require_compiling("AGAIN")?;
        let dest = self.pop_ds()?;
        self.compile_backward_branch("BRANCH", dest);
        Ok(())
    }

    pub fn word_while(&mut self) -> Result<(), ForthError> {
        self.require_compiling("WHILE")?;
        let dest = self.pop_ds()?;
        let zbranch = self.find_xt("0BRANCH").unwrap() as Cell;
        self.compile_list.push(zbranch);
        self.compile_list.push(0);
        self.push_ds(dest);
        self.push_ds((self.compile_list.len() - 1) as Cell);
        Ok(())
    }

    pub fn word_repeat(&mut self) -> Result<(), ForthError> {
        self.require_compiling("REPEAT")?;
        let origin = self.pop_ds()?;
        let dest = self.pop_ds()?;
        self.compile_backward_branch("BRANCH", dest);
        if origin < 0 || origin as usize >= self.compile_list.len() {
            return Err(ForthError {
                message: "INVALID BRANCH TARGET".to_string(),
            });
        }
        let origin = origin as usize;
        self.compile_list[origin] = (self.compile_list.len() - origin) as Cell;
        Ok(())
    }

    pub fn do_run(&mut self) -> Result<(), ForthError> {
        let start = self.pop_ds()?;
        let limit = self.pop_ds()?;
        self.push_rs(limit);
        self.push_rs(start);
        Ok(())
    }

    fn loop_back_or_skip(
        &mut self,
        leave: bool,
        limit: Cell,
        index: Cell,
    ) -> Result<(), ForthError> {
        if leave {
            self.ip += 1;
            return Ok(());
        }
        self.push_rs(limit);
        self.push_rs(index);
        self.branch()
    }

    pub fn loop_run(&mut self) -> Result<(), ForthError> {
        let index = self.pop_rs()?.wrapping_add(1);
        let limit = self.pop_rs()?;
        self.loop_back_or_skip(index >= limit, limit, index)
    }

    pub fn plus_loop_run(&mut self) -> Result<(), ForthError> {
        let step = self.pop_ds()?;
        let index = self.pop_rs()?.wrapping_add(step);
        let limit = self.pop_rs()?;
        let done = (step > 0 && index >= limit) || (step < 0 && index <= limit);
        self.loop_back_or_skip(done, limit, index)
    }

    pub fn leave_run(&mut self) -> Result<(), ForthError> {
        let len = self.rs.len();
        if len >= 2 {
            self.rs[len - 1] = self.rs[len - 2];
        }
        Ok(())
    }

    pub fn word_i(&mut self) -> Result<(), ForthError> {
        let index = self.peek_rs()?;
        self.push_ds(index);
        Ok(())
    }

    pub fn word_j(&mut self) -> Result<(), ForthError> {
        if self.rs.len() < 4 {
            return Err(ForthError {
                message: "J: NOT ENOUGH LOOP LEVELS".to_string(),
            });
        }
        let outer = self.rs[self.rs.len() - 3];
        self.push_ds(outer);
        Ok(())
    }

    pub fn unloop(&mut self) -> Result<(), ForthError> {
        if self.rs.len() < 2 {
            return Err(ForthError {
                message: "RETURN STACK UNDERFLOW".to_string(),
            });
        }
        self.rs.pop();
        self.rs.pop();
        Ok(())
    }

    pub fn word_do(&mut self) -> Result<(), ForthError> {
        self.require_compiling("DO")?;
        let xt = self.find_xt("(DO)").unwrap() as Cell;
        self.compile_list.push(xt);
        self.push_ds(self.compile_list.len() as Cell);
        Ok(())
    }

    pub fn word_loop(&mut self) -> Result<(), ForthError> {
        self.require_compiling("LOOP")?;
        let dest = self.pop_ds()?;
        self.compile_backward_branch("(LOOP)", dest);
        Ok(())
    }

    pub fn word_plus_loop(&mut self) -> Result<(), ForthError> {
        self.require_compiling("+LOOP")?;
        let dest = self.pop_ds()?;
        self.compile_backward_branch("(+LOOP)", dest);
        Ok(())
    }

    pub fn word_leave(&mut self) -> Result<(), ForthError> {
        self.require_compiling("LEAVE")?;
        let xt = self.find_xt("(LEAVE)").unwrap() as Cell;
        self.compile_list.push(xt);
        Ok(())
    }

    fn write_text(&mut self, text: &str) -> Result<(), ForthError> {
        self.output
            .write_all(text.as_bytes())
            .map_err(|e| ForthError {
                message: e.to_string(),
            })
    }

    fn format_cell(val: Cell, base: i32, signed: bool) -> String {
        let base = if (2..=36).contains(&base) { base } else { 10 } as u32;
        let negative = signed && val < 0;
        let mut magnitude: u32 = if signed {
            (val as i32).unsigned_abs()
        } else {
            val as u16 as u32
        };
        if magnitude == 0 {
            return "0".to_string();
        }
        let mut digits = Vec::new();
        while magnitude > 0 {
            digits.push(
                std::char::from_digit(magnitude % base, base)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
            magnitude /= base;
        }
        if negative {
            digits.push('-');
        }
        digits.iter().rev().collect()
    }

    pub fn dot(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        let text = Self::format_cell(n, self.get_base()?, true);
        self.write_text(&format!("{text} "))
    }

    pub fn u_dot(&mut self) -> Result<(), ForthError> {
        let n = self.pop_ds()?;
        let text = Self::format_cell(n, self.get_base()?, false);
        self.write_text(&format!("{text} "))
    }

    pub fn dot_s(&mut self) -> Result<(), ForthError> {
        let base = self.get_base()?;
        let mut text = format!("<{}>", self.ds.len());
        for &cell in &self.ds {
            text.push(' ');
            text.push_str(&Self::format_cell(cell, base, true));
        }
        text.push(' ');
        self.write_text(&text)
    }

    pub fn page(&mut self) -> Result<(), ForthError> {
        self.write_text("\x1b[2J\x1b[H")
    }

    fn parse_string(&mut self) -> Vec<u8> {
        let bytes = self.tib.as_bytes();
        let mut pos = self.in_ptr;
        if pos < bytes.len() && bytes[pos] == b' ' {
            pos += 1;
        }
        let start = pos;
        while pos < bytes.len() && bytes[pos] != b'"' {
            pos += 1;
        }
        let text = bytes[start..pos].to_vec();
        if pos < bytes.len() {
            pos += 1;
        }
        self.in_ptr = pos;
        text
    }

    fn store_string(&mut self, text: &[u8]) -> Result<Cell, ForthError> {
        let addr = self.here_value()?;
        for (i, &b) in text.iter().enumerate() {
            self.c_store(addr as u16 as u32 + i as u32, b)?;
        }
        self.store_cell(DP_ADDR as u32, addr.wrapping_add(text.len() as Cell))?;
        Ok(addr)
    }

    fn compile_string_literal(&mut self, addr: Cell, len: usize) {
        let lit = self.find_xt("LIT").unwrap() as Cell;
        self.compile_list.extend([lit, addr, lit, len as Cell]);
    }

    pub fn dot_quote(&mut self) -> Result<(), ForthError> {
        let text = self.parse_string();
        if self.state {
            let addr = self.store_string(&text)?;
            self.compile_string_literal(addr, text.len());
            let type_xt = self.find_xt("TYPE").unwrap() as Cell;
            self.compile_list.push(type_xt);
            Ok(())
        } else {
            self.output.write_all(&text).map_err(|e| ForthError {
                message: e.to_string(),
            })
        }
    }

    pub fn s_quote(&mut self) -> Result<(), ForthError> {
        let text = self.parse_string();
        let addr = self.store_string(&text)?;
        if self.state {
            self.compile_string_literal(addr, text.len());
        } else {
            self.push_ds(addr);
            self.push_ds(text.len() as Cell);
        }
        Ok(())
    }

    pub fn paren(&mut self) -> Result<(), ForthError> {
        let bytes = self.tib.as_bytes();
        let mut pos = self.in_ptr;
        while pos < bytes.len() && bytes[pos] != b')' {
            pos += 1;
        }
        if pos < bytes.len() {
            pos += 1;
        }
        self.in_ptr = pos;
        Ok(())
    }

    pub fn backslash(&mut self) -> Result<(), ForthError> {
        self.in_ptr = self.tib.len();
        Ok(())
    }

    fn pop_text(&mut self) -> Result<String, ForthError> {
        let len = self.pop_ds()?.max(0) as usize;
        let addr = self.pop_ds()? as u16 as usize;
        let bytes: Vec<u8> = (0..len).map(|i| self.memory[(addr + i) & 0xffff]).collect();
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn evaluate(&mut self) -> Result<(), ForthError> {
        let text = self.pop_text()?;
        let saved_tib = std::mem::replace(&mut self.tib, text);
        let saved_in_ptr = std::mem::replace(&mut self.in_ptr, 0);
        let result = self.interpret();
        self.tib = saved_tib;
        self.in_ptr = saved_in_ptr;
        result
    }

    pub fn load_system(&mut self, text: &str) -> Result<(), ForthError> {
        let saved_tib = self.tib.clone();
        let saved_in_ptr = self.in_ptr;
        let result = (|| {
            for line in text.lines() {
                self.check_interrupt()?;
                let trimmed = line.trim_start();
                if trimmed.is_empty() || trimmed.starts_with('\\') {
                    continue;
                }
                if let Some(rest) = trimmed.strip_prefix("#REQUIRE") {
                    if let Some(name) = rest.split_whitespace().next() {
                        self.require_module(name)?;
                    }
                    continue;
                }
                self.interpret_line(line)?;
            }
            Ok(())
        })();
        self.tib = saved_tib;
        self.in_ptr = saved_in_ptr;
        result
    }

    pub fn load_file(&mut self, path: &str) -> Result<(), ForthError> {
        let bytes = std::fs::read(path).map_err(|_| ForthError {
            message: format!("CANNOT OPEN: {path}"),
        })?;
        self.load_system(&String::from_utf8_lossy(&bytes))
    }

    pub fn included(&mut self) -> Result<(), ForthError> {
        let path = self.pop_text()?;
        self.load_file(&path)
    }

    pub fn include(&mut self) -> Result<(), ForthError> {
        let path = self.parse_word().ok_or_else(|| ForthError {
            message: "FILE NAME EXPECTED".to_string(),
        })?;
        self.load_file(&path)
    }

    pub fn load_file_word(&mut self) -> Result<(), ForthError> {
        let path = self.parse_word().ok_or_else(|| ForthError {
            message: "FILE NAME EXPECTED".to_string(),
        })?;
        let maxlen = self.pop_ds()?;
        let addr = self.pop_ds()? as u16 as usize;
        self.last_save_path = Some(path.clone());
        if maxlen <= 0 {
            self.push_ds(0);
            return Ok(());
        }
        let bytes = std::fs::read(&path).map_err(|_| ForthError {
            message: format!("CANNOT OPEN: {path}"),
        })?;
        let mut n = bytes.len().min(maxlen as usize);
        n = n.min(self.memory.len() - addr);
        self.memory[addr..addr + n].copy_from_slice(&bytes[..n]);
        self.push_ds(n as Cell);
        Ok(())
    }

    pub fn save_file_word(&mut self) -> Result<(), ForthError> {
        let path = self.last_save_path.clone().ok_or_else(|| ForthError {
            message: "EDITOR FILE".to_string(),
        })?;
        let len = self.pop_ds()?;
        let addr = self.pop_ds()? as u16 as usize;
        let n = if len <= 0 {
            0
        } else {
            (len as usize).min(self.memory.len() - addr)
        };
        std::fs::write(&path, &self.memory[addr..addr + n]).map_err(|_| ForthError {
            message: format!("CANNOT OPEN: {path}"),
        })?;
        Ok(())
    }

    pub fn running_word(&mut self) -> Result<(), ForthError> {
        self.push_ds(Self::flag(self.running));
        Ok(())
    }

    pub fn list_words(&mut self) -> Result<(), ForthError> {
        let names: Vec<String> = self
            .words
            .iter()
            .rev()
            .map(|w| {
                if w.immediate {
                    format!("I:{}", w.name)
                } else {
                    w.name.clone()
                }
            })
            .collect();
        self.write_text(&format!(
            "{}
",
            names.join(" ")
        ))
    }

    pub fn dump(&mut self) -> Result<(), ForthError> {
        let len = self.pop_ds()?;
        let mut addr = self.pop_ds()? as u16 as usize;
        if len <= 0 {
            return Ok(());
        }
        let mut remaining = len as usize;
        while remaining > 0 {
            if addr > 65535 {
                return Err(ForthError {
                    message: "OUT OF MEMORY RANGE".to_string(),
                });
            }
            let want = remaining.min(16);
            let fit = want.min(65536 - addr);
            let bytes = &self.memory[addr..addr + fit];
            let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02X}")).collect();
            let ascii: String = bytes
                .iter()
                .map(|&b| {
                    if (32..=126).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            let row = format!(
                "{addr:04X}  {}  {ascii}
",
                hex.join(" ")
            );
            self.write_text(&row)?;
            addr += fit;
            remaining -= fit;
            if fit < want {
                return Err(ForthError {
                    message: "OUT OF MEMORY RANGE".to_string(),
                });
            }
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub interactive: bool,
    pub file_paths: Vec<String>,
}

pub fn parse_args(args: &[String]) -> Config {
    Config {
        interactive: args.len() == 1,
        file_paths: args.iter().skip(1).cloned().collect(),
    }
}

pub fn run<R: BufRead, W: Write, E: Write>(cfg: &Config, input: R, output: W, mut err: E) -> i32 {
    let mut forth = Forth::new(input, output);
    for path in &cfg.file_paths {
        if let Err(e) = forth.load_file(path) {
            let _ = writeln!(err, "{}", e.message);
            return 1;
        }
    }
    if cfg.interactive {
        if let Err(e) = forth.quit() {
            let _ = writeln!(err, "{}", e.message);
            return 1;
        }
    }
    0
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

    #[test]
    fn branch_and_zero_branch() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        let lit = f.find_xt("LIT").unwrap() as Cell;
        let branch = f.find_xt("BRANCH").unwrap() as Cell;
        let zbranch = f.find_xt("0BRANCH").unwrap() as Cell;
        let exit = f.find_xt("EXIT").unwrap() as Cell;
        let dup = f.find_xt("DUP").unwrap() as Cell;
        let plus = f.find_xt("+").unwrap() as Cell;

        let fwd = f.define_word(
            "TESTBRANCH",
            WordKind::Colon,
            None,
            vec![lit, 10, branch, 3, lit, 999, lit, 20, exit],
            false,
        );
        f.push_ds(fwd as Cell);
        f.execute().unwrap();
        assert_eq!(f.ds, vec![10, 20]);
        f.ds.clear();

        let zero = f.define_word(
            "TESTZBRANCH",
            WordKind::Colon,
            None,
            vec![zbranch, 3, lit, 999, lit, 42, exit],
            false,
        );
        f.push_ds(0);
        f.push_ds(zero as Cell);
        f.execute().unwrap();
        assert_eq!(f.ds, vec![42]);
        f.ds.clear();

        f.push_ds(-1);
        f.push_ds(zero as Cell);
        f.execute().unwrap();
        assert_eq!(f.ds, vec![999, 42]);
        f.ds.clear();

        f.push_ds(zero as Cell);
        assert_eq!(f.execute().unwrap_err().message, "DATA STACK UNDERFLOW");

        let countdown = f.define_word(
            "TESTLOOP",
            WordKind::Colon,
            None,
            vec![
                lit, 3, dup, lit, -1, plus, dup, zbranch, 3, branch, -8, exit,
            ],
            false,
        );
        f.ds.clear();
        f.push_ds(countdown as Cell);
        f.execute().unwrap();
        assert_eq!(f.ds, vec![3, 2, 1, 0]);

        let bad = f.define_word("TESTBAD", WordKind::Colon, None, vec![branch, -5], false);
        f.push_ds(bad as Cell);
        assert_eq!(f.execute().unwrap_err().message, "INVALID BRANCH TARGET");
    }

    #[test]
    fn if_else_then() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.interpret_line(": PICK1 IF 11 THEN 99 ;").unwrap();
        f.interpret_line("-1 PICK1").unwrap();
        assert_eq!(f.ds, vec![11, 99]);
        f.ds.clear();
        f.interpret_line("0 PICK1").unwrap();
        assert_eq!(f.ds, vec![99]);
        f.ds.clear();

        f.interpret_line(": PICK2 IF 1 ELSE 2 THEN 3 ;").unwrap();
        f.interpret_line("-1 PICK2").unwrap();
        assert_eq!(f.ds, vec![1, 3]);
        f.ds.clear();
        f.interpret_line("0 PICK2").unwrap();
        assert_eq!(f.ds, vec![2, 3]);
        f.ds.clear();

        f.interpret_line(": NEST IF IF 1 ELSE 2 THEN ELSE 3 THEN ;")
            .unwrap();
        f.interpret_line("-1 -1 NEST").unwrap();
        assert_eq!(f.ds, vec![1]);
        f.ds.clear();
        f.interpret_line("0 -1 NEST").unwrap();
        assert_eq!(f.ds, vec![2]);
        f.ds.clear();
        f.interpret_line("0 NEST").unwrap();
        assert_eq!(f.ds, vec![3]);
        f.ds.clear();

        let zbranch = f.find_xt("0BRANCH").unwrap() as Cell;
        let branch = f.find_xt("BRANCH").unwrap() as Cell;
        f.interpret_line(": SHAPE IF 1 ELSE 2 THEN ;").unwrap();
        let xt = f.find_xt("SHAPE").unwrap();
        let lit = f.find_xt("LIT").unwrap() as Cell;
        let exit = f.find_xt("EXIT").unwrap() as Cell;
        assert_eq!(
            f.words[xt].body,
            vec![zbranch, 5, lit, 1, branch, 3, lit, 2, exit]
        );

        assert_eq!(
            f.interpret_line("IF").unwrap_err().message,
            "IF ONLY IN COMPILATION"
        );
        assert_eq!(
            f.interpret_line("ELSE").unwrap_err().message,
            "ELSE ONLY IN COMPILATION"
        );
        assert_eq!(
            f.interpret_line("THEN").unwrap_err().message,
            "THEN ONLY IN COMPILATION"
        );

        f.state = true;
        f.compile_list.clear();
        f.push_ds(50);
        assert_eq!(f.word_then().unwrap_err().message, "INVALID BRANCH TARGET");
        f.push_ds(-1);
        assert_eq!(f.word_else().unwrap_err().message, "INVALID BRANCH TARGET");
    }

    #[test]
    fn begin_until_again_while_repeat() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);
        f.define_primitive("=", Forth::equals);
        f.define_primitive("<", Forth::less);

        f.interpret_line(": UP3 0 BEGIN 1 + DUP 3 = UNTIL ;")
            .unwrap();
        f.interpret_line("UP3").unwrap();
        assert_eq!(f.ds, vec![3]);
        f.ds.clear();

        f.interpret_line(": FOREVER 0 BEGIN 1 + DUP 4 = IF EXIT THEN AGAIN ;")
            .unwrap();
        f.interpret_line("FOREVER").unwrap();
        assert_eq!(f.ds, vec![4]);
        f.ds.clear();

        f.interpret_line(": FILL5 0 BEGIN DUP 5 < WHILE 1 + REPEAT ;")
            .unwrap();
        f.interpret_line("FILL5").unwrap();
        assert_eq!(f.ds, vec![5]);
        f.ds.clear();

        let zbranch = f.find_xt("0BRANCH").unwrap() as Cell;
        let branch = f.find_xt("BRANCH").unwrap() as Cell;
        let exit = f.find_xt("EXIT").unwrap() as Cell;
        f.interpret_line(": SPIN BEGIN AGAIN ;").unwrap();
        let spin = f.find_xt("SPIN").unwrap();
        assert_eq!(f.words[spin].body, vec![branch, -1, exit]);
        f.interpret_line(": WAIT BEGIN UNTIL ;").unwrap();
        let wait = f.find_xt("WAIT").unwrap();
        assert_eq!(f.words[wait].body, vec![zbranch, -1, exit]);
        f.interpret_line(": LOOPED BEGIN 0 WHILE REPEAT ;").unwrap();
        let looped = f.find_xt("LOOPED").unwrap();
        let lit = f.find_xt("LIT").unwrap() as Cell;
        assert_eq!(
            f.words[looped].body,
            vec![lit, 0, zbranch, 3, branch, -5, exit]
        );

        for (word, message) in [
            ("BEGIN", "BEGIN ONLY IN COMPILATION"),
            ("UNTIL", "UNTIL ONLY IN COMPILATION"),
            ("AGAIN", "AGAIN ONLY IN COMPILATION"),
            ("WHILE", "WHILE ONLY IN COMPILATION"),
            ("REPEAT", "REPEAT ONLY IN COMPILATION"),
        ] {
            assert_eq!(f.interpret_line(word).unwrap_err().message, message);
        }

        f.state = true;
        f.compile_list.clear();
        f.push_ds(0);
        f.push_ds(90);
        assert_eq!(
            f.word_repeat().unwrap_err().message,
            "INVALID BRANCH TARGET"
        );
    }

    #[test]
    fn do_loop_i_j_leave_unloop() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);
        f.define_primitive("=", Forth::equals);

        let run = |f: &mut Forth<Cursor<Vec<u8>>, &mut Vec<u8>>, line: &str| {
            f.interpret_line(line).unwrap();
            let out = f.ds.clone();
            f.ds.clear();
            out
        };

        f.interpret_line(": SUM 0 5 0 DO I + LOOP ;").unwrap();
        assert_eq!(run(&mut f, "SUM"), vec![10]);
        assert!(f.rs.is_empty());

        f.interpret_line(": SKIP2 0 10 0 DO I + 2 +LOOP ;").unwrap();
        assert_eq!(run(&mut f, "SKIP2"), vec![20]);

        f.interpret_line(": DOWN3 0 0 10 DO I + -3 +LOOP ;")
            .unwrap();
        assert_eq!(run(&mut f, "DOWN3"), vec![22]);

        f.interpret_line(": NESTED 0 3 0 DO 2 0 DO J + LOOP LOOP ;")
            .unwrap();
        assert_eq!(run(&mut f, "NESTED"), vec![6]);
        assert!(f.rs.is_empty());

        f.interpret_line(": EARLY 0 10 0 DO I + I 3 = IF LEAVE THEN LOOP ;")
            .unwrap();
        assert_eq!(run(&mut f, "EARLY"), vec![6]);
        assert!(f.rs.is_empty());

        f.interpret_line(": ESCAPE 10 0 DO I 2 = IF I UNLOOP EXIT THEN LOOP 99 ;")
            .unwrap();
        assert_eq!(run(&mut f, "ESCAPE"), vec![2]);
        assert!(f.rs.is_empty());

        let do_xt = f.find_xt("(DO)").unwrap() as Cell;
        let loop_xt = f.find_xt("(LOOP)").unwrap() as Cell;
        let exit = f.find_xt("EXIT").unwrap() as Cell;
        let lit = f.find_xt("LIT").unwrap() as Cell;
        f.interpret_line(": EMPTY 3 0 DO LOOP ;").unwrap();
        let empty = f.find_xt("EMPTY").unwrap();
        assert_eq!(
            f.words[empty].body,
            vec![lit, 3, lit, 0, do_xt, loop_xt, -1, exit]
        );

        f.rs.clear();
        assert_eq!(
            f.interpret_line("I").unwrap_err().message,
            "RETURN STACK UNDERFLOW"
        );
        f.rs.extend([5, 0]);
        assert_eq!(
            f.interpret_line("J").unwrap_err().message,
            "J: NOT ENOUGH LOOP LEVELS"
        );
        f.interpret_line("UNLOOP").unwrap();
        assert!(f.rs.is_empty());
        assert_eq!(
            f.interpret_line("UNLOOP").unwrap_err().message,
            "RETURN STACK UNDERFLOW"
        );
        f.rs.push(1);
        f.interpret_line("(LEAVE)").unwrap();
        assert_eq!(f.rs, vec![1]);
        f.rs.clear();

        for (word, message) in [
            ("DO", "DO ONLY IN COMPILATION"),
            ("LOOP", "LOOP ONLY IN COMPILATION"),
            ("+LOOP", "+LOOP ONLY IN COMPILATION"),
            ("LEAVE", "LEAVE ONLY IN COMPILATION"),
        ] {
            assert_eq!(f.interpret_line(word).unwrap_err().message, message);
        }
    }

    #[test]
    fn numeric_output_and_dot_s() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        {
            let mut f = Forth::new(input, &mut output);

            f.interpret_line("42 .").unwrap();
            f.interpret_line("-1 U.").unwrap();
            f.interpret_line("-7 . 0 . 0 U.").unwrap();
            f.interpret_line("-32768 . 32767 U.").unwrap();

            f.store_cell(BASE_ADDR as u32, 16).unwrap();
            f.interpret_line("FF . -1 U. -FF .").unwrap();
            f.store_cell(BASE_ADDR as u32, 2).unwrap();
            f.interpret_line("101 . -1 U.").unwrap();
            f.store_cell(BASE_ADDR as u32, 36).unwrap();
            f.interpret_line("-1 U.").unwrap();
            f.store_cell(BASE_ADDR as u32, 99).unwrap();
            f.interpret_line("12 .").unwrap();
            f.store_cell(BASE_ADDR as u32, 10).unwrap();

            f.interpret_line(".S").unwrap();
            f.interpret_line("1 -2 3 .S").unwrap();
            assert_eq!(f.ds, vec![1, -2, 3]);
            f.ds.clear();

            f.page().unwrap();
            assert_eq!(
                f.interpret_line(".").unwrap_err().message,
                "DATA STACK UNDERFLOW"
            );
        }
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "42 65535 -7 0 0 -32768 32767 FF FFFF -FF 101 1111111111111111 1EKF 101 <0> <3> 1 -2 3 \x1b[2J\x1b[H"
        );
    }

    #[test]
    fn strings_and_comments() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        {
            let mut f = Forth::new(input, &mut output);

            f.interpret_line(".\" hello\" 7").unwrap();
            assert_eq!(f.ds, vec![7]);
            f.ds.clear();
            assert_eq!(f.here_value().unwrap(), INITIAL_DP);

            f.interpret_line("S\" abc\"").unwrap();
            assert_eq!(f.ds, vec![INITIAL_DP, 3]);
            assert_eq!(&f.memory[1024..1027], b"abc");
            assert_eq!(f.here_value().unwrap(), INITIAL_DP + 3);
            f.interpret_line("TYPE").unwrap();
            f.interpret_line("S\" \"").unwrap();
            assert_eq!(f.ds, vec![INITIAL_DP + 3, 0]);
            f.ds.clear();

            f.interpret_line(": GREET .\" hi there\" ;").unwrap();
            let greet = f.find_xt("GREET").unwrap();
            let lit = f.find_xt("LIT").unwrap() as Cell;
            let type_xt = f.find_xt("TYPE").unwrap() as Cell;
            let exit = f.find_xt("EXIT").unwrap() as Cell;
            let addr = INITIAL_DP + 3;
            assert_eq!(f.words[greet].body, vec![lit, addr, lit, 8, type_xt, exit]);
            assert_eq!(&f.memory[addr as usize..addr as usize + 8], b"hi there");
            f.interpret_line("GREET").unwrap();

            f.interpret_line(": NAME S\" xyz\" ;").unwrap();
            f.interpret_line("NAME").unwrap();
            assert_eq!(f.ds, vec![addr + 8, 3]);
            f.ds.clear();

            f.interpret_line("1 ( skipped words ) 2").unwrap();
            assert_eq!(f.ds, vec![1, 2]);
            f.ds.clear();
            f.interpret_line("3 ( unterminated 4").unwrap();
            assert_eq!(f.ds, vec![3]);
            f.ds.clear();
            f.interpret_line(": PAREN 5 ( inside ) 6 ;").unwrap();
            f.interpret_line("PAREN").unwrap();
            assert_eq!(f.ds, vec![5, 6]);
            f.ds.clear();

            f.interpret_line("8 \\ 9 10").unwrap();
            assert_eq!(f.ds, vec![8]);
            assert_eq!(f.in_ptr, f.tib.len());
        }
        assert_eq!(String::from_utf8(output).unwrap(), "helloabchi there");
    }

    #[test]
    fn evaluate_include_and_missing_file() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        f.interpret_line("S\" 1 2 +\" EVALUATE 10 *").unwrap();
        assert_eq!(f.ds, vec![30]);
        f.ds.clear();

        f.interpret_line(": ADDED S\" 4 5 +\" EVALUATE ;").unwrap();
        f.interpret_line("ADDED").unwrap();
        assert_eq!(f.ds, vec![9]);
        f.ds.clear();

        f.tib = "keep this".to_string();
        f.in_ptr = 4;
        let addr = f.here_value().unwrap();
        for (i, b) in b"1 NOPE".iter().enumerate() {
            f.memory[addr as usize + i] = *b;
        }
        f.push_ds(addr);
        f.push_ds(6);
        assert_eq!(f.evaluate().unwrap_err().message, "UNKNOWN WORD: NOPE");
        assert_eq!(f.tib, "keep this");
        assert_eq!(f.in_ptr, 4);
        f.ds.clear();

        let path = std::env::temp_dir().join(format!("pforth_include_{}.fs", std::process::id()));
        std::fs::write(
            &path,
            "\\ a comment line\n\n  \\ indented comment\n: SQ DUP * ;\n5 SQ\n",
        )
        .unwrap();
        let name = path.to_str().unwrap().to_string();

        f.tib = "outer".to_string();
        f.in_ptr = 2;
        f.interpret_line(&format!("INCLUDE {name}")).unwrap();
        assert_eq!(f.ds, vec![25]);
        f.ds.clear();

        f.tib = "outer".to_string();
        f.in_ptr = 2;
        f.load_file(&name).unwrap();
        assert_eq!(f.tib, "outer");
        assert_eq!(f.in_ptr, 2);
        f.ds.clear();

        let addr = f.here_value().unwrap();
        for (i, b) in name.bytes().enumerate() {
            f.memory[addr as usize + i] = b;
        }
        f.push_ds(addr);
        f.push_ds(name.len() as Cell);
        f.included().unwrap();
        assert_eq!(f.ds, vec![25]);
        f.ds.clear();
        std::fs::remove_file(&path).unwrap();

        let missing = "/no/such/dir/missing.fs";
        assert_eq!(
            f.interpret_line(&format!("INCLUDE {missing}"))
                .unwrap_err()
                .message,
            format!("CANNOT OPEN: {missing}")
        );
        assert_eq!(
            f.load_file(missing).unwrap_err().message,
            format!("CANNOT OPEN: {missing}")
        );
        assert_eq!(
            f.interpret_line("INCLUDE").unwrap_err().message,
            "FILE NAME EXPECTED"
        );
        assert_eq!(
            f.interpret_line(&format!("S\" {missing}\" INCLUDED"))
                .unwrap_err()
                .message,
            format!("CANNOT OPEN: {missing}")
        );
    }

    #[test]
    fn require_core_colon_words() {
        let input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let mut f = Forth::new(input, &mut output);

        assert!(f.loaded_modules.contains("CORE"));
        for name in [
            "NIP", "TUCK", "ROT", "-ROT", "2DUP", "2DROP", "/", "MOD", "0=", "<>", "0<>", "0<",
            "0>", "<=", ">=", "1+", "1-", "2+", "2-", "2*", "2/", "NEGATE", "+!", "ABS", "MIN",
            "MAX", "SPACE", "SPACES", "?", "PAGE", "ERASE",
        ] {
            let xt = f.find_xt(name).unwrap();
            assert_eq!(f.words[xt].kind, WordKind::Colon, "{name}");
        }

        let run = |f: &mut Forth<Cursor<Vec<u8>>, &mut Vec<u8>>, line: &str| {
            f.interpret_line(line).unwrap();
            let out = f.ds.clone();
            f.ds.clear();
            out
        };
        assert_eq!(run(&mut f, "1 2 NIP"), vec![2]);
        assert_eq!(run(&mut f, "1 2 TUCK"), vec![2, 1, 2]);
        assert_eq!(run(&mut f, "1 2 3 ROT"), vec![2, 3, 1]);
        assert_eq!(run(&mut f, "1 2 3 -ROT"), vec![3, 1, 2]);
        assert_eq!(run(&mut f, "1 2 2DUP"), vec![1, 2, 1, 2]);
        assert_eq!(run(&mut f, "1 2 2DROP"), Vec::<Cell>::new());
        assert_eq!(run(&mut f, "-7 2 /"), vec![-3]);
        assert_eq!(run(&mut f, "-7 2 MOD"), vec![-1]);
        assert_eq!(run(&mut f, "0 0= 5 0= 3 0<> 0 0<>"), vec![-1, 0, -1, 0]);
        assert_eq!(run(&mut f, "-1 0< 1 0< 1 0> 0 0>"), vec![-1, 0, -1, 0]);
        assert_eq!(run(&mut f, "1 2 <> 2 2 <>"), vec![-1, 0]);
        assert_eq!(run(&mut f, "1 2 <= 2 2 <= 3 2 <="), vec![-1, -1, 0]);
        assert_eq!(run(&mut f, "1 2 >= 2 2 >= 3 2 >="), vec![0, -1, -1]);
        assert_eq!(
            run(&mut f, "5 1+ 5 1- 5 2+ 5 2- 5 2*"),
            vec![6, 4, 7, 3, 10]
        );
        assert_eq!(run(&mut f, "-9 2/ 7 NEGATE"), vec![-4, -7]);
        assert_eq!(run(&mut f, "-5 ABS 5 ABS"), vec![5, 5]);
        assert_eq!(
            run(&mut f, "3 9 MIN 9 3 MIN 3 9 MAX 9 3 MAX"),
            vec![3, 3, 9, 9]
        );
        assert_eq!(run(&mut f, "2000 DUP 10 SWAP ! 5 SWAP +! 2000 @"), vec![15]);
        assert_eq!(run(&mut f, "2000 3 SPACES -1 SPACES"), vec![2000]);
        assert_eq!(run(&mut f, "2000 ?"), vec![]);
        f.interpret_line("PAGE").unwrap();
        f.memory[3000] = 7;
        f.memory[3001] = 7;
        assert_eq!(run(&mut f, "3000 2 ERASE 3000 @"), vec![0]);

        let dir = std::env::temp_dir().join(format!("pforth_req_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("libs")).unwrap();
        std::fs::create_dir_all(dir.join("demos")).unwrap();
        std::fs::create_dir_all(dir.join("work/deeper")).unwrap();
        std::fs::write(dir.join("libs/twice.fs"), ": TWICE DUP + ;\n").unwrap();
        std::fs::write(
            dir.join("demos/other.fs"),
            "#REQUIRE twice\n: FOURFOLD TWICE TWICE ;\n",
        )
        .unwrap();
        let saved_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.join("work/deeper")).unwrap();

        let run2 = |f: &mut Forth<Cursor<Vec<u8>>, &mut Vec<u8>>, line: &str| {
            let result = f.interpret_line(line);
            let out = f.ds.clone();
            f.ds.clear();
            (result, out)
        };
        let (r, _) = run2(&mut f, "#REQUIRE other.fs");
        r.unwrap();
        assert_eq!(run2(&mut f, "3 FOURFOLD"), (Ok(()), vec![12]));
        assert_eq!(run2(&mut f, "#REQUIRE OTHER.FS"), (Ok(()), vec![]));
        let (r, _) = run2(&mut f, "#REQUIRE nothere");
        let missing = r.unwrap_err().message;
        let (r, _) = run2(&mut f, "#REQUIRE");
        let no_name = r.unwrap_err().message;

        std::env::set_current_dir(saved_dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(missing, "MODULE NOT FOUND: NOTHERE");
        assert_eq!(no_name, "MODULE NAME EXPECTED");
        assert!(f.loaded_modules.contains("TWICE"));
        assert!(f.loaded_modules.contains("OTHER.FS"));
    }
}
