package forth

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"os/signal"
	"strings"
	"sync/atomic"
	"syscall"
)

const (
	SpanAddr = 1000
	DPAddr   = 1002
	BaseAddr = 1004
)

// Forth represents the Forth interpreter and virtual machine state.
type Forth struct {
	DS          []Cell
	RS          []Cell
	Memory      []byte
	Words       []*Word
	wordsByName map[string]int
	Latest      int
	DP          int
	Base        int
	State       bool

	TIB []byte
	IN  int

	Input   io.Reader
	Output  io.Writer
	bufIn   *bufio.Reader
	Running bool

	Loaded      map[string]bool
	Interrupted int32
}

// New creates and initializes a new Forth engine instance.
func New(input io.Reader, output io.Writer) *Forth {
	f := &Forth{
		DS:          make([]Cell, 0, 64),
		RS:          make([]Cell, 0, 64),
		Memory:      make([]byte, 65536),
		Words:       make([]*Word, 0, 128),
		wordsByName: make(map[string]int),
		Latest:      -1,
		DP:          1024,
		Base:        10,
		TIB:         make([]byte, 0, 256),
		IN:          0,
		Input:       input,
		Output:      output,
		bufIn:       bufio.NewReader(input),
		Running:     true,
		Loaded:      make(map[string]bool),
	}
	f.Store(SpanAddr, 0)
	f.Store(DPAddr, 1024)
	f.Store(BaseAddr, 10)
	return f
}

// DSPush pushes a cell onto the data stack.
func (f *Forth) DSPush(v Cell) {
	f.DS = append(f.DS, v)
}

// DSPop pops a cell from the data stack.
func (f *Forth) DSPop() Cell {
	if len(f.DS) == 0 {
		forthError("DATA STACK UNDERFLOW")
	}
	v := f.DS[len(f.DS)-1]
	f.DS = f.DS[:len(f.DS)-1]
	return v
}

// DSPeek returns top of data stack without popping.
func (f *Forth) DSPeek() Cell {
	if len(f.DS) == 0 {
		forthError("DATA STACK UNDERFLOW")
	}
	return f.DS[len(f.DS)-1]
}

// RSPush pushes a cell onto the return stack.
func (f *Forth) RSPush(v Cell) {
	f.RS = append(f.RS, v)
}

// RSPop pops a cell from the return stack.
func (f *Forth) RSPop() Cell {
	if len(f.RS) == 0 {
		forthError("RETURN STACK UNDERFLOW")
	}
	v := f.RS[len(f.RS)-1]
	f.RS = f.RS[:len(f.RS)-1]
	return v
}

// RSPeek returns top of return stack without popping.
func (f *Forth) RSPeek() Cell {
	if len(f.RS) == 0 {
		forthError("RETURN STACK UNDERFLOW")
	}
	return f.RS[len(f.RS)-1]
}

// WordDepth pushes the current depth of the data stack onto the data stack.
func (f *Forth) WordDepth() {
	f.DSPush(Cell(len(f.DS)))
}

// WordDup duplicates top of stack: DUP ( n -- n n )
func (f *Forth) WordDup() {
	f.DSPush(f.DSPeek())
}

// WordDrop removes top of stack: DROP ( n -- )
func (f *Forth) WordDrop() {
	_ = f.DSPop()
}

// WordSwap exchanges top two stack items: SWAP ( a b -- b a )
func (f *Forth) WordSwap() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(b)
	f.DSPush(a)
}

// WordOver copies second item to top of stack: OVER ( a b -- a b a )
func (f *Forth) WordOver() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(a)
	f.DSPush(b)
	f.DSPush(a)
}

// WordQDup duplicates top of stack only if non-zero: ?DUP ( n -- 0 | n n )
func (f *Forth) WordQDup() {
	v := f.DSPeek()
	if v != 0 {
		f.DSPush(v)
	}
}

// WordRot rotates third item to top of stack: ROT ( a b c -- b c a )
func (f *Forth) WordRot() {
	c := f.DSPop()
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(b)
	f.DSPush(c)
	f.DSPush(a)
}

// WordNRot rotates top item to third position: -ROT ( a b c -- c a b )
func (f *Forth) WordNRot() {
	c := f.DSPop()
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(c)
	f.DSPush(a)
	f.DSPush(b)
}

// WordNip drops second item: NIP ( a b -- b )
func (f *Forth) WordNip() {
	b := f.DSPop()
	_ = f.DSPop()
	f.DSPush(b)
}

