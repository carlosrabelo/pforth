package forth

import (
	"bytes"
	"os"
	"path/filepath"
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

func TestMemoryBlockOperations(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// FILL
	f.DSPush(3000) // addr
	f.DSPush(10)   // len
	f.DSPush(0x55) // val
	f.WordFill()
	for i := 0; i < 10; i++ {
		if f.CFetch(3000+i) != 0x55 {
			t.Fatalf("FILL failed at index %d, expected 0x55 got 0x%x", i, f.CFetch(3000+i))
		}
	}

	// ERASE
	f.DSPush(3000) // addr
	f.DSPush(10)   // len
	f.WordErase()
	for i := 0; i < 10; i++ {
		if f.CFetch(3000+i) != 0 {
			t.Fatalf("ERASE failed at index %d, expected 0 got 0x%x", i, f.CFetch(3000+i))
		}
	}

	// CMOVE non-overlapping: dst=9010, src=9000, len=5
	for i := 0; i < 5; i++ {
		f.CStore(9000+i, byte(i+10))
	}
	f.DSPush(9010) // dst
	f.DSPush(9000) // src
	f.DSPush(5)    // len
	f.WordCMove()
	for i := 0; i < 5; i++ {
		if f.CFetch(9010+i) != byte(i+10) {
			t.Fatalf("CMOVE failed at %d, expected %d got %d", 9010+i, i+10, f.CFetch(9010+i))
		}
	}

	// CMOVE forward overlapping with dst > src (propagates first byte)
	for i := 0; i < 5; i++ {
		f.CStore(9020+i, byte(i+20))
	}
	f.DSPush(9021) // dst
	f.DSPush(9020) // src
	f.DSPush(4)    // len
	f.WordCMove()
	if f.CFetch(9021) != 20 || f.CFetch(9022) != 20 || f.CFetch(9023) != 20 || f.CFetch(9024) != 20 {
		t.Fatalf("CMOVE forward overlap propagation failed: got [%d, %d, %d, %d]",
			f.CFetch(9021), f.CFetch(9022), f.CFetch(9023), f.CFetch(9024))
	}

	// CMOVE> backward overlapping with dst > src (correctly preserves values)
	for i := 0; i < 5; i++ {
		f.CStore(9030+i, byte(i+30))
	}
	f.DSPush(9031) // dst
	f.DSPush(9030) // src
	f.DSPush(4)    // len
	f.WordCMoveGreater()
	if f.CFetch(9031) != 30 || f.CFetch(9032) != 31 || f.CFetch(9033) != 32 || f.CFetch(9034) != 33 {
		t.Fatalf("CMOVE> backward overlap preservation failed: got [%d, %d, %d, %d]",
			f.CFetch(9031), f.CFetch(9032), f.CFetch(9033), f.CFetch(9034))
	}

	// Underflow tests
	underflow := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic")
			}
		}()
		fn()
	}

	underflow(func() { f.WordFill() })
	underflow(func() { f.DSPush(1); f.WordFill() })
	underflow(func() { f.DSPush(1); f.DSPush(1); f.WordFill() })
	underflow(func() { f.WordErase() })
	underflow(func() { f.DSPush(1); f.WordErase() })
	underflow(func() { f.WordCMove() })
	underflow(func() { f.DSPush(1); f.WordCMove() })
	underflow(func() { f.DSPush(1); f.DSPush(1); f.WordCMove() })
	underflow(func() { f.WordCMoveGreater() })
	underflow(func() { f.DSPush(1); f.WordCMoveGreater() })
	underflow(func() { f.DSPush(1); f.DSPush(1); f.WordCMoveGreater() })
}

func TestBasicStackManipulationWords(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// DUP ( n -- n n )
	f.DSPush(42)
	f.WordDup()
	if len(f.DS) != 2 || f.DSPop() != 42 || f.DSPop() != 42 {
		t.Fatal("DUP failed")
	}

	// DROP ( n -- )
	f.DSPush(10)
	f.DSPush(20)
	f.WordDrop()
	if len(f.DS) != 1 || f.DSPop() != 10 {
		t.Fatal("DROP failed")
	}

	// SWAP ( a b -- b a )
	f.DSPush(1)
	f.DSPush(2)
	f.WordSwap()
	if len(f.DS) != 2 || f.DSPop() != 1 || f.DSPop() != 2 {
		t.Fatal("SWAP failed")
	}

	// OVER ( a b -- a b a )
	f.DSPush(1)
	f.DSPush(2)
	f.WordOver()
	if len(f.DS) != 3 || f.DSPop() != 1 || f.DSPop() != 2 || f.DSPop() != 1 {
		t.Fatal("OVER failed")
	}

	// Underflow tests
	underflow := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic")
			}
		}()
		fn()
	}

	underflow(func() { f.WordDup() })
	underflow(func() { f.WordDrop() })
	underflow(func() { f.WordSwap() })
	underflow(func() { f.DSPush(1); f.WordSwap() })
	underflow(func() { f.WordOver() })
	underflow(func() { f.DSPush(1); f.WordOver() })
}

func TestExtendedSingleStackManipulationWords(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// ?DUP non-zero
	f.DSPush(42)
	f.WordQDup()
	if len(f.DS) != 2 || f.DSPop() != 42 || f.DSPop() != 42 {
		t.Fatal("?DUP non-zero failed")
	}

	// ?DUP zero
	f.DSPush(0)
	f.WordQDup()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("?DUP zero failed")
	}

	// ROT ( a b c -- b c a )
	f.DSPush(1)
	f.DSPush(2)
	f.DSPush(3)
	f.WordRot()
	if len(f.DS) != 3 || f.DSPop() != 1 || f.DSPop() != 3 || f.DSPop() != 2 {
		t.Fatal("ROT failed")
	}

	// -ROT ( a b c -- c a b )
	f.DSPush(1)
	f.DSPush(2)
	f.DSPush(3)
	f.WordNRot()
	if len(f.DS) != 3 || f.DSPop() != 2 || f.DSPop() != 1 || f.DSPop() != 3 {
		t.Fatal("-ROT failed")
	}

	// NIP ( a b -- b )
	f.DSPush(10)
	f.DSPush(20)
	f.WordNip()
	if len(f.DS) != 1 || f.DSPop() != 20 {
		t.Fatal("NIP failed")
	}

	// TUCK ( a b -- b a b )
	f.DSPush(10)
	f.DSPush(20)
	f.WordTuck()
	if len(f.DS) != 3 || f.DSPop() != 20 || f.DSPop() != 10 || f.DSPop() != 20 {
		t.Fatal("TUCK failed")
	}

	// Underflow tests
	underflow := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic")
			}
		}()
		fn()
	}

	underflow(func() { f.WordQDup() })
	underflow(func() { f.WordRot() })
	underflow(func() { f.DSPush(1); f.WordRot() })
	underflow(func() { f.DSPush(1); f.DSPush(2); f.WordRot() })
	underflow(func() { f.WordNRot() })
	underflow(func() { f.DSPush(1); f.WordNRot() })
	underflow(func() { f.DSPush(1); f.DSPush(2); f.WordNRot() })
	underflow(func() { f.WordNip() })
	underflow(func() { f.DSPush(1); f.WordNip() })
	underflow(func() { f.WordTuck() })
	underflow(func() { f.DSPush(1); f.WordTuck() })
}

func TestDoubleCellStackManipulationWords(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// 2DUP ( a b -- a b a b )
	f.DSPush(10)
	f.DSPush(20)
	f.Word2Dup()
	if len(f.DS) != 4 || f.DSPop() != 20 || f.DSPop() != 10 || f.DSPop() != 20 || f.DSPop() != 10 {
		t.Fatal("2DUP failed")
	}

	// 2DROP ( a b -- )
	f.DSPush(10)
	f.DSPush(20)
	f.Word2Drop()
	if len(f.DS) != 0 {
		t.Fatal("2DROP failed")
	}

	// 2SWAP ( a b c d -- c d a b )
	f.DSPush(1)
	f.DSPush(2)
	f.DSPush(3)
	f.DSPush(4)
	f.Word2Swap()
	if len(f.DS) != 4 || f.DSPop() != 2 || f.DSPop() != 1 || f.DSPop() != 4 || f.DSPop() != 3 {
		t.Fatal("2SWAP failed")
	}

	// 2OVER ( a b c d -- a b c d a b )
	f.DSPush(1)
	f.DSPush(2)
	f.DSPush(3)
	f.DSPush(4)
	f.Word2Over()
	if len(f.DS) != 6 ||
		f.DSPop() != 2 || f.DSPop() != 1 ||
		f.DSPop() != 4 || f.DSPop() != 3 ||
		f.DSPop() != 2 || f.DSPop() != 1 {
		t.Fatal("2OVER failed")
	}

	// Underflow tests
	underflow := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic")
			}
		}()
		fn()
	}

	underflow(func() { f.Word2Dup() })
	underflow(func() { f.DSPush(1); f.Word2Dup() })
	underflow(func() { f.Word2Drop() })
	underflow(func() { f.DSPush(1); f.Word2Drop() })
	underflow(func() { f.Word2Swap() })
	underflow(func() { f.DSPush(1); f.Word2Swap() })
	underflow(func() { f.DSPush(1); f.DSPush(2); f.Word2Swap() })
	underflow(func() { f.DSPush(1); f.DSPush(2); f.DSPush(3); f.Word2Swap() })
	underflow(func() { f.Word2Over() })
	underflow(func() { f.DSPush(1); f.Word2Over() })
	underflow(func() { f.DSPush(1); f.DSPush(2); f.Word2Over() })
	underflow(func() { f.DSPush(1); f.DSPush(2); f.DSPush(3); f.Word2Over() })
}

