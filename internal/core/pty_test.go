package core

import (
	"io"
	"sync"
	"testing"

	"github.com/charmbracelet/ssh"
)

// ---------------------------------------------------------------------------
// tailBuffer
// ---------------------------------------------------------------------------

func TestTailBuffer_WriteWithinCapacity(t *testing.T) {
	tb := newTailBuffer(20)
	tb.write([]byte("hello"))
	if got := tb.snapshot(); got != "hello" {
		t.Fatalf("snapshot = %q, want %q", got, "hello")
	}
	tb.write([]byte(" world"))
	if got := tb.snapshot(); got != "hello world" {
		t.Fatalf("snapshot = %q, want %q", got, "hello world")
	}
}

func TestTailBuffer_WriteExceedsCapacity(t *testing.T) {
	tb := newTailBuffer(5)
	tb.write([]byte("abcdefg"))
	if got := tb.snapshot(); got != "cdefg" {
		t.Fatalf("snapshot = %q, want %q (oldest bytes dropped)", got, "cdefg")
	}
	tb.write([]byte("hij"))
	if got := tb.snapshot(); got != "fghij" {
		t.Fatalf("snapshot = %q, want %q", got, "fghij")
	}
}

func TestTailBuffer_Empty(t *testing.T) {
	tb := newTailBuffer(100)
	if got := tb.snapshot(); got != "" {
		t.Fatalf("snapshot = %q, want empty", got)
	}
}

func TestTailBuffer_Concurrent(t *testing.T) {
	tb := newTailBuffer(64 * 1024)
	var wg sync.WaitGroup
	for i := 0; i < 10; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for j := 0; j < 100; j++ {
				tb.write([]byte("x"))
			}
		}()
	}
	wg.Wait()
	snap := tb.snapshot()
	if len(snap) == 0 {
		t.Fatal("concurrent writes produced empty buffer")
	}
}

// ---------------------------------------------------------------------------
// chanReader
// ---------------------------------------------------------------------------

func TestChanReader_ReadAll(t *testing.T) {
	ch := make(chan byte, 10)
	ch <- 'a'
	ch <- 'b'
	ch <- 'c'
	close(ch)

	r := chanReader{ch: ch}
	out := make([]byte, 10)
	n, err := r.Read(out)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if n != 3 {
		t.Fatalf("n = %d, want 3", n)
	}
	if string(out[:n]) != "abc" {
		t.Fatalf("got %q, want %q", string(out[:n]), "abc")
	}
}

func TestChanReader_ReadSmallBuffer(t *testing.T) {
	ch := make(chan byte, 5)
	ch <- 'a'
	ch <- 'b'
	ch <- 'c'

	r := chanReader{ch: ch}
	out := make([]byte, 2)
	n1, err := r.Read(out)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if n1 != 2 {
		t.Fatalf("n1 = %d, want 2", n1)
	}
	n2, err := r.Read(out)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if n2 != 1 {
		t.Fatalf("n2 = %d, want 1", n2)
	}
}

func TestChanReader_ReadClosedChannel(t *testing.T) {
	ch := make(chan byte)
	close(ch)

	r := chanReader{ch: ch}
	out := make([]byte, 10)
	n, err := r.Read(out)
	if err != io.EOF {
		t.Fatalf("err = %v, want io.EOF", err)
	}
	if n != 0 {
		t.Fatalf("n = %d, want 0", n)
	}
}

func TestChanReader_ReadZeroLengthBuffer(t *testing.T) {
	ch := make(chan byte, 1)
	ch <- 'x'
	r := chanReader{ch: ch}
	out := make([]byte, 0)
	n, err := r.Read(out)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if n != 0 {
		t.Fatalf("n = %d, want 0", n)
	}
}

// ---------------------------------------------------------------------------
// digraphMachine - extracted from HandleSession
// ---------------------------------------------------------------------------

type digraphState int

const (
	digraphNormal digraphState = iota
	digraphPendingBang
)

// digraphMachine implements the !> pause-menu trigger state machine.
type digraphMachine struct {
	state digraphState
}