// WordTuck duplicates top item under second item: TUCK ( a b -- b a b )
func (f *Forth) WordTuck() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(b)
	f.DSPush(a)
	f.DSPush(b)
}

// Word2Dup duplicates top pair of cells: 2DUP ( a b -- a b a b )
func (f *Forth) Word2Dup() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(a)
	f.DSPush(b)
	f.DSPush(a)
	f.DSPush(b)
}

// Word2Drop removes top pair of cells: 2DROP ( a b -- )
func (f *Forth) Word2Drop() {
	_ = f.DSPop()
	_ = f.DSPop()
}

// Word2Swap exchanges top two pairs of cells: 2SWAP ( a b c d -- c d a b )
func (f *Forth) Word2Swap() {
	d := f.DSPop()
	c := f.DSPop()
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(c)
	f.DSPush(d)
	f.DSPush(a)
	f.DSPush(b)
}

// Word2Over copies third and fourth items to top: 2OVER ( a b c d -- a b c d a b )
func (f *Forth) Word2Over() {
	d := f.DSPop()
	c := f.DSPop()
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(a)
	f.DSPush(b)
	f.DSPush(c)
	f.DSPush(d)
	f.DSPush(a)
	f.DSPush(b)
}

// WordToR moves cell from data stack to return stack: >R ( n -- ) ( R: -- n )
func (f *Forth) WordToR() {
	val := f.DSPop()
	f.RSPush(val)
}

// WordRFrom moves cell from return stack to data stack: R> ( -- n ) ( R: n -- )
func (f *Forth) WordRFrom() {
	val := f.RSPop()
	f.DSPush(val)
}

// WordRFetch copies top of return stack onto data stack: R@ ( -- n ) ( R: n -- n )
func (f *Forth) WordRFetch() {
	val := f.RSPeek()
	f.DSPush(val)
}

// WordPlus pops a and b, pushing sum: + ( a b -- sum )
func (f *Forth) WordPlus() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(a + b)
}

// WordMinus pops a and b, pushing diff: - ( a b -- diff )
func (f *Forth) WordMinus() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(a - b)
}

// WordMultiply pops a and b, pushing prod: * ( a b -- prod )
func (f *Forth) WordMultiply() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(a * b)
}

// WordSlashMod pops a and b, pushing remainder then quotient: /MOD ( a b -- rem quot )
func (f *Forth) WordSlashMod() {
	b := f.DSPop()
	a := f.DSPop()
	if b == 0 {
		forthError("DIVISION BY ZERO")
	}
	f.DSPush(a % b)
	f.DSPush(a / b)
}

// WordSlash pops a and b, pushing quotient: / ( a b -- quot )
func (f *Forth) WordSlash() {
	b := f.DSPop()
	a := f.DSPop()
	if b == 0 {
		forthError("DIVISION BY ZERO")
	}
	f.DSPush(a / b)
}

// WordMod pops a and b, pushing remainder: MOD ( a b -- rem )
func (f *Forth) WordMod() {
	b := f.DSPop()
	a := f.DSPop()
	if b == 0 {
		forthError("DIVISION BY ZERO")
	}
	f.DSPush(a % b)
}

// WordNegate pops n, pushing two's complement negation: NEGATE ( n -- -n )
func (f *Forth) WordNegate() {
	n := f.DSPop()
	f.DSPush(-n)
}

// WordOnePlus increments top of stack by 1: 1+ ( n -- n+1 )
func (f *Forth) WordOnePlus() {
	n := f.DSPop()
	f.DSPush(n + 1)
}

// WordOneMinus decrements top of stack by 1: 1- ( n -- n-1 )
func (f *Forth) WordOneMinus() {
	n := f.DSPop()
	f.DSPush(n - 1)
}

// WordTwoPlus increments top of stack by 2: 2+ ( n -- n+2 )
func (f *Forth) WordTwoPlus() {
	n := f.DSPop()
	f.DSPush(n + 2)
}

// WordTwoMinus decrements top of stack by 2: 2- ( n -- n-2 )
func (f *Forth) WordTwoMinus() {
	n := f.DSPop()
	f.DSPush(n - 2)
}

// WordTwoStar multiplies top of stack by 2: 2* ( n -- n*2 )
func (f *Forth) WordTwoStar() {
	n := f.DSPop()
	f.DSPush(n << 1)
}