func TestReturnStackTransferWords(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// >R ( n -- ) ( R: -- n )
	f.DSPush(99)
	f.WordToR()
	if len(f.DS) != 0 || len(f.RS) != 1 || f.RSPeek() != 99 {
		t.Fatal(">R failed")
	}

	// R@ ( -- n ) ( R: n -- n )
	f.WordRFetch()
	if len(f.DS) != 1 || f.DSPeek() != 99 || len(f.RS) != 1 || f.RSPeek() != 99 {
		t.Fatal("R@ failed")
	}

	// R> ( -- n ) ( R: n -- )
	f.WordRFrom()
	if len(f.RS) != 0 || len(f.DS) != 2 || f.DSPop() != 99 || f.DSPop() != 99 {
		t.Fatal("R> failed")
	}

	// Underflow checks
	underflow := func(expectedMsg string, fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatalf("expected underflow panic with message %q", expectedMsg)
			} else {
				fe, ok := r.(*ForthError)
				if !ok || fe.Message != expectedMsg {
					t.Fatalf("expected %q, got %v", expectedMsg, r)
				}
			}
		}()
		fn()
	}

	underflow("DATA STACK UNDERFLOW", func() { f.WordToR() })
	underflow("RETURN STACK UNDERFLOW", func() { f.WordRFrom() })
	underflow("RETURN STACK UNDERFLOW", func() { f.WordRFetch() })
}

func TestBasicArithmeticAndDivisionPrimitives(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// +
	f.DSPush(10)
	f.DSPush(20)
	f.WordPlus()
	if len(f.DS) != 1 || f.DSPop() != 30 {
		t.Fatal("+ failed")
	}

	// - ( 20 7 - -> 13 )
	f.DSPush(20)
	f.DSPush(7)
	f.WordMinus()
	if len(f.DS) != 1 || f.DSPop() != 13 {
		t.Fatal("- failed")
	}

	// *
	f.DSPush(6)
	f.DSPush(7)
	f.WordMultiply()
	if len(f.DS) != 1 || f.DSPop() != 42 {
		t.Fatal("* failed")
	}

	// /MOD ( 17 5 /MOD -> 2 rem, 3 quot )
	f.DSPush(17)
	f.DSPush(5)
	f.WordSlashMod()
	if len(f.DS) != 2 || f.DSPop() != 3 || f.DSPop() != 2 {
		t.Fatal("/MOD failed")
	}

	// /
	f.DSPush(20)
	f.DSPush(4)
	f.WordSlash()
	if len(f.DS) != 1 || f.DSPop() != 5 {
		t.Fatal("/ failed")
	}

	// MOD
	f.DSPush(23)
	f.DSPush(7)
	f.WordMod()
	if len(f.DS) != 1 || f.DSPop() != 2 {
		t.Fatal("MOD failed")
	}

	// NEGATE
	f.DSPush(10)
	f.WordNegate()
	if len(f.DS) != 1 || f.DSPop() != -10 {
		t.Fatal("NEGATE failed")
	}
	f.DSPush(-5)
	f.WordNegate()
	if len(f.DS) != 1 || f.DSPop() != 5 {
		t.Fatal("NEGATE negative failed")
	}

	// Division by zero
	expectDivZero := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected division by zero panic")
			} else {
				fe, ok := r.(*ForthError)
				if !ok || fe.Message != "DIVISION BY ZERO" {
					t.Fatalf("expected DIVISION BY ZERO, got %v", r)
				}
			}
		}()
		fn()
	}

	f.DSPush(10)
	f.DSPush(0)
	expectDivZero(func() { f.WordSlashMod() })

	f.DSPush(10)
	f.DSPush(0)
	expectDivZero(func() { f.WordSlash() })

	f.DSPush(10)
	f.DSPush(0)
	expectDivZero(func() { f.WordMod() })

	// Underflow tests
	underflow := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic")
			}
		}()
		fn()
	}

	underflow(func() { f.WordPlus() })
	underflow(func() { f.DSPush(1); f.WordPlus() })
	underflow(func() { f.WordMinus() })
	underflow(func() { f.DSPush(1); f.WordMinus() })
	underflow(func() { f.WordMultiply() })
	underflow(func() { f.DSPush(1); f.WordMultiply() })
	underflow(func() { f.WordSlashMod() })
	underflow(func() { f.DSPush(1); f.WordSlashMod() })
	underflow(func() { f.WordSlash() })
	underflow(func() { f.DSPush(1); f.WordSlash() })
	underflow(func() { f.WordMod() })
	underflow(func() { f.DSPush(1); f.WordMod() })
	underflow(func() { f.WordNegate() })
}

func TestArithmeticScalingAndHelperWords(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// 1+
	f.DSPush(5)
	f.WordOnePlus()
	if len(f.DS) != 1 || f.DSPop() != 6 {
		t.Fatal("1+ failed")
	}

	// 1-
	f.DSPush(5)
	f.WordOneMinus()
	if len(f.DS) != 1 || f.DSPop() != 4 {
		t.Fatal("1- failed")
	}

	// 2+
	f.DSPush(5)
	f.WordTwoPlus()
	if len(f.DS) != 1 || f.DSPop() != 7 {
		t.Fatal("2+ failed")
	}

	// 2-
	f.DSPush(5)
	f.WordTwoMinus()
	if len(f.DS) != 1 || f.DSPop() != 3 {
		t.Fatal("2- failed")
	}

	// 2*
	f.DSPush(5)
	f.WordTwoStar()
	if len(f.DS) != 1 || f.DSPop() != 10 {
		t.Fatal("2* failed")
	}
	f.DSPush(-3)
	f.WordTwoStar()
	if len(f.DS) != 1 || f.DSPop() != -6 {
		t.Fatal("2* negative failed")
	}

	// 2/
	f.DSPush(10)
	f.WordTwoSlash()
	if len(f.DS) != 1 || f.DSPop() != 5 {
		t.Fatal("2/ failed")
	}
	f.DSPush(-6)
	f.WordTwoSlash()
	if len(f.DS) != 1 || f.DSPop() != -3 {
		t.Fatal("2/ negative failed")
	}

	// ABS
	f.DSPush(-5)
	f.WordAbs()
	if len(f.DS) != 1 || f.DSPop() != 5 {
		t.Fatal("ABS negative failed")
	}
	f.DSPush(5)
	f.WordAbs()
	if len(f.DS) != 1 || f.DSPop() != 5 {
		t.Fatal("ABS positive failed")
	}

	// MIN
	f.DSPush(10)
	f.DSPush(20)
	f.WordMin()
	if len(f.DS) != 1 || f.DSPop() != 10 {
		t.Fatal("MIN failed")
	}
	f.DSPush(-5)
	f.DSPush(-10)
	f.WordMin()
	if len(f.DS) != 1 || f.DSPop() != -10 {
		t.Fatal("MIN negative failed")
	}

	// MAX
	f.DSPush(10)
	f.DSPush(20)
	f.WordMax()
	if len(f.DS) != 1 || f.DSPop() != 20 {
		t.Fatal("MAX failed")
	}
	f.DSPush(-5)
	f.DSPush(-10)
	f.WordMax()
	if len(f.DS) != 1 || f.DSPop() != -5 {
		t.Fatal("MAX negative failed")
	}

	// Underflow tests
	underflow := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic")
			}
		}()
		fn()
	}

	underflow(func() { f.WordOnePlus() })
	underflow(func() { f.WordOneMinus() })
	underflow(func() { f.WordTwoPlus() })
	underflow(func() { f.WordTwoMinus() })
	underflow(func() { f.WordTwoStar() })
	underflow(func() { f.WordTwoSlash() })
	underflow(func() { f.WordAbs() })
	underflow(func() { f.WordMin() })
	underflow(func() { f.DSPush(1); f.WordMin() })
	underflow(func() { f.WordMax() })
	underflow(func() { f.DSPush(1); f.WordMax() })
}