// feed processes a single byte. Returns:
//
//	forward  - bytes that should be forwarded to the PTY
//	menu     - true if the !> digraph was completed (menu should open)
func (dm *digraphMachine) feed(c byte) (forward []byte, menu bool) {
	if dm.state == digraphPendingBang {
		dm.state = digraphNormal
		if c == '>' {
			return nil, true
		}
		// Not '>': replay the pending '!'
		forward = append(forward, '!')
	}
	if c == '!' {
		dm.state = digraphPendingBang
		return forward, false
	}
	forward = append(forward, c)
	return forward, false
}

// timeout handles the 250ms timeout expiring while a '!' is pending.
func (dm *digraphMachine) timeout() (forward []byte) {
	if dm.state == digraphPendingBang {
		dm.state = digraphNormal
		forward = append(forward, '!')
	}
	return forward
}

// flush returns any pending '!' not yet forwarded (channel closed mid-pending).
func (dm *digraphMachine) flush() (forward []byte) {
	if dm.state == digraphPendingBang {
		dm.state = digraphNormal
		forward = append(forward, '!')
	}
	return forward
}

func TestDigraphMachine_NormalBytes(t *testing.T) {
	dm := &digraphMachine{}
	forward, menu := dm.feed('a')
	if menu {
		t.Fatal("unexpected menu trigger")
	}
	if len(forward) != 1 || forward[0] != 'a' {
		t.Fatalf("forward = %v, want ['a']", forward)
	}
}

func TestDigraphMachine_BangThenOther(t *testing.T) {
	dm := &digraphMachine{}
	forward, menu := dm.feed('!')
	if menu || len(forward) != 0 {
		t.Fatalf("! should be buffered, got forward=%v menu=%v", forward, menu)
	}
	forward, menu = dm.feed('x')
	if menu {
		t.Fatal("unexpected menu trigger for !x")
	}
	if len(forward) != 2 || forward[0] != '!' || forward[1] != 'x' {
		t.Fatalf("forward = %v, want ['!','x']", forward)
	}
}

func TestDigraphMachine_BangThenGreater(t *testing.T) {
	dm := &digraphMachine{}
	forward, menu := dm.feed('!')
	if menu || len(forward) != 0 {
		t.Fatalf("! should be buffered, got forward=%v menu=%v", forward, menu)
	}
	forward, menu = dm.feed('>')
	if !menu {
		t.Fatal("!> should trigger menu")
	}
	if len(forward) != 0 {
		t.Fatalf("forward = %v, want empty", forward)
	}
}

func TestDigraphMachine_BangTimeout(t *testing.T) {
	dm := &digraphMachine{}
	dm.feed('!')
	forward := dm.timeout()
	if len(forward) != 1 || forward[0] != '!' {
		t.Fatalf("timeout forward = %v, want ['!']", forward)
	}
}

func TestDigraphMachine_DoubleBang(t *testing.T) {
	dm := &digraphMachine{}
	dm.feed('!')
	forward, menu := dm.feed('!')
	if menu {
		t.Fatal("!! should not trigger menu")
	}
	if len(forward) != 1 || forward[0] != '!' {
		t.Fatalf("forward = %v, want ['!'] (first ! replayed, second ! pending)", forward)
	}
}

func TestDigraphMachine_FlushPending(t *testing.T) {
	dm := &digraphMachine{}
	dm.feed('!')
	forward := dm.flush()
	if len(forward) != 1 || forward[0] != '!' {
		t.Fatalf("flush forward = %v, want ['!']", forward)
	}
}

func TestDigraphMachine_FlushNoPending(t *testing.T) {
	dm := &digraphMachine{}
	forward := dm.flush()
	if len(forward) != 0 {
		t.Fatalf("flush with no pending = %v, want empty", forward)
	}
}

func TestDigraphMachine_TimeoutNoPending(t *testing.T) {
	dm := &digraphMachine{}
	forward := dm.timeout()
	if len(forward) != 0 {
		t.Fatalf("timeout with no pending = %v, want empty", forward)
	}
}