// WordTwoSlash divides top of stack by 2: 2/ ( n -- n/2 )
func (f *Forth) WordTwoSlash() {
	n := f.DSPop()
	f.DSPush(n >> 1)
}

// WordAbs pushes absolute value of signed cell: ABS ( n -- |n| )
func (f *Forth) WordAbs() {
	n := f.DSPop()
	if n < 0 {
		f.DSPush(-n)
	} else {
		f.DSPush(n)
	}
}

// WordMin pushes minimum of two signed integers: MIN ( a b -- min )
func (f *Forth) WordMin() {
	b := f.DSPop()
	a := f.DSPop()
	if a < b {
		f.DSPush(a)
	} else {
		f.DSPush(b)
	}
}

// WordMax pushes maximum of two signed integers: MAX ( a b -- max )
func (f *Forth) WordMax() {
	b := f.DSPop()
	a := f.DSPop()
	if a > b {
		f.DSPush(a)
	} else {
		f.DSPush(b)
	}
}

// WordAnd pops a and b, pushing bitwise AND: AND ( a b -- c )
func (f *Forth) WordAnd() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(a & b)
}

// WordOr pops a and b, pushing bitwise OR: OR ( a b -- c )
func (f *Forth) WordOr() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(a | b)
}

// WordXor pops a and b, pushing bitwise XOR: XOR ( a b -- c )
func (f *Forth) WordXor() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(a ^ b)
}

// WordInvert pops n, pushing one's complement: INVERT ( n -- ~n )
func (f *Forth) WordInvert() {
	n := f.DSPop()
	f.DSPush(^n)
}

func boolCell(b bool) Cell {
	if b {
		return -1
	}
	return 0
}

// WordEquals pops a and b, pushing -1 if a == b else 0: = ( a b -- flag )
func (f *Forth) WordEquals() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(boolCell(a == b))
}

// WordNotEquals pops a and b, pushing -1 if a != b else 0: <> ( a b -- flag )
func (f *Forth) WordNotEquals() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(boolCell(a != b))
}

// WordLess pops a and b, pushing -1 if signed a < b else 0: < ( a b -- flag )
func (f *Forth) WordLess() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(boolCell(a < b))
}

// WordGreater pops a and b, pushing -1 if signed a > b else 0: > ( a b -- flag )
func (f *Forth) WordGreater() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(boolCell(a > b))
}

// WordLessEqual pops a and b, pushing -1 if signed a <= b else 0: <= ( a b -- flag )
func (f *Forth) WordLessEqual() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(boolCell(a <= b))
}

// WordGreaterEqual pops a and b, pushing -1 if signed a >= b else 0: >= ( a b -- flag )
func (f *Forth) WordGreaterEqual() {
	b := f.DSPop()
	a := f.DSPop()
	f.DSPush(boolCell(a >= b))
}

// WordZeroEquals pops n, pushing -1 if n == 0 else 0: 0= ( n -- flag )
func (f *Forth) WordZeroEquals() {
	n := f.DSPop()
	f.DSPush(boolCell(n == 0))
}

// WordZeroNotEquals pops n, pushing -1 if n != 0 else 0: 0<> ( n -- flag )
func (f *Forth) WordZeroNotEquals() {
	n := f.DSPop()
	f.DSPush(boolCell(n != 0))
}

// WordZeroLess pops n, pushing -1 if signed n < 0 else 0: 0< ( n -- flag )
func (f *Forth) WordZeroLess() {
	n := f.DSPop()
	f.DSPush(boolCell(n < 0))
}

// WordZeroGreater pops n, pushing -1 if signed n > 0 else 0: 0> ( n -- flag )
func (f *Forth) WordZeroGreater() {
	n := f.DSPop()
	f.DSPush(boolCell(n > 0))
}

// WordULess pops a and b, pushing -1 if unsigned a < b else 0: U< ( a b -- flag )
func (f *Forth) WordULess() {
	b := uint16(f.DSPop())
	a := uint16(f.DSPop())
	f.DSPush(boolCell(a < b))
}

// WordUGreater pops a and b, pushing -1 if unsigned a > b else 0: U> ( a b -- flag )
func (f *Forth) WordUGreater() {
	b := uint16(f.DSPop())
	a := uint16(f.DSPop())
	f.DSPush(boolCell(a > b))
}