func TestBitwiseLogicPrimitives(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// AND
	f.DSPush(0x00FF)
	f.DSPush(0x0F0F)
	f.WordAnd()
	if len(f.DS) != 1 || f.DSPop() != 0x000F {
		t.Fatal("AND failed")
	}

	// OR
	f.DSPush(0x00F0)
	f.DSPush(0x000F)
	f.WordOr()
	if len(f.DS) != 1 || f.DSPop() != 0x00FF {
		t.Fatal("OR failed")
	}

	// XOR
	f.DSPush(0x00FF)
	f.DSPush(0x000F)
	f.WordXor()
	if len(f.DS) != 1 || f.DSPop() != 0x00F0 {
		t.Fatal("XOR failed")
	}
	f.DSPush(42)
	f.DSPush(42)
	f.WordXor()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("XOR identical failed")
	}

	// INVERT
	f.DSPush(0)
	f.WordInvert()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("INVERT 0 failed")
	}
	f.DSPush(-1)
	f.WordInvert()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("INVERT -1 failed")
	}

	// Underflow tests
	underflow := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic")
			}
		}()
		fn()
	}

	underflow(func() { f.WordAnd() })
	underflow(func() { f.DSPush(1); f.WordAnd() })
	underflow(func() { f.WordOr() })
	underflow(func() { f.DSPush(1); f.WordOr() })
	underflow(func() { f.WordXor() })
	underflow(func() { f.DSPush(1); f.WordXor() })
	underflow(func() { f.WordInvert() })
}

func TestComparisonAndEqualityPrimitives(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// =
	f.DSPush(10)
	f.DSPush(10)
	f.WordEquals()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("= true failed")
	}
	f.DSPush(10)
	f.DSPush(20)
	f.WordEquals()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("= false failed")
	}

	// <>
	f.DSPush(10)
	f.DSPush(20)
	f.WordNotEquals()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("<> true failed")
	}
	f.DSPush(10)
	f.DSPush(10)
	f.WordNotEquals()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("<> false failed")
	}

	// <
	f.DSPush(5)
	f.DSPush(10)
	f.WordLess()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("< true failed")
	}
	f.DSPush(10)
	f.DSPush(5)
	f.WordLess()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("< false failed")
	}
	f.DSPush(-5)
	f.DSPush(5)
	f.WordLess()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("< signed negative failed")
	}

	// >
	f.DSPush(10)
	f.DSPush(5)
	f.WordGreater()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("> true failed")
	}
	f.DSPush(5)
	f.DSPush(10)
	f.WordGreater()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("> false failed")
	}

	// <=
	f.DSPush(5)
	f.DSPush(10)
	f.WordLessEqual()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("<= less failed")
	}
	f.DSPush(10)
	f.DSPush(10)
	f.WordLessEqual()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("<= equal failed")
	}
	f.DSPush(10)
	f.DSPush(5)
	f.WordLessEqual()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("<= false failed")
	}

	// >=
	f.DSPush(10)
	f.DSPush(5)
	f.WordGreaterEqual()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal(">= greater failed")
	}
	f.DSPush(10)
	f.DSPush(10)
	f.WordGreaterEqual()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal(">= equal failed")
	}
	f.DSPush(5)
	f.DSPush(10)
	f.WordGreaterEqual()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal(">= false failed")
	}

	// 0=
	f.DSPush(0)
	f.WordZeroEquals()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("0= true failed")
	}
	f.DSPush(5)
	f.WordZeroEquals()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("0= false failed")
	}

	// 0<>
	f.DSPush(5)
	f.WordZeroNotEquals()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("0<> true failed")
	}
	f.DSPush(0)
	f.WordZeroNotEquals()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("0<> false failed")
	}

	// 0<
	f.DSPush(-5)
	f.WordZeroLess()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("0< true failed")
	}
	f.DSPush(5)
	f.WordZeroLess()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("0< false failed")
	}

	// 0>
	f.DSPush(5)
	f.WordZeroGreater()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("0> true failed")
	}
	f.DSPush(-5)
	f.WordZeroGreater()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("0> false failed")
	}

	// U<, U>, U<=, U>=
	// -1 signed is 65535 unsigned
	f.DSPush(-1)
	f.DSPush(1)
	f.WordULess()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("U< unsigned check failed")
	}
	f.DSPush(-1)
	f.DSPush(1)
	f.WordUGreater()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("U> unsigned check failed")
	}

	f.DSPush(10)
	f.DSPush(10)
	f.WordULessEqual()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("U<= equal failed")
	}
	f.DSPush(20)
	f.DSPush(10)
	f.WordULessEqual()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("U<= false failed")
	}

	f.DSPush(10)
	f.DSPush(10)
	f.WordUGreaterEqual()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("U>= equal failed")
	}
	f.DSPush(5)
	f.DSPush(10)
	f.WordUGreaterEqual()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("U>= false failed")
	}

	// Underflow tests
	underflow := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic")
			}
		}()
		fn()
	}

	underflow(func() { f.WordEquals() })
	underflow(func() { f.DSPush(1); f.WordEquals() })
	underflow(func() { f.WordNotEquals() })
	underflow(func() { f.DSPush(1); f.WordNotEquals() })
	underflow(func() { f.WordLess() })
	underflow(func() { f.DSPush(1); f.WordLess() })
	underflow(func() { f.WordGreater() })
	underflow(func() { f.DSPush(1); f.WordGreater() })
	underflow(func() { f.WordLessEqual() })
	underflow(func() { f.DSPush(1); f.WordLessEqual() })
	underflow(func() { f.WordGreaterEqual() })
	underflow(func() { f.DSPush(1); f.WordGreaterEqual() })
	underflow(func() { f.WordZeroEquals() })
	underflow(func() { f.WordZeroNotEquals() })
	underflow(func() { f.WordZeroLess() })
	underflow(func() { f.WordZeroGreater() })
	underflow(func() { f.WordULess() })
	underflow(func() { f.DSPush(1); f.WordULess() })
	underflow(func() { f.WordUGreater() })
	underflow(func() { f.DSPush(1); f.WordUGreater() })
	underflow(func() { f.WordULessEqual() })
	underflow(func() { f.DSPush(1); f.WordULessEqual() })
	underflow(func() { f.WordUGreaterEqual() })
	underflow(func() { f.DSPush(1); f.WordUGreaterEqual() })
}

func TestDictionaryStructureAndLookup(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// DefinePrimitive & DefineImmediate
	w1 := f.DefinePrimitive("MYWORD", func(f *Forth) {})
	if w1 == nil || w1.Name != "MYWORD" || w1.Immediate {
		t.Fatal("DefinePrimitive failed")
	}

	w2 := f.DefineImmediate("MYIMM", func(f *Forth) {})
	if w2 == nil || w2.Name != "MYIMM" || !w2.Immediate {
		t.Fatal("DefineImmediate failed")
	}

	if f.LatestWord() != w2 {
		t.Fatal("LatestWord failed")
	}

	// Lookup & FindXT
	foundW, ok := f.Lookup("myword")
	if !ok || foundW != w1 {
		t.Fatal("Lookup case-insensitive failed")
	}

	xt1, ok := f.FindXT("MYWORD")
	if !ok || xt1 != 0 {
		t.Fatalf("FindXT expected 0, got %d", xt1)
	}

	xt2, ok := f.FindXT("MYIMM")
	if !ok || xt2 != 1 {
		t.Fatalf("FindXT expected 1, got %d", xt2)
	}

	// FIND non-immediate
	// Counted string at 3000: length 6, "MYWORD"
	f.CStore(3000, 6)
	for i, c := range []byte("MYWORD") {
		f.CStore(3001+i, c)
	}
	f.DSPush(3000)
	f.WordFind()
	if len(f.DS) != 2 || f.DSPop() != -1 || f.DSPop() != Cell(xt1) {
		t.Fatal("FIND non-immediate failed")
	}

	// FIND immediate
	// Counted string at 3010: length 5, "myimm"
	f.CStore(3010, 5)
	for i, c := range []byte("myimm") {
		f.CStore(3011+i, c)
	}
	f.DSPush(3010)
	f.WordFind()
	if len(f.DS) != 2 || f.DSPop() != 1 || f.DSPop() != Cell(xt2) {
		t.Fatal("FIND immediate failed")
	}

	// FIND unknown
	f.CStore(3020, 7)
	for i, c := range []byte("UNKNOWN") {
		f.CStore(3021+i, c)
	}
	f.DSPush(3020)
	f.WordFind()
	if len(f.DS) != 2 || f.DSPop() != 0 || f.DSPop() != 3020 {
		t.Fatal("FIND unknown failed")
	}

	// ' (tick)
	f.SetLine("  MYWORD   MYIMM ")
	f.WordTick()
	if len(f.DS) != 1 || f.DSPop() != Cell(xt1) {
		t.Fatal("tick MYWORD failed")
	}
	f.WordTick()
	if len(f.DS) != 1 || f.DSPop() != Cell(xt2) {
		t.Fatal("tick MYIMM failed")
	}

	// tick missing word name
	func() {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected panic on tick with empty line")
			}
		}()
		f.WordTick()
	}()

	// tick word not found
	f.SetLine("NONEXISTENT")
	func() {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected panic on tick with unknown word")
			}
		}()
		f.WordTick()
	}()

	// >BODY
	w1.PFA = 2050
	f.DSPush(Cell(xt1))
	f.WordToBody()
	if len(f.DS) != 1 || f.DSPop() != 2050 {
		t.Fatal(">BODY failed")
	}

	// Underflow tests
	underflow := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic")
			}
		}()
		fn()
	}

	underflow(func() { f.WordFind() })
	underflow(func() { f.WordToBody() })
}

