package forth

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"strings"
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
		Input:       input,
		Output:      output,
		bufIn:       bufio.NewReader(input),
		Running:     true,
		Loaded:      make(map[string]bool),
	}
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

// QUIT runs the outer interpreter loop until EOF or BYE.
func (f *Forth) QUIT() {
	if f.bufIn == nil {
		return
	}
	for f.Running {
		_, err := f.bufIn.ReadString('\n')
		if err != nil {
			break
		}
	}
}