// WordULessEqual pops a and b, pushing -1 if unsigned a <= b else 0: U<= ( a b -- flag )
func (f *Forth) WordULessEqual() {
	b := uint16(f.DSPop())
	a := uint16(f.DSPop())
	f.DSPush(boolCell(a <= b))
}

// WordUGreaterEqual pops a and b, pushing -1 if unsigned a >= b else 0: U>= ( a b -- flag )
func (f *Forth) WordUGreaterEqual() {
	b := uint16(f.DSPop())
	a := uint16(f.DSPop())
	f.DSPush(boolCell(a >= b))
}

// DefineWord creates a new dictionary entry.
func (f *Forth) DefineWord(name string, wtype WordType, code XTCode, body []Cell, immediate bool) *Word {
	upper := strings.ToUpper(name)
	idx := len(f.Words)
	w := &Word{
		Name:      upper,
		Immediate: immediate,
		Type:      wtype,
		Code:      code,
		Body:      body,
		PFA:       f.DP,
	}
	f.Words = append(f.Words, w)
	f.wordsByName[upper] = idx
	f.Latest = idx
	return w
}

// DefinePrimitive creates a native primitive word.
func (f *Forth) DefinePrimitive(name string, code XTCode) *Word {
	return f.DefineWord(name, WordPrimitive, code, nil, false)
}

// DefineImmediate creates an immediate native primitive word.
func (f *Forth) DefineImmediate(name string, code XTCode) *Word {
	return f.DefineWord(name, WordPrimitive, code, nil, true)
}

// LatestWord returns the most recently defined dictionary word or nil.
func (f *Forth) LatestWord() *Word {
	if f.Latest < 0 || f.Latest >= len(f.Words) {
		return nil
	}
	return f.Words[f.Latest]
}

// Lookup finds a word by name (case-insensitive).
func (f *Forth) Lookup(name string) (*Word, bool) {
	upper := strings.ToUpper(name)
	idx, ok := f.wordsByName[upper]
	if !ok || idx < 0 || idx >= len(f.Words) {
		return nil, false
	}
	return f.Words[idx], true
}

// FindXT returns the XT (integer index) of a word by name (case-insensitive).
func (f *Forth) FindXT(name string) (int, bool) {
	upper := strings.ToUpper(name)
	idx, ok := f.wordsByName[upper]
	if !ok || idx < 0 || idx >= len(f.Words) {
		return -1, false
	}
	return idx, true
}

// ParseWord parses the next whitespace-delimited word from TIB starting at IN.
func (f *Forth) ParseWord() (string, bool) {
	for f.IN < len(f.TIB) && (f.TIB[f.IN] == ' ' || f.TIB[f.IN] == '\t' || f.TIB[f.IN] == '\r' || f.TIB[f.IN] == '\n') {
		f.IN++
	}
	if f.IN >= len(f.TIB) {
		return "", false
	}
	start := f.IN
	for f.IN < len(f.TIB) && f.TIB[f.IN] != ' ' && f.TIB[f.IN] != '\t' && f.TIB[f.IN] != '\r' && f.TIB[f.IN] != '\n' {
		f.IN++
	}
	return string(f.TIB[start:f.IN]), true
}

// WordFind looks up a counted string name in the dictionary: FIND ( addr -- xt flag )
func (f *Forth) WordFind() {
	addr := int(f.DSPop())
	length := int(f.CFetch(addr))
	nameBytes := make([]byte, length)
	for i := 0; i < length; i++ {
		nameBytes[i] = f.CFetch(addr + 1 + i)
	}
	name := strings.ToUpper(string(nameBytes))
	w, ok := f.Lookup(name)
	if !ok {
		f.DSPush(Cell(addr))
		f.DSPush(0)
		return
	}
	xt, _ := f.FindXT(name)
	f.DSPush(Cell(xt))
	if w.Immediate {
		f.DSPush(1)
	} else {
		f.DSPush(-1)
	}
}

// WordTick parses the next word name from input and pushes its XT: ' ( -- xt )
func (f *Forth) WordTick() {
	token, ok := f.ParseWord()
	if !ok {
		forthError("WORD NAME EXPECTED")
	}
	upper := strings.ToUpper(token)
	xt, ok := f.FindXT(upper)
	if !ok {
		forthError("WORD NOT FOUND: %s", token)
	}
	f.DSPush(Cell(xt))
}