func TestDictionaryMemoryAllocationPointers(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// DP ( -- addr )
	f.WordDP()
	if len(f.DS) != 1 || f.DSPop() != Cell(DPAddr) {
		t.Fatalf("DP expected %d", DPAddr)
	}

	// HERE initial
	f.WordHere()
	if len(f.DS) != 1 || f.DSPop() != 1024 {
		t.Fatal("HERE initial expected 1024")
	}

	// ALLOT 10
	f.DSPush(10)
	f.WordAllot()
	if f.Here() != 1034 {
		t.Fatalf("ALLOT 10 expected HERE=1034, got %d", f.Here())
	}

	// , ( n -- )
	f.DSPush(0x1234)
	f.WordComma()
	if f.Here() != 1036 {
		t.Fatalf(", expected HERE=1036, got %d", f.Here())
	}
	if f.Fetch(1034) != 0x1234 {
		t.Fatalf(", stored value mismatch, got 0x%x", f.Fetch(1034))
	}

	// C, ( c -- )
	f.DSPush(0x55)
	f.WordCComma()
	if f.Here() != 1037 {
		t.Fatalf("C, expected HERE=1037, got %d", f.Here())
	}
	if f.CFetch(1036) != 0x55 {
		t.Fatalf("C, stored value mismatch, got 0x%x", f.CFetch(1036))
	}

	// PAD ( -- addr ) (HERE + 68)
	f.WordPAD()
	if len(f.DS) != 1 || f.DSPop() != Cell(1037+68) {
		t.Fatalf("PAD expected %d", 1037+68)
	}

	// Underflow tests
	underflow := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic")
			}
		}()
		fn()
	}

	underflow(func() { f.WordAllot() })
	underflow(func() { f.WordComma() })
	underflow(func() { f.WordCComma() })
}

func TestExecutionTokenDispatcher(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// Primitive word execution
	called := false
	wPrim := f.DefinePrimitive("TESTPRIM", func(f *Forth) {
		called = true
		f.DSPush(777)
	})
	xtPrim, _ := f.FindXT("TESTPRIM")
	f.DSPush(Cell(xtPrim))
	f.WordExecute()
	if !called || len(f.DS) != 1 || f.DSPop() != 777 {
		t.Fatal("ExecuteWord primitive failed")
	}

	// Constant word execution
	wConst := f.DefineWord("TESTCONST", WordConstant, nil, []Cell{1234}, false)
	xtConst, _ := f.FindXT("TESTCONST")
	f.DSPush(Cell(xtConst))
	f.WordExecute()
	if len(f.DS) != 1 || f.DSPop() != 1234 {
		t.Fatal("ExecuteWord constant failed")
	}
	_ = wConst

	// Variable word execution
	wVar := f.DefineWord("TESTVAR", WordVariable, nil, nil, false)
	wVar.PFA = 2048
	xtVar, _ := f.FindXT("TESTVAR")
	f.DSPush(Cell(xtVar))
	f.WordExecute()
	if len(f.DS) != 1 || f.DSPop() != 2048 {
		t.Fatal("ExecuteWord variable failed")
	}

	// Create word execution
	wCreate := f.DefineWord("TESTCREATE", WordCreate, nil, nil, false)
	wCreate.PFA = 3050
	xtCreate, _ := f.FindXT("TESTCREATE")
	f.DSPush(Cell(xtCreate))
	f.WordExecute()
	if len(f.DS) != 1 || f.DSPop() != 3050 {
		t.Fatal("ExecuteWord create failed")
	}

	// Invalid XT panic
	invalidXT := func(xt Cell) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatalf("expected panic on invalid XT %d", xt)
			}
		}()
		f.DSPush(xt)
		f.WordExecute()
	}
	invalidXT(-1)
	invalidXT(9999)

	// Underflow
	func() {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic on WordExecute")
			}
		}()
		f.WordExecute()
	}()
	_ = wPrim
}

func TestDefiningWords(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// CREATE
	f.SetLine("MYBUF")
	f.WordCreate()
	xtBuf, ok := f.FindXT("MYBUF")
	if !ok {
		t.Fatal("CREATE MYBUF not found")
	}
	// Execute MYBUF -> pushes PFA (1024)
	f.DSPush(Cell(xtBuf))
	f.WordExecute()
	if len(f.DS) != 1 || f.DSPop() != 1024 {
		t.Fatal("MYBUF execute expected PFA=1024")
	}
	f.Allot(10) // Allot buffer bytes

	// CONSTANT ( n -- )
	f.DSPush(42)
	f.SetLine("ANSWER")
	f.WordConstant()
	xtConst, ok := f.FindXT("ANSWER")
	if !ok {
		t.Fatal("CONSTANT ANSWER not found")
	}
	f.DSPush(Cell(xtConst))
	f.WordExecute()
	if len(f.DS) != 1 || f.DSPop() != 42 {
		t.Fatal("ANSWER execute expected 42")
	}

	// VARIABLE ( -- )
	f.SetLine("COUNTER")
	f.WordVariable()
	xtVar, ok := f.FindXT("COUNTER")
	if !ok {
		t.Fatal("VARIABLE COUNTER not found")
	}
	// Initial value at PFA should be 0
	f.DSPush(Cell(xtVar))
	f.WordExecute()
	pfa := int(f.DSPop())
	if f.Fetch(pfa) != 0 {
		t.Fatalf("COUNTER initial value expected 0, got %d", f.Fetch(pfa))
	}
	// Store 99 and fetch
	f.Store(pfa, 99)
	if f.Fetch(pfa) != 99 {
		t.Fatalf("COUNTER stored value expected 99, got %d", f.Fetch(pfa))
	}

	// Missing name panic
	missingName := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected panic on missing name")
			}
		}()
		f.SetLine("")
		fn()
	}
	missingName(func() { f.WordCreate() })
	missingName(func() { f.DSPush(1); f.WordConstant() })
	missingName(func() { f.WordVariable() })

	// Underflow on CONSTANT
	func() {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic on CONSTANT")
			}
		}()
		f.SetLine("FAIL")
		f.WordConstant()
	}()
}

func TestDelimitedWordScannerAndNumericParser(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// BASE ( -- addr )
	f.WordBase()
	if len(f.DS) != 1 || f.DSPop() != Cell(BaseAddr) {
		t.Fatalf("BASE expected %d", BaseAddr)
	}
	if f.GetBase() != 10 {
		t.Fatalf("default BASE expected 10, got %d", f.GetBase())
	}
	f.SetBase(16)
	if f.GetBase() != 16 {
		t.Fatalf("updated BASE expected 16, got %d", f.GetBase())
	}
	f.SetBase(10)

	// ParseNumber
	checkNum := func(token string, expected Cell) {
		val, ok := f.ParseNumber(token)
		if !ok || val != expected {
			t.Fatalf("ParseNumber(%q) expected (%d, true), got (%d, %v)", token, expected, val, ok)
		}
	}
	checkNum("123", 123)
	checkNum("-45", -45)
	checkNum("+50", 50)
	checkNum("$1F", 31)
	checkNum("-$10", -16)
	checkNum("%1010", 10)
	checkNum("-%11", -3)

	f.SetBase(16)
	checkNum("FF", 255)
	f.SetBase(10)

	// Invalid numbers
	checkInvalid := func(token string) {
		_, ok := f.ParseNumber(token)
		if ok {
			t.Fatalf("ParseNumber(%q) expected false, got true", token)
		}
	}
	checkInvalid("12A")
	checkInvalid("XYZ")
	checkInvalid("$")
	checkInvalid("-")
	checkInvalid("")

	// WORD ( delim -- addr )
	f.SetLine("   HELLO   WORLD  ")
	f.DSPush(Cell(' '))
	f.WordWord()
	addr1 := int(f.DSPop())
	len1 := int(f.CFetch(addr1))
	if len1 != 5 {
		t.Fatalf("WORD HELLO length expected 5, got %d", len1)
	}
	var s1 strings.Builder
	for i := 0; i < len1; i++ {
		s1.WriteByte(f.CFetch(addr1 + 1 + i))
	}
	if s1.String() != "HELLO" {
		t.Fatalf("WORD HELLO expected 'HELLO', got %q", s1.String())
	}

	f.DSPush(Cell(' '))
	f.WordWord()
	addr2 := int(f.DSPop())
	len2 := int(f.CFetch(addr2))
	if len2 != 5 {
		t.Fatalf("WORD WORLD length expected 5, got %d", len2)
	}
	var s2 strings.Builder
	for i := 0; i < len2; i++ {
		s2.WriteByte(f.CFetch(addr2 + 1 + i))
	}
	if s2.String() != "WORLD" {
		t.Fatalf("WORD WORLD expected 'WORLD', got %q", s2.String())
	}

	// NUMBER ( addr -- n flag )
	f.DSPush(Cell(addr1)) // "HELLO" is not a number
	f.WordNumber()
	flag1 := f.DSPop()
	_ = f.DSPop()
	if flag1 != 0 {
		t.Fatal("NUMBER HELLO should fail")
	}

	// Create counted string for "123" at 3500
	f.CStore(3500, 3)
	f.CStore(3501, '1')
	f.CStore(3502, '2')
	f.CStore(3503, '3')
	f.DSPush(3500)
	f.WordNumber()
	flag2 := f.DSPop()
	val2 := f.DSPop()
	if flag2 != -1 || val2 != 123 {
		t.Fatalf("NUMBER '123' expected (123, -1), got (%d, %d)", val2, flag2)
	}

	// Underflow tests
	underflow := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic")
			}
		}()
		fn()
	}

	underflow(func() { f.WordWord() })
	underflow(func() { f.WordNumber() })
}

