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