// WordToBody converts XT to its parameter field address (PFA): >BODY ( xt -- pfa )
func (f *Forth) WordToBody() {
	xt := int(f.DSPop())
	if xt >= 0 && xt < len(f.Words) && f.Words[xt].PFA != 0 {
		f.DSPush(Cell(f.Words[xt].PFA))
	} else {
		f.DSPush(Cell(xt + 2))
	}
}

// Here returns current dictionary pointer address (value of DP variable).
func (f *Forth) Here() int {
	return int(f.Fetch(DPAddr))
}

// Allot advances the dictionary pointer by n bytes.
func (f *Forth) Allot(n int) {
	cur := f.Here()
	f.Store(DPAddr, Cell(cur+n))
	f.DP = cur + n
}

// Comma stores 16-bit cell at HERE and allots 2 bytes.
func (f *Forth) Comma(val Cell) {
	f.Store(f.Here(), val)
	f.Allot(2)
}

// CComma stores byte at HERE and allots 1 byte.
func (f *Forth) CComma(val byte) {
	f.CStore(f.Here(), val)
	f.Allot(1)
}

// WordDP pushes the address of the DP system variable onto the stack: DP ( -- addr )
func (f *Forth) WordDP() {
	f.DSPush(Cell(DPAddr))
}

// WordHere pushes the current dictionary pointer value onto the stack: HERE ( -- addr )
func (f *Forth) WordHere() {
	f.DSPush(Cell(f.Here()))
}

// WordPAD pushes temporary buffer address (HERE + 68) onto the stack: PAD ( -- addr )
func (f *Forth) WordPAD() {
	f.DSPush(Cell(f.Here() + 68))
}

// WordAllot advances the dictionary pointer by n bytes: ALLOT ( n -- )
func (f *Forth) WordAllot() {
	n := int(f.DSPop())
	f.Allot(n)
}

// WordComma compiles 16-bit cell at HERE and advances DP by 2: , ( n -- )
func (f *Forth) WordComma() {
	val := f.DSPop()
	f.Comma(val)
}

// WordCComma compiles byte at HERE and advances DP by 1: C, ( c -- )
func (f *Forth) WordCComma() {
	val := byte(f.DSPop() & 0xFF)
	f.CComma(val)
}

// WordExecute pops execution token and executes the word: EXECUTE ( xt -- )
func (f *Forth) WordExecute() {
	xt := int(f.DSPop())
	f.ExecuteWord(xt)
}

// ExecuteWord executes a dictionary word by its XT index.
func (f *Forth) ExecuteWord(xt int) {
	if xt < 0 || xt >= len(f.Words) {
		forthError("INVALID EXECUTION TOKEN: %d", xt)
	}
	w := f.Words[xt]
	switch w.Type {
	case WordPrimitive:
		if w.Code != nil {
			w.Code(f)
		}
	case WordConstant:
		if len(w.Body) > 0 {
			f.DSPush(w.Body[0])
		}
	case WordVariable, WordCreate:
		f.DSPush(Cell(w.PFA))
	case WordColon:
		for ip := 0; ip < len(w.Body); ip++ {
			f.CheckInterrupt()
			f.ExecuteWord(int(w.Body[ip]))
		}
	}
}

// CFetch reads a byte from 64KB memory.
func (f *Forth) CFetch(addr int) byte {
	if addr < 0 || addr >= len(f.Memory) {
		forthError("MEMORY ACCESS OUT OF BOUNDS: %d", addr)
	}
	return f.Memory[addr]
}

// CStore writes a byte to 64KB memory.
func (f *Forth) CStore(addr int, val byte) {
	if addr < 0 || addr >= len(f.Memory) {
		forthError("MEMORY ACCESS OUT OF BOUNDS: %d", addr)
	}
	f.Memory[addr] = val
}

// Fetch reads a 16-bit cell (little-endian) from memory.
func (f *Forth) Fetch(addr int) Cell {
	if addr < 0 || addr+1 >= len(f.Memory) {
		forthError("FETCH ADDR OUT OF RANGE: %d", addr)
	}
	u := uint16(f.Memory[addr]) | uint16(f.Memory[addr+1])<<8
	return Cell(int16(u))
}

// Store writes a 16-bit cell (little-endian) to memory.
func (f *Forth) Store(addr int, val Cell) {
	if addr < 0 || addr+1 >= len(f.Memory) {
		forthError("STORE ADDR OUT OF RANGE: %d", addr)
	}
	u := uint16(val)
	f.Memory[addr] = byte(u & 0xFF)
	f.Memory[addr+1] = byte((u >> 8) & 0xFF)
}

