package forth

import (
	"bytes"
	"strings"
	"testing"
)

func TestNew(t *testing.T) {
	in := &bytes.Buffer{}
	out := &bytes.Buffer{}
	f := New(in, out)
	if f == nil {
		t.Fatal("expected non-nil Forth instance")
	}
	if len(f.Memory) != 65536 {
		t.Fatalf("expected 64KB memory, got %d", len(f.Memory))
	}
}

func TestCharIOPrimitives(t *testing.T) {
	in := strings.NewReader("A")
	out := &bytes.Buffer{}
	f := New(in, out)

	// KEY
	f.Key()
	if len(f.DS) != 1 || f.DSPop() != 'A' {
		t.Fatal("KEY failed to read 'A'")
	}

	// KEY on EOF
	f.Key()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("KEY on EOF should push 0")
	}

	// EMIT
	f.DSPush(65)
	f.WordEmit()
	if out.String() != "A" {
		t.Fatalf("EMIT expected 'A', got %q", out.String())
	}
	out.Reset()

	// CR
	f.WordCR()
	if out.String() != "\n" {
		t.Fatalf("CR expected newline, got %q", out.String())
	}
	out.Reset()

	// SPACE
	f.WordSpace()
	if out.String() != " " {
		t.Fatalf("SPACE expected single space, got %q", out.String())
	}
	out.Reset()

	// SPACES
	f.DSPush(3)
	f.WordSpaces()
	if out.String() != "   " {
		t.Fatalf("SPACES expected 3 spaces, got %q", out.String())
	}
	out.Reset()

	// SPACES negative/zero
	f.DSPush(0)
	f.WordSpaces()
	f.DSPush(-2)
	f.WordSpaces()
	if out.String() != "" {
		t.Fatalf("SPACES <= 0 expected empty, got %q", out.String())
	}
	out.Reset()

	// TYPE
	testMsg := "Hello Forth"
	addr := 2000
	for i := 0; i < len(testMsg); i++ {
		f.CStore(addr+i, testMsg[i])
	}
	f.DSPush(Cell(addr))
	f.DSPush(Cell(len(testMsg)))
	f.WordType()
	if out.String() != testMsg {
		t.Fatalf("TYPE expected %q, got %q", testMsg, out.String())
	}
	out.Reset()

	// BYE
	if !f.Running {
		t.Fatal("expected Running to be true initially")
	}
	f.WordBye()
	if f.Running {
		t.Fatal("expected Running to be false after BYE")
	}
}

func TestStackUnderflow(t *testing.T) {
	in := &bytes.Buffer{}
	out := &bytes.Buffer{}
	f := New(in, out)

	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic on DSPop from empty stack")
		} else {
			fe, ok := r.(*ForthError)
			if !ok || fe.Message != "DATA STACK UNDERFLOW" {
				t.Fatalf("unexpected panic value: %v", r)
			}
		}
	}()

	f.DSPop()
}
