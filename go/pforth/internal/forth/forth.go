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
	return Cell(int(f.Memory[addr]) | int(f.Memory[addr+1])<<8)
}

// Store writes a 16-bit cell (little-endian) to memory.
func (f *Forth) Store(addr int, val Cell) {
	if addr < 0 || addr+1 >= len(f.Memory) {
		forthError("STORE ADDR OUT OF RANGE: %d", addr)
	}
	f.Memory[addr] = byte(val & 0xFF)
	f.Memory[addr+1] = byte((val >> 8) & 0xFF)
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