// WordFetch pops addr and pushes 16-bit cell from memory: @ ( addr -- n )
func (f *Forth) WordFetch() {
	addr := int(f.DSPop())
	f.DSPush(f.Fetch(addr))
}

// WordStore pops addr and val, writing 16-bit cell to memory: ! ( n addr -- )
func (f *Forth) WordStore() {
	addr := int(f.DSPop())
	val := f.DSPop()
	f.Store(addr, val)
}

// WordCFetch pops addr and pushes single byte from memory: C@ ( addr -- c )
func (f *Forth) WordCFetch() {
	addr := int(f.DSPop())
	f.DSPush(Cell(f.CFetch(addr)))
}

// WordCStore pops addr and byte, writing byte to memory: C! ( c addr -- )
func (f *Forth) WordCStore() {
	addr := int(f.DSPop())
	val := byte(f.DSPop() & 0xFF)
	f.CStore(addr, val)
}

// WordPlusStore pops addr and n, adding n to the 16-bit cell at addr: +! ( n addr -- )
func (f *Forth) WordPlusStore() {
	addr := int(f.DSPop())
	n := f.DSPop()
	cur := f.Fetch(addr)
	f.Store(addr, cur+n)
}

// WordQuestion pops addr, fetches cell at addr and prints it followed by space: ? ( addr -- )
func (f *Forth) WordQuestion() {
	addr := int(f.DSPop())
	val := f.Fetch(addr)
	f.EmitStr(fmt.Sprintf("%d ", val))
}

// WordFill fills memory region starting at addr of len bytes with byte value: FILL ( addr len val -- )
func (f *Forth) WordFill() {
	val := byte(f.DSPop() & 0xFF)
	length := int(f.DSPop())
	addr := int(f.DSPop())
	for i := 0; i < length; i++ {
		f.CStore(addr+i, val)
	}
}

// WordErase fills memory region starting at addr of len bytes with zero: ERASE ( addr len -- )
func (f *Forth) WordErase() {
	length := int(f.DSPop())
	addr := int(f.DSPop())
	for i := 0; i < length; i++ {
		f.CStore(addr+i, 0)
	}
}

// WordCMove copies len bytes from src to dst forward order: CMOVE ( dst src len -- )
func (f *Forth) WordCMove() {
	length := int(f.DSPop())
	src := int(f.DSPop())
	dst := int(f.DSPop())
	for i := 0; i < length; i++ {
		f.CStore(dst+i, f.CFetch(src+i))
	}
}

// WordCMoveGreater copies len bytes from src to dst backward order: CMOVE> ( dst src len -- )
func (f *Forth) WordCMoveGreater() {
	length := int(f.DSPop())
	src := int(f.DSPop())
	dst := int(f.DSPop())
	for i := length - 1; i >= 0; i-- {
		f.CStore(dst+i, f.CFetch(src+i))
	}
}

// WordSpan pushes the address of the SPAN system variable onto the stack.
func (f *Forth) WordSpan() {
	f.DSPush(Cell(SpanAddr))
}

// WordExpect reads up to maxlen characters into memory at addr, updating SPAN.
func (f *Forth) WordExpect() {
	maxlen := int(f.DSPop())
	addr := int(f.DSPop())
	if maxlen <= 0 {
		f.Store(SpanAddr, 0)
		return
	}
	count := 0
	for count < maxlen {
		b, err := f.bufIn.ReadByte()
		if err != nil {
			break
		}
		if b == '\r' {
			next, err := f.bufIn.Peek(1)
			if err == nil && len(next) > 0 && next[0] == '\n' {
				f.bufIn.ReadByte()
			}
			break
		}
		if b == '\n' {
			break
		}
		f.CStore(addr+count, b)
		count++
	}
	f.Store(SpanAddr, Cell(count))
}

// ReadLine reads a line from the input stream, trimming line terminators.
func (f *Forth) ReadLine() (string, error) {
	if f.bufIn == nil {
		return "", io.EOF
	}
	line, err := f.bufIn.ReadString('\n')
	if err != nil {
		if len(line) > 0 {
			line = strings.TrimRight(line, "\r\n")
			return line, nil
		}
		return "", err
	}
	line = strings.TrimRight(line, "\r\n")
	return line, nil
}