func TestOuterInterpreterLoop(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// Define primitive +
	f.DefinePrimitive("+", func(f *Forth) { f.WordPlus() })

	// InterpretLine with numbers and +
	f.InterpretLine("10 20 +")
	if len(f.DS) != 1 || f.DSPop() != 30 {
		t.Fatal("InterpretLine '10 20 +' failed")
	}

	// Unknown word error
	func() {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected panic on unknown word")
			} else {
				fe, ok := r.(*ForthError)
				if !ok || !strings.Contains(fe.Message, "UNKNOWN WORD: BOGUS") {
					t.Fatalf("unexpected panic message: %v", r)
				}
			}
		}()
		f.InterpretLine("BOGUS")
	}()

	// ABORT clears stacks and state
	f.DSPush(1)
	f.RSPush(2)
	f.State = true
	f.WordAbort()
	if len(f.DS) != 0 || len(f.RS) != 0 || f.State {
		t.Fatal("WordAbort failed to reset state")
	}

	// QUIT in REPL
	in := strings.NewReader("100 200 +\n")
	f2 := New(in, out)
	f2.DefinePrimitive("+", func(f *Forth) { f.WordPlus() })
	f2.QUIT()
	if len(f2.DS) != 1 || f2.DSPop() != 300 {
		t.Fatal("QUIT REPL loop execution failed")
	}
}

func TestCompilerStateAndLiteralCompilation(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// STATE initially 0
	f.WordState()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("STATE initially expected 0")
	}

	// ] sets STATE to -1
	f.WordRightBracket()
	f.WordState()
	if len(f.DS) != 1 || f.DSPop() != -1 {
		t.Fatal("] expected STATE = -1")
	}

	// [ sets STATE to 0
	f.WordLeftBracket()
	f.WordState()
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatal("[ expected STATE = 0")
	}

	// LITERAL ( n -- )
	f.CompileList = f.CompileList[:0]
	f.DSPush(42)
	f.WordLiteral()
	litXT, ok := f.FindXT("LIT")
	if !ok {
		t.Fatal("LIT should be defined after LITERAL")
	}
	if len(f.CompileList) != 2 || f.CompileList[0] != Cell(litXT) || f.CompileList[1] != 42 {
		t.Fatalf("LITERAL compilation mismatch: %v", f.CompileList)
	}

	// LIT runtime execution
	f.CurrentBody = []Cell{999}
	f.IP = 0
	f.WordLIT()
	if len(f.DS) != 1 || f.DSPop() != 999 || f.IP != 1 {
		t.Fatal("WordLIT runtime failed")
	}

	// COMPILE
	wFoo := f.DefinePrimitive("FOO", func(f *Forth) {})
	xtFoo, _ := f.FindXT("FOO")
	f.CompileList = f.CompileList[:0]
	f.SetLine("FOO")
	f.WordCompile()
	if len(f.CompileList) != 1 || f.CompileList[0] != Cell(xtFoo) {
		t.Fatal("COMPILE failed")
	}
	_ = wFoo

	// [COMPILE]
	wBar := f.DefineImmediate("BAR", func(f *Forth) {})
	xtBar, _ := f.FindXT("BAR")
	f.CompileList = f.CompileList[:0]
	f.SetLine("BAR")
	f.WordBracketCompile()
	if len(f.CompileList) != 1 || f.CompileList[0] != Cell(xtBar) {
		t.Fatal("[COMPILE] failed")
	}
	_ = wBar

	// Underflow tests
	underflow := func(fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatal("expected underflow panic")
			}
		}()
		fn()
	}

	underflow(func() { f.WordLiteral() })
}

func TestColonDefinitionCompiler(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()

	// 1. Colon definition and execution: : SQUARE DUP * ; 5 SQUARE
	f.InterpretLine(": SQUARE DUP * ;")
	if f.State {
		t.Fatal("expected State == false after definition")
	}
	sqW, ok := f.Lookup("SQUARE")
	if !ok || sqW.Type != WordColon {
		t.Fatal("SQUARE should be registered as WordColon")
	}

	f.InterpretLine("5 SQUARE")
	if len(f.DS) != 1 || f.DSPop() != 25 {
		t.Fatalf("5 SQUARE expected 25, got DS: %v", f.DS)
	}

	// 2. IMMEDIATE word execution during compilation
	calledImm := false
	f.DefineImmediate("HOOK", func(f *Forth) {
		calledImm = true
	})
	f.InterpretLine(": TESTHOOK HOOK ;")
	if !calledImm {
		t.Fatal("HOOK was not executed during compilation")
	}

	// 3. IMMEDIATE word modifying latest
	f.InterpretLine(": MYIMM ;")
	f.WordImmediate()
	myImmW, _ := f.Lookup("MYIMM")
	if !myImmW.Immediate {
		t.Fatal("IMMEDIATE failed to mark word")
	}

	// 4. RECURSE compiles compiling XT
	f.InterpretLine(": MYREC RECURSE ;")
	recW, _ := f.Lookup("MYREC")
	recXT, _ := f.FindXT("MYREC")
	exitXT, _ := f.FindXT("EXIT")
	if len(recW.Body) != 2 || recW.Body[0] != Cell(recXT) || recW.Body[1] != Cell(exitXT) {
		t.Fatalf("RECURSE body mismatch: %v", recW.Body)
	}

	// 5. Error handling
	expectPanic := func(name string, fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatalf("%s expected panic", name)
			}
		}()
		fn()
	}

	expectPanic("already compiling", func() {
		f.State = true
		f.WordColon()
	})
	f.State = false

	expectPanic("missing word name", func() {
		f.SetLine("")
		f.WordColon()
	})

	expectPanic("semicolon when not compiling", func() {
		f.State = false
		f.WordSemicolon()
	})

	expectPanic("recurse when not compiling", func() {
		f.State = false
		f.WordRecurse()
	})

	expectPanic("recurse with invalid compiling XT", func() {
		f.State = true
		f.CompilingXT = -1
		f.WordRecurse()
	})
	f.State = false
}

func TestRuntimeBranchExecution(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)

	// 1. Unconditional BRANCH forward
	f.CurrentBody = []Cell{5}
	f.IP = 0
	f.WordBranch()
	if f.IP != 5 {
		t.Fatalf("BRANCH forward expected IP=5, got %d", f.IP)
	}

	// 2. Unconditional BRANCH backward
	f.CurrentBody = []Cell{10, 20, 30, -2}
	f.IP = 3
	f.WordBranch()
	if f.IP != 1 {
		t.Fatalf("BRANCH backward expected IP=1, got %d", f.IP)
	}

	// 3. 0BRANCH when flag == 0 (takes branch)
	f.CurrentBody = []Cell{7}
	f.IP = 0
	f.DSPush(0)
	f.WordZeroBranch()
	if f.IP != 7 {
		t.Fatalf("0BRANCH on 0 expected IP=7, got %d", f.IP)
	}

	// 4. 0BRANCH when flag != 0 (skips offset cell)
	f.CurrentBody = []Cell{7}
	f.IP = 0
	f.DSPush(-1)
	f.WordZeroBranch()
	if f.IP != 1 {
		t.Fatalf("0BRANCH on non-zero expected IP=1, got %d", f.IP)
	}

	// 5. Underflow test
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("0BRANCH expected underflow panic on empty stack")
		}
	}()
	f.WordZeroBranch()
}

