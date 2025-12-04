package forth

import "fmt"

// Cell represents a 16-bit Forth cell value.
type Cell = int16

// WordType represents the category of dictionary word.
type WordType int

const (
	WordPrimitive WordType = iota
	WordColon
	WordConstant
	WordVariable
	WordCreate
)

// XTCode is the native Go function executed by primitive words.
type XTCode func(f *Forth)

// Word represents a dictionary entry.
type Word struct {
	Name      string
	Immediate bool
	Type      WordType
	Code      XTCode
	Body      []Cell
	PFA       int
}

// ForthError represents a recoverable Forth runtime error.
type ForthError struct {
	Message string
}

func (e *ForthError) Error() string {
	return e.Message
}

func forthError(format string, args ...any) {
	panic(&ForthError{Message: fmt.Sprintf(format, args...)})
}
