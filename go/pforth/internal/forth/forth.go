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

	CompileName string
	CompileList []Cell
	CompileWord *Word
	CompilingXT int
	CurrentBody []Cell
	IP          int

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
		CompileList: make([]Cell, 0, 64),
		CurrentBody: make([]Cell, 0, 64),
		CompilingXT: -1,
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
		savedBody := f.CurrentBody
		savedIP := f.IP
		f.CurrentBody = w.Body
		f.IP = 0
		defer func() {
			f.CurrentBody = savedBody
			f.IP = savedIP
		}()
		for f.IP < len(f.CurrentBody) {
			f.CheckInterrupt()
			nxt := int(f.CurrentBody[f.IP])
			f.IP++
			f.ExecuteWord(nxt)
		}
	}
}

// WordLIT pushes the inline literal cell following it and advances IP: LIT ( -- n )
func (f *Forth) WordLIT() {
	if f.IP < len(f.CurrentBody) {
		f.DSPush(f.CurrentBody[f.IP])
		f.IP++
	}
}

// WordState pushes the current compiler state flag (-1 compiling, 0 interpreting): STATE ( -- flag )
func (f *Forth) WordState() {
	f.DSPush(boolCell(f.State))
}

// WordLeftBracket sets STATE to interpreting (0): [ ( -- )
func (f *Forth) WordLeftBracket() {
	f.State = false
}

// WordRightBracket sets STATE to compiling (-1): ] ( -- )
func (f *Forth) WordRightBracket() {
	f.State = true
}

// WordLiteral compiles LIT followed by n: LITERAL ( n -- )
func (f *Forth) WordLiteral() {
	val := f.DSPop()
	litXT, ok := f.FindXT("LIT")
	if !ok {
		f.DefinePrimitive("LIT", func(f *Forth) { f.WordLIT() })
		litXT, _ = f.FindXT("LIT")
	}
	f.CompileList = append(f.CompileList, Cell(litXT), val)
}

// WordCompile compiles next token's execution token: COMPILE
func (f *Forth) WordCompile() {
	token, ok := f.ParseWord()
	if !ok {
		forthError("WORD NAME EXPECTED")
	}
	upper := strings.ToUpper(token)
	xt, ok := f.FindXT(upper)
	if !ok {
		forthError("WORD NOT FOUND: %s", token)
	}
	f.CompileList = append(f.CompileList, Cell(xt))
}

// WordBracketCompile compiles next token's XT even if immediate: [COMPILE] ( -- )
func (f *Forth) WordBracketCompile() {
	token, ok := f.ParseWord()
	if !ok {
		forthError("WORD NAME EXPECTED")
	}
	upper := strings.ToUpper(token)
	xt, ok := f.FindXT(upper)
	if !ok {
		forthError("WORD NOT FOUND: %s", token)
	}
	f.CompileList = append(f.CompileList, Cell(xt))
}

// WordColon starts compiling a new colon definition: : ( -- )
func (f *Forth) WordColon() {
	if f.State {
		forthError("ALREADY COMPILING")
	}
	token, ok := f.ParseWord()
	if !ok {
		forthError("WORD NAME EXPECTED")
	}
	f.CompileList = f.CompileList[:0]
	f.DefineWord(token, WordColon, nil, nil, false)
	f.CompilingXT = len(f.Words) - 1
	f.State = true
}

// WordSemicolon closes current colon definition: ; ( -- )
func (f *Forth) WordSemicolon() {
	if !f.State {
		forthError("NOT IN COMPILATION STATE")
	}
	exitXT, ok := f.FindXT("EXIT")
	if !ok {
		f.DefinePrimitive("EXIT", func(f *Forth) { f.WordExit() })
		exitXT, _ = f.FindXT("EXIT")
	}
	f.CompileList = append(f.CompileList, Cell(exitXT))
	f.State = false
	if f.CompilingXT >= 0 && f.CompilingXT < len(f.Words) {
		f.Words[f.CompilingXT].Body = make([]Cell, len(f.CompileList))
		copy(f.Words[f.CompilingXT].Body, f.CompileList)
	}
	f.CompileList = f.CompileList[:0]
	f.CompilingXT = -1
}

// WordExit terminates execution of current colon definition: EXIT ( -- )
func (f *Forth) WordExit() {
	f.IP = len(f.CurrentBody)
}

