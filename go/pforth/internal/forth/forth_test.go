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

func TestExpectAndSpan(t *testing.T) {
	in := strings.NewReader("ForthCode\r\nRest")
	out := &bytes.Buffer{}
	f := New(in, out)

	bufAddr := 3000
	f.DSPush(Cell(bufAddr))
	f.DSPush(20) // maxlen
	f.WordExpect()

	// SPAN check
	if f.Fetch(SpanAddr) != 9 {
		t.Fatalf("expected SPAN = 9, got %d", f.Fetch(SpanAddr))
	}

	// Read content from memory
	var s strings.Builder
	for i := 0; i < 9; i++ {
		s.WriteByte(f.CFetch(bufAddr + i))
	}
	if s.String() != "ForthCode" {
		t.Fatalf("expected 'ForthCode', got %q", s.String())
	}

	// WordSpan stack check
	f.WordSpan()
	if f.DSPop() != Cell(SpanAddr) {
		t.Fatalf("expected WordSpan to push %d", SpanAddr)
	}

	// Truncate at maxlen
	in2 := strings.NewReader("1234567890\n")
	f2 := New(in2, out)
	f2.DSPush(Cell(bufAddr))
	f2.DSPush(4)
	f2.WordExpect()
	if f2.Fetch(SpanAddr) != 4 {
		t.Fatalf("expected SPAN = 4, got %d", f2.Fetch(SpanAddr))
	}
}

func TestReadLineAndTIB(t *testing.T) {
	in := strings.NewReader("line1\r\nline2\nline3")
	out := &bytes.Buffer{}
	f := New(in, out)

	l1, err := f.ReadLine()
	if err != nil || l1 != "line1" {
		t.Fatalf("line1 error: %v, got %q", err, l1)
	}

	l2, err := f.ReadLine()
	if err != nil || l2 != "line2" {
		t.Fatalf("line2 error: %v, got %q", err, l2)
	}

	l3, err := f.ReadLine()
	if err != nil || l3 != "line3" {
		t.Fatalf("line3 error: %v, got %q", err, l3)
	}

	_, err = f.ReadLine()
	if err == nil {
		t.Fatal("expected EOF on 4th ReadLine")
	}

	// SetLine and TIB
	f.SetLine("10 20 +")
	if string(f.TIB) != "10 20 +" || f.IN != 0 {
		t.Fatalf("SetLine failed, TIB=%q, IN=%d", string(f.TIB), f.IN)
	}
}

func TestREPLPromptAndQUIT(t *testing.T) {
	in := strings.NewReader("line1\nline2\n")
	out := &bytes.Buffer{}
	f := New(in, out)

	f.QUIT()

	outStr := out.String()
	if !strings.Contains(outStr, " ok\n") {
		t.Fatalf("expected REPL prompt ' ok\\n', got %q", outStr)
	}
	if f.Running {
		t.Fatal("expected Running to be false after EOF")
	}
}

func TestInterruptRecovery(t *testing.T) {
	in := strings.NewReader("infinite_loop_line\n")
	out := &bytes.Buffer{}
	f := New(in, out)

	f.DSPush(100)
	f.DSPush(200)
	f.RSPush(300)

	// Simulate SIGINT
	f.Interrupted = 1

	f.replStep()

	// Stacks should be aborted/cleared
	if len(f.DS) != 0 {
		t.Fatalf("expected data stack to be cleared on interrupt, got %d items", len(f.DS))
	}
	if len(f.RS) != 0 {
		t.Fatalf("expected return stack to be cleared on interrupt, got %d items", len(f.RS))
	}

	outStr := out.String()
	if !strings.Contains(outStr, "INTERRUPTED") {
		t.Fatalf("expected 'INTERRUPTED' message, got %q", outStr)
	}
	if !strings.Contains(outStr, " ok\n") {
		t.Fatalf("expected ' ok\\n' prompt after interrupt, got %q", outStr)
	}

	// Engine should NOT be crashed or terminated
	if !f.Running {
		t.Fatal("SIGINT must not terminate the interpreter process")
	}
}

func TestAbort(t *testing.T) {
	in := &bytes.Buffer{}
	out := &bytes.Buffer{}
	f := New(in, out)

	f.DSPush(42)
	f.RSPush(99)
	f.State = true
	f.Interrupted = 1

	f.Abort()

	if len(f.DS) != 0 || len(f.RS) != 0 || f.State || f.Interrupted != 0 {
		t.Fatal("Abort did not reset all state")
	}
}

func TestStacksAndDepth(t *testing.T) {
	in := &bytes.Buffer{}
	out := &bytes.Buffer{}
	f := New(in, out)

	// DEPTH initially 0
	f.WordDepth()
	if f.DSPop() != 0 {
		t.Fatal("expected depth 0 initially")
	}

	// Data Stack operations
	f.DSPush(10)
	f.DSPush(20)
	f.DSPush(30)
	if f.DSPeek() != 30 {
		t.Fatalf("expected DSPeek 30, got %d", f.DSPeek())
	}
	f.WordDepth()
	if f.DSPop() != 3 {
		t.Fatalf("expected depth 3, got %d", f.DSPeek())
	}
	if f.DSPop() != 30 || f.DSPop() != 20 || f.DSPop() != 10 {
		t.Fatal("DS LIFO order violation")
	}

	// DS Peek underflow
	func() {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected DSPeek to panic on empty stack")
			}
		}()
		f.DSPeek()
	}()

	// Return Stack operations
	f.RSPush(100)
	f.RSPush(200)
	if f.RSPeek() != 200 {
		t.Fatalf("expected RSPeek 200, got %d", f.RSPeek())
	}
	if f.RSPop() != 200 || f.RSPop() != 100 {
		t.Fatal("RS LIFO order violation")
	}

	// RS Pop underflow
	func() {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected RSPop to panic on empty stack")
			}
		}()
		f.RSPop()
	}()

	// RS Peek underflow
	func() {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected RSPeek to panic on empty stack")
			}
		}()
		f.RSPeek()
	}()
}

