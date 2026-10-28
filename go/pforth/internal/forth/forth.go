package forth

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"strings"
)

// Cell represents a 16-bit Forth cell value.
type Cell = int16

// Forth represents the Forth interpreter and virtual machine state.
type Forth struct {
	Input   io.Reader
	Output  io.Writer
	bufIn   *bufio.Reader
	Running bool
}

// New creates and initializes a new Forth engine instance.
func New(input io.Reader, output io.Writer) *Forth {
	return &Forth{
		Input:   input,
		Output:  output,
		bufIn:   bufio.NewReader(input),
		Running: true,
	}
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
