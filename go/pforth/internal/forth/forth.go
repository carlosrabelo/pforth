package forth

import (
	"io"
)

// Cell represents a 16-bit Forth cell value.
type Cell = int16

// Forth represents the Forth interpreter and virtual machine state.
type Forth struct {
	Input  io.Reader
	Output io.Writer
}

// New creates and initializes a new Forth engine instance.
func New(input io.Reader, output io.Writer) *Forth {
	return &Forth{
		Input:  input,
		Output: output,
	}
}