// WordImmediate marks the most recently defined word as immediate: IMMEDIATE ( -- )
func (f *Forth) WordImmediate() {
	w := f.LatestWord()
	if w != nil {
		w.Immediate = true
	}
}

// WordRecurse compiles the XT of the word currently being defined: RECURSE ( -- )
func (f *Forth) WordRecurse() {
	if !f.State {
		forthError("RECURSE ONLY IN COMPILATION")
	}
	if f.CompilingXT < 0 {
		forthError("NO COMPILATION IN PROGRESS")
	}
	f.CompileList = append(f.CompileList, Cell(f.CompilingXT))
}

// WordBranch performs an unconditional branch by inline relative offset: BRANCH
func (f *Forth) WordBranch() {
	if f.IP < len(f.CurrentBody) {
		offset := int(f.CurrentBody[f.IP])
		f.IP += offset
	}
}

// WordZeroBranch branches by inline offset if TOS is 0, else skips offset: 0BRANCH ( flag -- )
func (f *Forth) WordZeroBranch() {
	flag := f.DSPop()
	if f.IP < len(f.CurrentBody) {
		offset := int(f.CurrentBody[f.IP])
		if flag == 0 {
			f.IP += offset
		} else {
			f.IP++
		}
	}
}

// WordIF compiles 0BRANCH and leaves offset address on DS: IF ( -- orig )
func (f *Forth) WordIF() {
	if !f.State {
		forthError("IF ONLY IN COMPILATION")
	}
	xt, ok := f.FindXT("0BRANCH")
	if !ok {
		f.DefinePrimitive("0BRANCH", func(f *Forth) { f.WordZeroBranch() })
		xt, _ = f.FindXT("0BRANCH")
	}
	f.CompileList = append(f.CompileList, Cell(xt), 0)
	f.DSPush(Cell(len(f.CompileList) - 1))
}

// WordTHEN resolves forward branch offset from orig to current compilation position: THEN ( orig -- )
func (f *Forth) WordTHEN() {
	if !f.State {
		forthError("THEN ONLY IN COMPILATION")
	}
	pos := int(f.DSPop())
	if pos < 0 || pos >= len(f.CompileList) {
		forthError("INVALID BRANCH TARGET")
	}
	offset := Cell(len(f.CompileList) - pos)
	f.CompileList[pos] = offset
}

// WordELSE compiles unconditional BRANCH, resolves orig1, and returns orig2: ELSE ( orig1 -- orig2 )
func (f *Forth) WordELSE() {
	if !f.State {
		forthError("ELSE ONLY IN COMPILATION")
	}
	branchXT, ok := f.FindXT("BRANCH")
	if !ok {
		f.DefinePrimitive("BRANCH", func(f *Forth) { f.WordBranch() })
		branchXT, _ = f.FindXT("BRANCH")
	}
	f.CompileList = append(f.CompileList, Cell(branchXT), 0)
	pos2 := len(f.CompileList) - 1
	pos1 := int(f.DSPop())
	if pos1 < 0 || pos1 >= len(f.CompileList) {
		forthError("INVALID BRANCH TARGET")
	}
	offset1 := Cell(pos2 + 1 - pos1)
	f.CompileList[pos1] = offset1
	f.DSPush(Cell(pos2))
}

// WordCreate parses next word name from input, creating new WordCreate entry: CREATE ( -- )
func (f *Forth) WordCreate() {
	token, ok := f.ParseWord()
	if !ok {
		forthError("WORD NAME EXPECTED")
	}
	w := f.DefineWord(token, WordCreate, nil, nil, false)
	w.PFA = f.Here()
}

// WordConstant pops n from stack, parses next word name, and creates WordConstant: CONSTANT ( n -- )
func (f *Forth) WordConstant() {
	val := f.DSPop()
	token, ok := f.ParseWord()
	if !ok {
		forthError("WORD NAME EXPECTED")
	}
	f.DefineWord(token, WordConstant, nil, []Cell{val}, false)
}

// WordVariable parses next word name from input, creates WordVariable allocating 2 bytes initialized to 0: VARIABLE ( -- )
func (f *Forth) WordVariable() {
	token, ok := f.ParseWord()
	if !ok {
		forthError("WORD NAME EXPECTED")
	}
	w := f.DefineWord(token, WordVariable, nil, nil, false)
	w.PFA = f.Here()
	f.Comma(0)
}