// SetLine sets the TIB and resets the input pointer IN to 0.
func (f *Forth) SetLine(line string) {
	f.TIB = []byte(line)
	f.IN = 0
}

// Key reads a single byte from input stream.
func (f *Forth) Key() {
	if f.bufIn == nil {
		f.DSPush(0)
		return
	}
	b, err := f.bufIn.ReadByte()
	if err != nil {
		f.DSPush(0)
		return
	}
	f.DSPush(Cell(b))
}

// Emit writes a single byte to output.
func (f *Forth) Emit(c byte) {
	if f.Output != nil {
		f.Output.Write([]byte{c})
	}
}

// WordEmit pops a character code from stack and writes it.
func (f *Forth) WordEmit() {
	c := byte(f.DSPop() & 0xFF)
	f.Emit(c)
}

// WordCR emits a newline.
func (f *Forth) WordCR() {
	if f.Output != nil {
		f.Output.Write([]byte("\n"))
	}
}

// WordSpace emits a single space.
func (f *Forth) WordSpace() {
	f.Emit(' ')
}

// WordSpaces emits n consecutive spaces.
func (f *Forth) WordSpaces() {
	n := f.DSPop()
	for i := Cell(0); i < n; i++ {
		f.Emit(' ')
	}
}

// WordType pops address and length, emitting characters from memory.
func (f *Forth) WordType() {
	length := int(f.DSPop())
	addr := int(f.DSPop())
	for i := 0; i < length; i++ {
		f.Emit(f.CFetch(addr + i))
	}
}

// WordBye sets Running to false to signal interpreter termination.
func (f *Forth) WordBye() {
	f.Running = false
}

// LoadFile reads and evaluates a Forth source file by path.
func (f *Forth) LoadFile(path string) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("CANNOT OPEN: %s", path)
	}
	f.LoadSystem(string(data))
	return nil
}

// LoadSystem evaluates Forth code line by line.
func (f *Forth) LoadSystem(data string) {
	for _, line := range strings.Split(data, "\n") {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "\\") {
			continue
		}
	}
}

// EmitStr writes a string to output.
func (f *Forth) EmitStr(s string) {
	if f.Output != nil {
		f.Output.Write([]byte(s))
	}
}

// Abort resets stacks, execution state, and returns to REPL.
func (f *Forth) Abort() {
	atomic.StoreInt32(&f.Interrupted, 0)
	f.State = false
	f.DS = f.DS[:0]
	f.RS = f.RS[:0]
	f.IN = len(f.TIB)
}

// CheckInterrupt verifies if execution was halted by SIGINT.
func (f *Forth) CheckInterrupt() {
	if atomic.LoadInt32(&f.Interrupted) != 0 {
		atomic.StoreInt32(&f.Interrupted, 0)
		f.Abort()
		forthError("INTERRUPTED")
	}
}

// InterpretLine sets the line into TIB and executes it.
func (f *Forth) InterpretLine(line string) {
	f.SetLine(line)
}

// replStep reads and processes a single line in the REPL.
func (f *Forth) replStep() {
	defer func() {
		if r := recover(); r != nil {
			msg := fmt.Sprint(r)
			if _, ok := r.(*ForthError); ok {
				f.EmitStr(msg)
				f.EmitStr("\n")
			} else {
				f.EmitStr("PANIC: ")
				f.EmitStr(msg)
				f.EmitStr("\n")
			}
			f.Abort()
			f.EmitStr(" ok\n")
		}
	}()

	line, err := f.ReadLine()
	if err != nil {
		f.Running = false
		return
	}

	if atomic.LoadInt32(&f.Interrupted) != 0 {
		atomic.StoreInt32(&f.Interrupted, 0)
		f.Abort()
		f.EmitStr("\n INTERRUPTED\n ok\n")
		return
	}

	f.InterpretLine(line)
	f.EmitStr(" ok\n")
}

// QUIT runs the outer interpreter loop until EOF or BYE.
func (f *Forth) QUIT() {
	f.State = false
	f.DS = f.DS[:0]
	f.RS = f.RS[:0]

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT)
	quit := make(chan struct{})
	go func() {
		for {
			select {
			case <-sigChan:
				atomic.StoreInt32(&f.Interrupted, 1)
			case <-quit:
				return
			}
		}
	}()
	defer func() {
		signal.Stop(sigChan)
		close(quit)
	}()

	f.EmitStr(" ok\n")
	for f.Running {
		f.replStep()
	}
}