func TestConditionalBranchingCompiler(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()

	// 1. IF ... THEN without ELSE
	f.InterpretLine(": TESTIF IF 999 THEN ;")
	f.InterpretLine("0 TESTIF")
	if len(f.DS) != 0 {
		t.Fatalf("0 TESTIF expected empty stack, got %v", f.DS)
	}
	f.InterpretLine("-1 TESTIF")
	if len(f.DS) != 1 || f.DSPop() != 999 {
		t.Fatalf("-1 TESTIF expected 999, got %v", f.DS)
	}

	// 2. IF ... ELSE ... THEN
	f.InterpretLine(": TESTELSE IF 100 ELSE 200 THEN ;")
	f.InterpretLine("1 TESTELSE")
	if len(f.DS) != 1 || f.DSPop() != 100 {
		t.Fatalf("1 TESTELSE expected 100, got %v", f.DS)
	}
	f.InterpretLine("0 TESTELSE")
	if len(f.DS) != 1 || f.DSPop() != 200 {
		t.Fatalf("0 TESTELSE expected 200, got %v", f.DS)
	}

	// 3. Nested conditionals
	f.InterpretLine(": NESTED IF IF 1 ELSE 2 THEN ELSE 3 THEN ;")
	f.InterpretLine("1 1 NESTED")
	if f.DSPop() != 1 {
		t.Fatal("NESTED 1 1 failed")
	}
	f.InterpretLine("0 1 NESTED")
	if f.DSPop() != 2 {
		t.Fatal("NESTED 0 1 failed")
	}
	f.InterpretLine("1 0 NESTED")
	if f.DSPop() != 3 {
		t.Fatal("NESTED 1 0 failed")
	}
	f.InterpretLine("0 0 NESTED")
	if f.DSPop() != 3 {
		t.Fatal("NESTED 0 0 failed")
	}

	// 4. Compilation error checks
	expectPanic := func(name string, fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatalf("%s expected panic", name)
			}
		}()
		fn()
	}

	expectPanic("IF when not compiling", func() {
		f.State = false
		f.WordIF()
	})

	expectPanic("THEN when not compiling", func() {
		f.State = false
		f.WordTHEN()
	})

	expectPanic("ELSE when not compiling", func() {
		f.State = false
		f.WordELSE()
	})

	expectPanic("THEN invalid target", func() {
		f.State = true
		f.DSPush(9999)
		f.WordTHEN()
	})
	f.State = false

	expectPanic("ELSE invalid target", func() {
		f.State = true
		f.DSPush(9999)
		f.WordELSE()
	})
	f.State = false
}

func TestIndefiniteLoopConstructs(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()

	// 1. BEGIN ... UNTIL
	f.InterpretLine(": COUNTDOWN BEGIN 1 - DUP 0 = UNTIL ;")
	f.InterpretLine("5 COUNTDOWN")
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatalf("COUNTDOWN expected 0 on stack, got %v", f.DS)
	}

	// 2. BEGIN ... WHILE ... REPEAT
	f.InterpretLine(": SUMDOWN 0 SWAP BEGIN DUP 0 > WHILE SWAP OVER + SWAP 1 - REPEAT DROP ;")
	f.InterpretLine("5 SUMDOWN")
	if len(f.DS) != 1 || f.DSPop() != 15 {
		t.Fatalf("5 SUMDOWN expected 15, got %v", f.DS)
	}

	// 3. BEGIN ... AGAIN with early EXIT
	f.InterpretLine(": TESTAGAIN BEGIN 1 - DUP 0 = IF EXIT THEN AGAIN ;")
	f.InterpretLine("3 TESTAGAIN")
	if len(f.DS) != 1 || f.DSPop() != 0 {
		t.Fatalf("3 TESTAGAIN expected 0, got %v", f.DS)
	}

	// 4. Compilation error checks
	expectPanic := func(name string, fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatalf("%s expected panic", name)
			}
		}()
		fn()
	}

	expectPanic("BEGIN when not compiling", func() {
		f.State = false
		f.WordBEGIN()
	})

	expectPanic("UNTIL when not compiling", func() {
		f.State = false
		f.WordUNTIL()
	})

	expectPanic("AGAIN when not compiling", func() {
		f.State = false
		f.WordAGAIN()
	})

	expectPanic("WHILE when not compiling", func() {
		f.State = false
		f.WordWHILE()
	})

	expectPanic("REPEAT when not compiling", func() {
		f.State = false
		f.WordREPEAT()
	})

	expectPanic("REPEAT invalid target", func() {
		f.State = true
		f.DSPush(0)
		f.DSPush(9999)
		f.WordREPEAT()
	})
	f.State = false
}

func TestCountedLoopRuntimeAndCompiler(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()

	// 1. DO ... LOOP: sum 0 to 4
	f.InterpretLine(": SUM5 0 5 0 DO I + LOOP ;")
	f.InterpretLine("SUM5")
	if len(f.DS) != 1 || f.DSPop() != 10 {
		t.Fatalf("SUM5 expected 10, got %v", f.DS)
	}

	// 2. DO ... +LOOP: step by 2
	f.InterpretLine(": SUMEVEN 0 10 0 DO I + 2 +LOOP ;")
	f.InterpretLine("SUMEVEN")
	if len(f.DS) != 1 || f.DSPop() != 20 {
		t.Fatalf("SUMEVEN expected 20, got %v", f.DS)
	}

	// DO ... +LOOP: negative step
	f.InterpretLine(": SUMDOWN 0 0 10 DO I + -2 +LOOP ;")
	f.InterpretLine("SUMDOWN")
	if len(f.DS) != 1 || f.DSPop() != 30 {
		t.Fatalf("SUMDOWN expected 30, got %v", f.DS)
	}

	// 3. Nested DO loops with I and J
	f.InterpretLine(": NESTEDLOOP 0 3 0 DO 2 0 DO I J + + LOOP LOOP ;")
	f.InterpretLine("NESTEDLOOP")
	if len(f.DS) != 1 || f.DSPop() != 9 {
		t.Fatalf("NESTEDLOOP expected 9, got %v", f.DS)
	}

	// 4. LEAVE
	f.InterpretLine(": TESTLEAVE 0 10 0 DO I 5 = IF LEAVE ELSE I + THEN LOOP ;")
	f.InterpretLine("TESTLEAVE")
	if len(f.DS) != 1 {
		t.Fatalf("TESTLEAVE expected 1 item on DS, got %v", f.DS)
	}
	if val := f.DSPop(); val != 10 {
		t.Fatalf("TESTLEAVE expected 10, got %d", val)
	}

	// 5. UNLOOP and early exit
	f.InterpretLine(": EARLYEXIT 10 0 DO I 5 = IF UNLOOP 99 EXIT THEN LOOP 0 ;")
	f.InterpretLine("EARLYEXIT")
	if len(f.DS) != 1 || f.DSPop() != 99 {
		t.Fatalf("EARLYEXIT expected 99, got %v", f.DS)
	}
	if len(f.RS) != 0 {
		t.Fatalf("EARLYEXIT left dirty return stack: %v", f.RS)
	}

	// 6. Compilation and runtime error checks
	expectPanic := func(name string, fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatalf("%s expected panic", name)
			}
		}()
		fn()
	}

	expectPanic("DO when not compiling", func() {
		f.State = false
		f.WordDo()
	})

	expectPanic("LOOP when not compiling", func() {
		f.State = false
		f.WordLoop()
	})

	expectPanic("+LOOP when not compiling", func() {
		f.State = false
		f.WordPlusLoop()
	})

	expectPanic("LEAVE when not compiling", func() {
		f.State = false
		f.WordLeave()
	})

	expectPanic("J not enough levels", func() {
		f.RS = f.RS[:0]
		f.RSPush(10)
		f.RSPush(0)
		f.WordJ()
	})
	f.RS = f.RS[:0]

	expectPanic("UNLOOP underflow", func() {
		f.RS = f.RS[:0]
		f.WordUnloop()
	})
	f.RS = f.RS[:0]
}

func TestNumericOutputAndStackInspector(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()

	// 1. Signed dot (.)
	f.InterpretLine("42 . -15 . 0 .")
	if out.String() != "42 -15 0 " {
		t.Fatalf("WordDot expected '42 -15 0 ', got %q", out.String())
	}
	out.Reset()

	// 2. Unsigned dot (U.)
	f.InterpretLine("-1 U. 42 U.")
	if out.String() != "65535 42 " {
		t.Fatalf("WordUDot expected '65535 42 ', got %q", out.String())
	}
	out.Reset()

	// 3. Different base (hex)
	f.InterpretLine("255 16 BASE ! . -1 U. $A BASE !")
	if out.String() != "FF FFFF " {
		t.Fatalf("Hex formatting expected 'FF FFFF ', got %q", out.String())
	}
	out.Reset()

	// 4. Stack inspector (.S) non-destructive
	f.DSPush(10)
	f.DSPush(20)
	f.DSPush(30)
	f.WordDotS()
	if out.String() != "<3> 10 20 30 " {
		t.Fatalf(".S expected '<3> 10 20 30 ', got %q", out.String())
	}
	if len(f.DS) != 3 || f.DS[0] != 10 || f.DS[1] != 20 || f.DS[2] != 30 {
		t.Fatalf(".S altered stack: %v", f.DS)
	}
	out.Reset()
	f.DS = f.DS[:0]

	// 5. PAGE
	f.WordPage()
	if out.String() != "\033[2J\033[H" {
		t.Fatalf("PAGE expected ANSI escape, got %q", out.String())
	}
	out.Reset()

	// 6. Underflow tests
	expectPanic := func(name string, fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatalf("%s expected panic", name)
			}
		}()
		fn()
	}

	expectPanic("dot on empty stack", func() {
		f.WordDot()
	})

	expectPanic("U. on empty stack", func() {
		f.WordUDot()
	})
}