// GetBase returns current numeric base from BaseAddr (defaults to 10 if <= 0).
func (f *Forth) GetBase() int {
	b := int(f.Fetch(BaseAddr))
	if b <= 0 {
		return 10
	}
	return b
}

// SetBase sets current numeric base in BaseAddr and updates f.Base.
func (f *Forth) SetBase(b int) {
	f.Store(BaseAddr, Cell(b))
	f.Base = b
}

// ParseNumber parses a string token into a 16-bit cell according to current BASE or prefixes ($ for hex, % for binary).
func (f *Forth) ParseNumber(token string) (Cell, bool) {
	if len(token) == 0 {
		return 0, false
	}
	curBase := f.GetBase()
	neg := false
	start := 0
	if token[0] == '-' {
		neg = true
		start = 1
	} else if token[0] == '+' {
		start = 1
	}
	if start >= len(token) {
		return 0, false
	}

	parsedBase := curBase
	if token[start] == '$' {
		parsedBase = 16
		start++
	} else if token[start] == '%' {
		parsedBase = 2
		start++
	} else if token[start] == '#' {
		start++
		if start < len(token) && token[start] == '#' {
			start++
		}
	}
	if start >= len(token) {
		return 0, false
	}

	var val uint32
	for i := start; i < len(token); i++ {
		c := token[i]
		digit := -1
		if c >= '0' && c <= '9' {
			digit = int(c - '0')
		} else if c >= 'A' && c <= 'F' {
			digit = int(c - 'A' + 10)
		} else if c >= 'a' && c <= 'f' {
			digit = int(c - 'a' + 10)
		} else {
			return 0, false
		}
		if digit >= parsedBase {
			return 0, false
		}
		val = val*uint32(parsedBase) + uint32(digit)
	}

	result := Cell(int16(uint16(val & 0xFFFF)))
	if neg {
		return -result, true
	}
	return result, true
}

// WordWord parses next token up to delim, storing counted string at HERE: WORD ( delim -- addr )
func (f *Forth) WordWord() {
	delimCell := f.DSPop()
	delim := byte(delimCell & 0xFF)
	for f.IN < len(f.TIB) && f.TIB[f.IN] == delim {
		f.IN++
	}
	dest := f.Here()
	if f.IN >= len(f.TIB) {
		f.CStore(dest, 0)
		f.DSPush(Cell(dest))
		return
	}
	start := dest + 1
	count := 0
	for f.IN < len(f.TIB) && f.TIB[f.IN] != delim && f.TIB[f.IN] != '\r' && f.TIB[f.IN] != '\n' {
		f.CStore(start+count, f.TIB[f.IN])
		count++
		f.IN++
	}
	f.CStore(dest, byte(count))
	f.CStore(start+count, ' ')
	f.DSPush(Cell(dest))
}

// WordNumber converts counted string at addr to number: NUMBER ( addr -- n flag )
func (f *Forth) WordNumber() {
	addr := int(f.DSPop())
	length := int(f.CFetch(addr))
	tokenBytes := make([]byte, length)
	for i := 0; i < length; i++ {
		tokenBytes[i] = f.CFetch(addr + 1 + i)
	}
	num, ok := f.ParseNumber(string(tokenBytes))
	if ok {
		f.DSPush(num)
		f.DSPush(-1)
	} else {
		f.DSPush(0)
		f.DSPush(0)
	}
}