func TestDigraphMachine_BangReplayThenNormal(t *testing.T) {
	dm := &digraphMachine{}
	dm.feed('!')
	forward, _ := dm.feed('!')
	if string(forward) != "!" {
		t.Fatalf("forward = %q, want %q", string(forward), "!")
	}
	forward, menu := dm.feed('a')
	if menu {
		t.Fatal("unexpected menu after !! replayed")
	}
	if string(forward) != "!a" {
		t.Fatalf("forward = %q, want %q (replayed pending ! before a)", string(forward), "!a")
	}
}

// ---------------------------------------------------------------------------
// computeMinSize - extracted from updateSizeLocked
// ---------------------------------------------------------------------------

func computeMinSize(windows []ssh.Window) (rows, cols uint16, ok bool) {
	if len(windows) == 0 {
		return 0, 0, false
	}
	var minRows, minCols uint16
	first := true
	for _, w := range windows {
		if w.Width == 0 || w.Height == 0 {
			continue
		}
		if first {
			minRows = uint16(w.Height)
			minCols = uint16(w.Width)
			first = false
		} else {
			if uint16(w.Height) < minRows {
				minRows = uint16(w.Height)
			}
			if uint16(w.Width) < minCols {
				minCols = uint16(w.Width)
			}
		}
	}
	if first {
		return 0, 0, false
	}
	return minRows, minCols, true
}

func TestComputeMinSize_SingleClient(t *testing.T) {
	rows, cols, ok := computeMinSize([]ssh.Window{
		{Width: 80, Height: 24},
	})
	if !ok {
		t.Fatal("expected ok=true")
	}
	if rows != 24 || cols != 80 {
		t.Fatalf("rows=%d cols=%d, want 24,80", rows, cols)
	}
}

func TestComputeMinSize_TwoClientsSameSize(t *testing.T) {
	rows, cols, ok := computeMinSize([]ssh.Window{
		{Width: 80, Height: 24},
		{Width: 80, Height: 24},
	})
	if !ok {
		t.Fatal("expected ok=true")
	}
	if rows != 24 || cols != 80 {
		t.Fatalf("rows=%d cols=%d, want 24,80", rows, cols)
	}
}

func TestComputeMinSize_PicksMinimum(t *testing.T) {
	rows, cols, ok := computeMinSize([]ssh.Window{
		{Width: 120, Height: 40},
		{Width: 80, Height: 24},
		{Width: 100, Height: 30},
	})
	if !ok {
		t.Fatal("expected ok=true")
	}
	if rows != 24 || cols != 80 {
		t.Fatalf("rows=%d cols=%d, want 24,80", rows, cols)
	}
}

func TestComputeMinSize_SkipsZeroDimensions(t *testing.T) {
	rows, cols, ok := computeMinSize([]ssh.Window{
		{Width: 0, Height: 0},
		{Width: 80, Height: 24},
		{Width: 0, Height: 30},
	})
	if !ok {
		t.Fatal("expected ok=true")
	}
	if rows != 24 || cols != 80 {
		t.Fatalf("rows=%d cols=%d, want 24,80", rows, cols)
	}
}

func TestComputeMinSize_AllZero(t *testing.T) {
	_, _, ok := computeMinSize([]ssh.Window{
		{Width: 0, Height: 0},
		{Width: 0, Height: 0},
	})
	if ok {
		t.Fatal("all-zero windows should return ok=false")
	}
}

func TestComputeMinSize_Empty(t *testing.T) {
	_, _, ok := computeMinSize(nil)
	if ok {
		t.Fatal("empty windows should return ok=false")
	}
	_, _, ok = computeMinSize([]ssh.Window{})
	if ok {
		t.Fatal("empty windows should return ok=false")
	}
}

func TestComputeMinSize_MinRowsFromOneMinColsFromOther(t *testing.T) {
	rows, cols, ok := computeMinSize([]ssh.Window{
		{Width: 120, Height: 40},
		{Width: 80, Height: 50},
	})
	if !ok {
		t.Fatal("expected ok=true")
	}
	if rows != 40 || cols != 80 {
		t.Fatalf("rows=%d cols=%d, want 40,80", rows, cols)
	}
}