func TestStringLiteralsAndOutput(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()

	// 1. ." in interpret mode
	f.InterpretLine(".\" Hello World!\"")
	if out.String() != "Hello World!" {
		t.Fatalf(".\" in interpret mode expected 'Hello World!', got %q", out.String())
	}
	out.Reset()

	// 2. ." inside colon definition
	f.InterpretLine(": GREET .\" Hi Forth!\" ;")
	f.InterpretLine("GREET")
	if out.String() != "Hi Forth!" {
		t.Fatalf(".\" in colon definition expected 'Hi Forth!', got %q", out.String())
	}
	out.Reset()

	// 3. S" in interpret mode
	f.InterpretLine("S\" ABCDE\"")
	if len(f.DS) != 2 {
		t.Fatalf("S\" in interpret mode expected 2 items on DS, got %v", f.DS)
	}
	length := int(f.DSPop())
	addr := int(f.DSPop())
	if length != 5 {
		t.Fatalf("S\" expected length 5, got %d", length)
	}
	extracted := string(f.Memory[addr : addr+length])
	if extracted != "ABCDE" {
		t.Fatalf("S\" expected 'ABCDE', got %q", extracted)
	}

	// 4. S" inside colon definition
	f.InterpretLine(": GETSTR S\" Testing 123\" ;")
	f.InterpretLine("GETSTR")
	if len(f.DS) != 2 {
		t.Fatalf("GETSTR expected 2 items on DS, got %v", f.DS)
	}
	length2 := int(f.DSPop())
	addr2 := int(f.DSPop())
	if length2 != 11 {
		t.Fatalf("GETSTR expected length 11, got %d", length2)
	}
	extracted2 := string(f.Memory[addr2 : addr2+length2])
	if extracted2 != "Testing 123" {
		t.Fatalf("GETSTR expected 'Testing 123', got %q", extracted2)
	}

	// 5. Parenthesis comments ( )
	f.InterpretLine("10 ( this is a comment ) 20 +")
	if len(f.DS) != 1 || f.DSPop() != 30 {
		t.Fatalf("( ) comment failed, DS: %v", f.DS)
	}
	f.InterpretLine(": TESTCOMM ( comment ) 5 ( another ) ;")
	f.InterpretLine("TESTCOMM")
	if len(f.DS) != 1 || f.DSPop() != 5 {
		t.Fatalf("( ) in colon failed, DS: %v", f.DS)
	}

	// 6. Backslash comment \
	f.InterpretLine("42 \\ everything here is ignored 999")
	if len(f.DS) != 1 || f.DSPop() != 42 {
		t.Fatalf("\\ comment failed, DS: %v", f.DS)
	}
}

func TestStringEvaluationAndFileLoading(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()

	// 1. EVALUATE
	f.InterpretLine("S\" 10 20 +\" EVALUATE")
	if len(f.DS) != 1 || f.DSPop() != 30 {
		t.Fatalf("EVALUATE 10 20 + failed, DS: %v", f.DS)
	}

	f.InterpretLine("S\" : TRIPLE 3 * ;\" EVALUATE")
	f.InterpretLine("7 TRIPLE")
	if len(f.DS) != 1 || f.DSPop() != 21 {
		t.Fatalf("EVALUATE : TRIPLE 3 * ; failed, DS: %v", f.DS)
	}

	// 2. INCLUDED
	tmpDir := t.TempDir()
	fsFile := filepath.Join(tmpDir, "fromfile.fs")
	if err := os.WriteFile(fsFile, []byte(": FROMFILE 100 200 + ;\n"), 0644); err != nil {
		t.Fatal(err)
	}
	f.InterpretLine("S\" " + fsFile + "\" INCLUDED")
	f.InterpretLine("FROMFILE")
	if len(f.DS) != 1 || f.DSPop() != 300 {
		t.Fatalf("FROMFILE via INCLUDED failed, DS: %v", f.DS)
	}

	// 3. INCLUDE
	fsFile2 := filepath.Join(tmpDir, "fromfile2.fs")
	if err := os.WriteFile(fsFile2, []byte(": FROMFILE2 50 25 - ;\n"), 0644); err != nil {
		t.Fatal(err)
	}
	f.InterpretLine("INCLUDE " + fsFile2)
	f.InterpretLine("FROMFILE2")
	if len(f.DS) != 1 || f.DSPop() != 25 {
		t.Fatalf("FROMFILE2 via INCLUDE failed, DS: %v", f.DS)
	}

	// 4. Error handling
	expectPanic := func(name string, fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatalf("%s expected panic", name)
			}
		}()
		fn()
	}

	expectPanic("EVALUATE underflow", func() {
		f.WordEvaluate()
	})

	expectPanic("INCLUDED underflow", func() {
		f.WordIncluded()
	})

	expectPanic("INCLUDED missing file", func() {
		f.InterpretLine("S\" /nonexistent/file.fs\" INCLUDED")
	})

	expectPanic("INCLUDE missing file", func() {
		f.InterpretLine("INCLUDE /nonexistent/file.fs")
	})

	expectPanic("INCLUDE without token", func() {
		f.SetLine("")
		f.WordInclude()
	})
}

func TestModuleDependencyResolver(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()

	// 1. #REQUIRE CORE
	f.InterpretLine("#REQUIRE CORE")
	if !f.Loaded["CORE"] {
		t.Fatal("expected CORE to be marked loaded")
	}

	// Verify words defined by core.fs
	f.InterpretLine("5 1+")
	if len(f.DS) != 1 || f.DSPop() != 6 {
		t.Fatalf("1+ from CORE failed, DS: %v", f.DS)
	}

	f.InterpretLine("10 20 NIP")
	if len(f.DS) != 1 || f.DSPop() != 20 {
		t.Fatalf("NIP from CORE failed, DS: %v", f.DS)
	}

	// 2. Deduplication: requiring CORE again is a no-op
	f.InterpretLine("#REQUIRE CORE")

	// 3. Error handling
	expectPanic := func(name string, fn func()) {
		defer func() {
			if r := recover(); r == nil {
				t.Fatalf("%s expected panic", name)
			}
		}()
		fn()
	}

	expectPanic("missing module name", func() {
		f.SetLine("")
		f.WordRequire()
	})

	expectPanic("nonexistent module", func() {
		f.InterpretLine("#REQUIRE NONEXISTENT_MODULE_NAME")
	})
}