// WordBase pushes the address of the BASE system variable: BASE ( -- addr )
func (f *Forth) WordBase() {
	f.DSPush(Cell(BaseAddr))
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
		f.InterpretLine(line)
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
	f.CompileList = f.CompileList[:0]
	f.CompilingXT = -1
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

// InterpretLoop processes tokens sequentially from TIB until end of line.
func (f *Forth) InterpretLoop() {
	for {
		f.CheckInterrupt()
		token, ok := f.ParseWord()
		if !ok {
			break
		}
		upper := strings.ToUpper(token)
		w, found := f.Lookup(upper)
		if found {
			xt, _ := f.FindXT(upper)
			if f.State && !w.Immediate {
				f.CompileList = append(f.CompileList, Cell(xt))
			} else {
				f.ExecuteWord(xt)
			}
		} else {
			num, isNum := f.ParseNumber(token)
			if isNum {
				if f.State {
					litXT, ok := f.FindXT("LIT")
					if !ok {
						f.DefinePrimitive("LIT", func(f *Forth) { f.WordLIT() })
						litXT, _ = f.FindXT("LIT")
					}
					f.CompileList = append(f.CompileList, Cell(litXT), num)
				} else {
					f.DSPush(num)
				}
			} else {
				errToken := token
				f.Abort()
				forthError("UNKNOWN WORD: %s", errToken)
			}
		}
	}
}

// WordInterpret executes the interpreter loop on the current TIB buffer: INTERPRET ( -- )
func (f *Forth) WordInterpret() {
	f.InterpretLoop()
}

// WordAbort clears data stack, resets compilation state, and discards input: ABORT ( -- )
func (f *Forth) WordAbort() {
	f.Abort()
}

// WordQuit resets return stack and runs outer interpreter: QUIT ( -- )
func (f *Forth) WordQuit() {
	f.QUIT()
}

// InterpretLine sets the line into TIB and executes it.
func (f *Forth) InterpretLine(line string) {
	f.SetLine(line)
	f.WordInterpret()
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

// InitPrimitives registers standard native Forth primitives into dictionary.
func (f *Forth) InitPrimitives() {
	// 001 Core I/O
	f.DefinePrimitive("KEY", func(f *Forth) { f.Key() })
	f.DefinePrimitive("EMIT", func(f *Forth) { f.WordEmit() })
	f.DefinePrimitive("CR", func(f *Forth) { f.WordCR() })
	f.DefinePrimitive("SPACE", func(f *Forth) { f.WordSpace() })
	f.DefinePrimitive("SPACES", func(f *Forth) { f.WordSpaces() })
	f.DefinePrimitive("TYPE", func(f *Forth) { f.WordType() })
	f.DefinePrimitive("BYE", func(f *Forth) { f.WordBye() })
	f.DefinePrimitive("EXPECT", func(f *Forth) { f.WordExpect() })
	f.DefinePrimitive("SPAN", func(f *Forth) { f.WordSpan() })
	f.DefinePrimitive("QUIT", func(f *Forth) { f.WordQuit() })
	f.DefinePrimitive("ABORT", func(f *Forth) { f.WordAbort() })

	// 002 Memory & Stacks
	f.DefinePrimitive("@", func(f *Forth) { f.WordFetch() })
	f.DefinePrimitive("!", func(f *Forth) { f.WordStore() })
	f.DefinePrimitive("C@", func(f *Forth) { f.WordCFetch() })
	f.DefinePrimitive("C!", func(f *Forth) { f.WordCStore() })
	f.DefinePrimitive("FILL", func(f *Forth) { f.WordFill() })
	f.DefinePrimitive("CMOVE", func(f *Forth) { f.WordCMove() })
	f.DefinePrimitive("CMOVE>", func(f *Forth) { f.WordCMoveGreater() })

	// 003 Stack Manipulation
	f.DefinePrimitive("DUP", func(f *Forth) { f.WordDup() })
	f.DefinePrimitive("DROP", func(f *Forth) { f.WordDrop() })
	f.DefinePrimitive("SWAP", func(f *Forth) { f.WordSwap() })
	f.DefinePrimitive("OVER", func(f *Forth) { f.WordOver() })
	f.DefinePrimitive("?DUP", func(f *Forth) { f.WordQDup() })
	f.DefinePrimitive("2SWAP", func(f *Forth) { f.Word2Swap() })
	f.DefinePrimitive("2OVER", func(f *Forth) { f.Word2Over() })
	f.DefinePrimitive(">R", func(f *Forth) { f.WordToR() })
	f.DefinePrimitive("R>", func(f *Forth) { f.WordRFrom() })
	f.DefinePrimitive("R@", func(f *Forth) { f.WordRFetch() })

	// 004 Arithmetic & Logic
	f.DefinePrimitive("+", func(f *Forth) { f.WordPlus() })
	f.DefinePrimitive("-", func(f *Forth) { f.WordMinus() })
	f.DefinePrimitive("*", func(f *Forth) { f.WordMultiply() })
	f.DefinePrimitive("/MOD", func(f *Forth) { f.WordSlashMod() })
	f.DefinePrimitive("AND", func(f *Forth) { f.WordAnd() })
	f.DefinePrimitive("OR", func(f *Forth) { f.WordOr() })
	f.DefinePrimitive("XOR", func(f *Forth) { f.WordXor() })
	f.DefinePrimitive("INVERT", func(f *Forth) { f.WordInvert() })
	f.DefinePrimitive("=", func(f *Forth) { f.WordEquals() })
	f.DefinePrimitive("<", func(f *Forth) { f.WordLess() })
	f.DefinePrimitive(">", func(f *Forth) { f.WordGreater() })
	f.DefinePrimitive("U<", func(f *Forth) { f.WordULess() })
	f.DefinePrimitive("U>", func(f *Forth) { f.WordUGreater() })
	f.DefinePrimitive("U<=", func(f *Forth) { f.WordULessEqual() })
	f.DefinePrimitive("U>=", func(f *Forth) { f.WordUGreaterEqual() })

	// 005 Dictionary & Words
	f.DefinePrimitive("FIND", func(f *Forth) { f.WordFind() })
	f.DefinePrimitive("'", func(f *Forth) { f.WordTick() })
	f.DefinePrimitive(">BODY", func(f *Forth) { f.WordToBody() })
	f.DefinePrimitive("DP", func(f *Forth) { f.WordDP() })
	f.DefinePrimitive("HERE", func(f *Forth) { f.WordHere() })
	f.DefinePrimitive("PAD", func(f *Forth) { f.WordPAD() })
	f.DefinePrimitive("ALLOT", func(f *Forth) { f.WordAllot() })
	f.DefinePrimitive(",", func(f *Forth) { f.WordComma() })
	f.DefinePrimitive("C,", func(f *Forth) { f.WordCComma() })
	f.DefinePrimitive("EXECUTE", func(f *Forth) { f.WordExecute() })
	f.DefinePrimitive("CREATE", func(f *Forth) { f.WordCreate() })
	f.DefinePrimitive("CONSTANT", func(f *Forth) { f.WordConstant() })
	f.DefinePrimitive("VARIABLE", func(f *Forth) { f.WordVariable() })

	// 006 Interpreter & Compiler
	f.DefinePrimitive("WORD", func(f *Forth) { f.WordWord() })
	f.DefinePrimitive("NUMBER", func(f *Forth) { f.WordNumber() })
	f.DefinePrimitive("BASE", func(f *Forth) { f.WordBase() })
	f.DefinePrimitive("INTERPRET", func(f *Forth) { f.WordInterpret() })
	f.DefinePrimitive("LIT", func(f *Forth) { f.WordLIT() })
	f.DefinePrimitive("STATE", func(f *Forth) { f.WordState() })
	f.DefineImmediate("[", func(f *Forth) { f.WordLeftBracket() })
	f.DefinePrimitive("]", func(f *Forth) { f.WordRightBracket() })
	f.DefineImmediate("LITERAL", func(f *Forth) { f.WordLiteral() })
	f.DefinePrimitive("COMPILE", func(f *Forth) { f.WordCompile() })
	f.DefineImmediate("[COMPILE]", func(f *Forth) { f.WordBracketCompile() })
	f.DefinePrimitive("EXIT", func(f *Forth) { f.WordExit() })
	f.DefinePrimitive(":", func(f *Forth) { f.WordColon() })
	f.DefineImmediate(";", func(f *Forth) { f.WordSemicolon() })
	f.DefineImmediate("IMMEDIATE", func(f *Forth) { f.WordImmediate() })
	f.DefineImmediate("RECURSE", func(f *Forth) { f.WordRecurse() })

	// 007 Control Flow
	f.DefinePrimitive("BRANCH", func(f *Forth) { f.WordBranch() })
	f.DefinePrimitive("0BRANCH", func(f *Forth) { f.WordZeroBranch() })
	f.DefineImmediate("IF", func(f *Forth) { f.WordIF() })
	f.DefineImmediate("THEN", func(f *Forth) { f.WordTHEN() })
	f.DefineImmediate("ELSE", func(f *Forth) { f.WordELSE() })
}