func TestMemoryModelAndEndianness(t *testing.T) {
	in := &bytes.Buffer{}
	out := &bytes.Buffer{}
	f := New(in, out)

	if len(f.Memory) != 65536 {
		t.Fatalf("expected 65536 bytes of memory, got %d", len(f.Memory))
	}

	// Little-endian verification
	f.Store(2000, 0x1234)
	if f.Memory[2000] != 0x34 || f.Memory[2001] != 0x12 {
		t.Fatalf("expected little-endian bytes [0x34, 0x12], got [0x%x, 0x%x]", f.Memory[2000], f.Memory[2001])
	}
	if f.Fetch(2000) != 0x1234 {
		t.Fatalf("expected Fetch to return 0x1234, got 0x%x", f.Fetch(2000))
	}

	// Negative cell value round-trip
	f.Store(2002, -1)
	if f.Memory[2002] != 0xFF || f.Memory[2003] != 0xFF {
		t.Fatalf("expected -1 bytes [0xFF, 0xFF], got [0x%x, 0x%x]", f.Memory[2002], f.Memory[2003])
	}
	if f.Fetch(2002) != -1 {
		t.Fatalf("expected Fetch to return -1, got %d", f.Fetch(2002))
	}

	// Single byte access
	f.CStore(2004, 0xAB)
	if f.CFetch(2004) != 0xAB {
		t.Fatalf("expected CFetch to return 0xAB, got 0x%x", f.CFetch(2004))
	}

	// Boundary access (last valid 16-bit cell at 65534)
	f.Store(65534, -42)
	if f.Fetch(65534) != -42 {
		t.Fatalf("expected Fetch at 65534 to return -42, got %d", f.Fetch(65534))
	}

	// Bounds checks panics
	checkPanic := func(name string, fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatalf("expected panic for out of bounds %s", name)
			}
		}()
		fn()
	}

	checkPanic("CFetch(-1)", func() { f.CFetch(-1) })
	checkPanic("CFetch(65536)", func() { f.CFetch(65536) })
	checkPanic("CStore(-1)", func() { f.CStore(-1, 0) })
	checkPanic("CStore(65536)", func() { f.CStore(65536, 0) })

	checkPanic("Fetch(-1)", func() { f.Fetch(-1) })
	checkPanic("Fetch(65535)", func() { f.Fetch(65535) })
	checkPanic("Fetch(65536)", func() { f.Fetch(65536) })
	checkPanic("Store(-1)", func() { f.Store(-1, 0) })
	checkPanic("Store(65535)", func() { f.Store(65535, 0) })
	checkPanic("Store(65536)", func() { f.Store(65536, 0) })
}

func TestCellAndByteAccessPrimitives(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// ! and @
	f.DSPush(0x5678)
	f.DSPush(2000)
	f.WordStore()
	if f.Fetch(2000) != 0x5678 {
		t.Fatalf("WordStore failed, got 0x%x", f.Fetch(2000))
	}

	f.DSPush(2000)
	f.WordFetch()
	if len(f.DS) != 1 || f.DSPop() != 0x5678 {
		t.Fatal("WordFetch failed")
	}

	// C! and C@
	f.DSPush(0x7F)
	f.DSPush(2100)
	f.WordCStore()
	if f.CFetch(2100) != 0x7F {
		t.Fatalf("WordCStore failed, got 0x%x", f.CFetch(2100))
	}

	f.DSPush(2100)
	f.WordCFetch()
	if len(f.DS) != 1 || f.DSPop() != 0x7F {
		t.Fatal("WordCFetch failed")
	}

	// +!
	f.Store(2200, 100)
	f.DSPush(25)
	f.DSPush(2200)
	f.WordPlusStore()
	if f.Fetch(2200) != 125 {
		t.Fatalf("+! failed: expected 125, got %d", f.Fetch(2200))
	}

	f.DSPush(-50)
	f.DSPush(2200)
	f.WordPlusStore()
	if f.Fetch(2200) != 75 {
		t.Fatalf("+! with negative failed: expected 75, got %d", f.Fetch(2200))
	}

	// ?
	f.Store(2300, 4321)
	f.DSPush(2300)
	f.WordQuestion()
	if out.String() != "4321 " {
		t.Fatalf("? failed: expected '4321 ', got %q", out.String())
	}
	out.Reset()

	// Underflow tests
	underflow := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic")
			}
		}()
		fn()
	}

	underflow(func() { f.WordFetch() })
	underflow(func() { f.WordStore() })
	underflow(func() { f.DSPush(1); f.WordStore() })
	underflow(func() { f.WordCFetch() })
	underflow(func() { f.WordCStore() })
	underflow(func() { f.DSPush(1); f.WordCStore() })
	underflow(func() { f.WordPlusStore() })
	underflow(func() { f.DSPush(1); f.WordPlusStore() })
	underflow(func() { f.WordQuestion() })
}