func TestCoreLibraryBootstrap(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()
	f.RequireModule("CORE")

	// Stack words
	f.InterpretLine("1 2 NIP")
	if len(f.DS) != 1 || f.DSPop() != 2 {
		t.Fatal("NIP failed")
	}

	f.InterpretLine("1 2 TUCK")
	if len(f.DS) != 3 || f.DS[0] != 2 || f.DS[1] != 1 || f.DS[2] != 2 {
		t.Fatalf("TUCK failed, DS: %v", f.DS)
	}
	f.DS = f.DS[:0]

	f.InterpretLine("1 2 3 ROT")
	if len(f.DS) != 3 || f.DS[0] != 2 || f.DS[1] != 3 || f.DS[2] != 1 {
		t.Fatalf("ROT failed, DS: %v", f.DS)
	}
	f.DS = f.DS[:0]

	f.InterpretLine("1 2 3 -ROT")
	if len(f.DS) != 3 || f.DS[0] != 3 || f.DS[1] != 1 || f.DS[2] != 2 {
		t.Fatalf("-ROT failed, DS: %v", f.DS)
	}
	f.DS = f.DS[:0]

	f.InterpretLine("1 2 2DUP")
	if len(f.DS) != 4 || f.DS[0] != 1 || f.DS[1] != 2 || f.DS[2] != 1 || f.DS[3] != 2 {
		t.Fatalf("2DUP failed, DS: %v", f.DS)
	}
	f.DS = f.DS[:0]

	f.InterpretLine("1 2 2DROP")
	if len(f.DS) != 0 {
		t.Fatalf("2DROP failed, DS: %v", f.DS)
	}

	// Division & Modulo
	f.InterpretLine("10 2 /")
	if len(f.DS) != 1 || f.DSPop() != 5 {
		t.Fatal("/ failed")
	}

	f.InterpretLine("10 3 MOD")
	if len(f.DS) != 1 || f.DSPop() != 1 {
		t.Fatal("MOD failed")
	}

	// Comparisons
	f.InterpretLine("0 0= 5 0=")
	if len(f.DS) != 2 || f.DS[0] != -1 || f.DS[1] != 0 {
		t.Fatalf("0= failed, DS: %v", f.DS)
	}
	f.DS = f.DS[:0]

	f.InterpretLine("5 5 <> 5 6 <>")
	if len(f.DS) != 2 || f.DS[0] != 0 || f.DS[1] != -1 {
		t.Fatalf("<> failed, DS: %v", f.DS)
	}
	f.DS = f.DS[:0]

	f.InterpretLine("0 0<> 5 0<>")
	if len(f.DS) != 2 || f.DS[0] != 0 || f.DS[1] != -1 {
		t.Fatalf("0<> failed, DS: %v", f.DS)
	}
	f.DS = f.DS[:0]

	f.InterpretLine("-1 0< 0 0< 1 0<")
	if len(f.DS) != 3 || f.DS[0] != -1 || f.DS[1] != 0 || f.DS[2] != 0 {
		t.Fatalf("0< failed, DS: %v", f.DS)
	}
	f.DS = f.DS[:0]

	f.InterpretLine("1 0> 0 0> -1 0>")
	if len(f.DS) != 3 || f.DS[0] != -1 || f.DS[1] != 0 || f.DS[2] != 0 {
		t.Fatalf("0> failed, DS: %v", f.DS)
	}
	f.DS = f.DS[:0]

	f.InterpretLine("4 5 <= 5 5 <= 6 5 <=")
	if len(f.DS) != 3 || f.DS[0] != -1 || f.DS[1] != -1 || f.DS[2] != 0 {
		t.Fatalf("<= failed, DS: %v", f.DS)
	}
	f.DS = f.DS[:0]

	f.InterpretLine("6 5 >= 5 5 >= 4 5 >=")
	if len(f.DS) != 3 || f.DS[0] != -1 || f.DS[1] != -1 || f.DS[2] != 0 {
		t.Fatalf(">= failed, DS: %v", f.DS)
	}
	f.DS = f.DS[:0]

	// Arithmetic
	f.InterpretLine("5 1+ 5 1- 5 2+ 5 2- 5 2* 10 2/ 5 NEGATE")
	expectedArith := []Cell{6, 4, 7, 3, 10, 5, -5}
	if len(f.DS) != len(expectedArith) {
		t.Fatalf("Arithmetic count mismatch: %v", f.DS)
	}
	for i, exp := range expectedArith {
		if f.DS[i] != exp {
			t.Fatalf("Arithmetic [%d] expected %d, got %d", i, exp, f.DS[i])
		}
	}
	f.DS = f.DS[:0]

	// +!
	f.InterpretLine("VARIABLE TESTVAR 10 TESTVAR ! 5 TESTVAR +! TESTVAR @")
	if len(f.DS) != 1 || f.DSPop() != 15 {
		t.Fatal("+! failed")
	}

	// ABS, MIN, MAX
	f.InterpretLine("-5 ABS 5 ABS 3 7 MIN 7 3 MIN 3 7 MAX 7 3 MAX")
	expectedExt := []Cell{5, 5, 3, 3, 7, 7}
	if len(f.DS) != len(expectedExt) {
		t.Fatalf("ABS/MIN/MAX count mismatch: %v", f.DS)
	}
	for i, exp := range expectedExt {
		if f.DS[i] != exp {
			t.Fatalf("ABS/MIN/MAX [%d] expected %d, got %d", i, exp, f.DS[i])
		}
	}
	f.DS = f.DS[:0]

	// I/O: SPACE, SPACES, ?, PAGE
	out.Reset()
	f.InterpretLine("SPACE")
	if out.String() != " " {
		t.Fatalf("SPACE expected ' ', got %q", out.String())
	}
	out.Reset()

	f.InterpretLine("3 SPACES")
	if out.String() != "   " {
		t.Fatalf("SPACES expected '   ', got %q", out.String())
	}
	out.Reset()

	f.InterpretLine("TESTVAR ?")
	if out.String() != "15 " {
		t.Fatalf("? expected '15 ', got %q", out.String())
	}
	out.Reset()

	f.InterpretLine("PAGE")
	if out.String() != "\014" {
		t.Fatalf("PAGE expected FormFeed (12), got %q", out.String())
	}
	out.Reset()

	// Misc: ERASE
	f.Store(2000, 0x1234)
	f.InterpretLine("2000 2 ERASE")
	if f.Fetch(2000) != 0 {
		t.Fatal("ERASE failed")
	}
}

func TestDemoHelloWorld(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()
	f.RequireModule("CORE")

	helloSrc := ": HELLO\n  .\" Hello, world!\" CR\n;\nHELLO\n"
	f.LoadSystem(helloSrc)
	if out.String() != "Hello, world!\n" {
		t.Fatalf("expected 'Hello, world!\\n', got %q", out.String())
	}
}

func TestDemoFactorial(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()
	f.RequireModule("CORE")

	factorialSrc := ": FACTORIAL ( n -- n! )\n  DUP 1 >\n  IF\n    DUP 1 - RECURSE *\n  THEN\n;\n5 FACTORIAL . CR\n7 FACTORIAL . CR\n"
	f.LoadSystem(factorialSrc)
	if out.String() != "120 \n5040 \n" {
		t.Fatalf("expected '120 \\n5040 \\n', got %q", out.String())
	}
}

func TestDemoFibonacci(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()
	f.RequireModule("CORE")

	fibSrc := ": FIB ( n -- fib[n] )\n  0 1 ROT 0 DO\n    TUCK +\n  LOOP DROP\n;\n10 FIB . CR\n"
	f.LoadSystem(fibSrc)
	if out.String() != "55 \n" {
		t.Fatalf("expected '55 \\n', got %q", out.String())
	}
}

func TestDemoBubbleSort(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()
	f.RequireModule("CORE")

	bubbleSrc := `CREATE DATA 9 C, 8 C, 7 C, 6 C, 5 C, 4 C, 3 C, 2 C, 1 C, 0 C,
: PRINT
  10 0 DO DATA I + C@ . LOOP CR
;
: BUBBLE ( addr len -- )
  DUP 1 DO
    DUP I - 0 DO
      OVER I + DUP 1+
      OVER C@ OVER C@
      2DUP > IF
        SWAP ROT C! SWAP C!
      ELSE
        2DROP 2DROP
      THEN
    LOOP
  LOOP
  2DROP
;
DATA 10 BUBBLE
PRINT
`
	f.LoadSystem(bubbleSrc)
	if out.String() != "0 1 2 3 4 5 6 7 8 9 \n" {
		t.Fatalf("expected '0 1 2 3 4 5 6 7 8 9 \\n', got %q", out.String())
	}
}

func TestDemoSieve(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()
	f.RequireModule("CORE")

	sieveSrc := `8190 CONSTANT SIZE
VARIABLE FLAGS SIZE ALLOT
: SETUP-FLAGS
  SIZE 0 DO
    1 FLAGS I + C!
  LOOP ;
: SIEVE
  SETUP-FLAGS
  0
  SIZE 0 DO
    FLAGS I + C@ IF
      I I + 3 +
      DUP I +
      BEGIN
        DUP SIZE <
      WHILE
        DUP FLAGS + 0 SWAP C!
        OVER +
      REPEAT
      DROP DROP 1 +
    THEN
  LOOP
  . CR
;
SIEVE
`
	f.LoadSystem(sieveSrc)
	if out.String() != "1899 \n" {
		t.Fatalf("expected '1899 \\n', got %q", out.String())
	}
}

func TestDemoTowerOfHanoi(t *testing.T) {
	out := &bytes.Buffer{}
	f := New(&bytes.Buffer{}, out)
	f.InitPrimitives()
	f.RequireModule("CORE")

	hanoiSrc := `VARIABLE SRC
VARIABLE DEST
VARIABLE AUX
: HANOI ( n src dest aux -- )
  AUX ! DEST ! SRC !
  DUP IF
    SRC @ DEST @ AUX @
    >R >R >R
    DUP 1- SRC @ AUX @ DEST @ RECURSE
    R> SRC ! R> DEST ! R> AUX !
    ." Move " SRC @ . ." -> " DEST @ . CR
    1- AUX @ DEST @ SRC @ RECURSE
  ELSE
    DROP
  THEN
;
3 1 3 2 HANOI
`
	f.LoadSystem(hanoiSrc)
	expected := "Move 1 -> 3 \n" +
		"Move 1 -> 2 \n" +
		"Move 3 -> 2 \n" +
		"Move 1 -> 3 \n" +
		"Move 2 -> 1 \n" +
		"Move 2 -> 3 \n" +
		"Move 1 -> 3 \n"
	if out.String() != expected {
		t.Fatalf("expected %q, got %q", expected, out.String())
	}
}
