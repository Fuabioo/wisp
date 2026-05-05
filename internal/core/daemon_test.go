package core

import (
	"testing"
)

// ---------------------------------------------------------------------------
// acquireClientID / releaseClientID
// ---------------------------------------------------------------------------

func TestAcquireRelease_Basic(t *testing.T) {
	cid, release := acquireClientID("testuser")
	if cid != "testuser-1" {
		t.Fatalf("got %q, want testuser-1", cid)
	}
	release()
	cid2, release2 := acquireClientID("testuser")
	if cid2 != "testuser-1" {
		t.Fatalf("got %q, want testuser-1 (reuse after release)", cid2)
	}
	release2()
}

func TestAcquireRelease_MultipleUsers(t *testing.T) {
	cidA, relA := acquireClientID("alice")
	cidB, _ := acquireClientID("bob")
	cidA2, relA2 := acquireClientID("alice")

	if cidA != "alice-1" {
		t.Fatalf("cidA = %q, want alice-1", cidA)
	}
	if cidB != "bob-1" {
		t.Fatalf("cidB = %q, want bob-1", cidB)
	}
	if cidA2 != "alice-2" {
		t.Fatalf("cidA2 = %q, want alice-2", cidA2)
	}
	relA()
	relA2()
}

func TestAcquireRelease_SuffixReuse(t *testing.T) {
	_, r1 := acquireClientID("x")
	cid2, r2 := acquireClientID("x")
	cid3, r3 := acquireClientID("x")
	if cid2 != "x-2" {
		t.Fatalf("cid2 = %q, want x-2", cid2)
	}
	if cid3 != "x-3" {
		t.Fatalf("cid3 = %q, want x-3", cid3)
	}
	r1()
	r2()
	r3()
	cid4, r4 := acquireClientID("x")
	if cid4 != "x-1" {
		t.Fatalf("cid4 = %q, want x-1", cid4)
	}
	r4()
}

func TestAcquireRelease_GapFill(t *testing.T) {
	_, r1 := acquireClientID("y")
	_, r2 := acquireClientID("y")
	_, r3 := acquireClientID("y")
	r2()
	cid4, r4 := acquireClientID("y")
	if cid4 != "y-2" {
		t.Fatalf("cid4 = %q, want y-2 (fill gap)", cid4)
	}
	r1()
	r3()
	r4()
}

func TestReleaseClientID_UnknownUser(t *testing.T) {
	releaseClientID("nonexistent", 1)
}

func TestReleaseClientID_UnknownSuffix(t *testing.T) {
	cid, release := acquireClientID("z")
	if cid != "z-1" {
		t.Fatalf("got %q, want z-1", cid)
	}
	release()
	releaseClientID("z", 99)
	cid2, release2 := acquireClientID("z")
	if cid2 != "z-1" {
		t.Fatalf("got %q, want z-1", cid2)
	}
	release2()
}

// ---------------------------------------------------------------------------
// Daemon RPC handlers (no SSH/PTY dependencies)
// ---------------------------------------------------------------------------

func TestDaemon_NewDaemon(t *testing.T) {
	d := NewDaemon()
	if d == nil || d.servers == nil {
		t.Fatal("NewDaemon returned nil or nil map")
	}
}

func TestDaemon_ListServers_Empty(t *testing.T) {
	d := NewDaemon()
	var infos []ServerInfo
	err := d.ListServers(new(int), &infos)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(infos) != 0 {
		t.Fatalf("expected 0 servers, got %d", len(infos))
	}
}

func TestDaemon_ListServers_SortedByPort(t *testing.T) {
	d := NewDaemon()
	d.servers = map[string]*ServerSession{
		"bbbbbbbb": {Info: ServerInfo{ID: "bbbbbbbb", Port: 2224, Status: "Active"}},
		"aaaaaaaa": {Info: ServerInfo{ID: "aaaaaaaa", Port: 2222, Status: "Active"}},
		"cccccccc": {Info: ServerInfo{ID: "cccccccc", Port: 2223, Status: "Down"}},
	}
	var infos []ServerInfo
	err := d.ListServers(new(int), &infos)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(infos) != 3 {
		t.Fatalf("expected 3 servers, got %d", len(infos))
	}
	if infos[0].Port != 2222 || infos[1].Port != 2223 || infos[2].Port != 2224 {
		t.Fatalf("servers not sorted by port: %v", infos)
	}
}

func TestDaemon_ListServers_SamePort(t *testing.T) {
	d := NewDaemon()
	d.servers = map[string]*ServerSession{
		"bbbbbbbb": {Info: ServerInfo{ID: "bbbbbbbb", Port: 2222, Status: "Active"}},
		"aaaaaaaa": {Info: ServerInfo{ID: "aaaaaaaa", Port: 2222, Status: "Down"}},
	}
	var infos []ServerInfo
	err := d.ListServers(new(int), &infos)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(infos) != 2 {
		t.Fatalf("expected 2 servers, got %d", len(infos))
	}
	if infos[0].ID != "aaaaaaaa" || infos[1].ID != "bbbbbbbb" {
		t.Fatalf("same-port servers not sorted by ID: %v", infos)
	}
}

func TestDaemon_KillServer_NotFound(t *testing.T) {
	d := NewDaemon()
	req := "nonexist"
	err := d.KillServer(&req, new(bool))
	if err == nil {
		t.Fatal("expected error for nonexistent session")
	}
}

func TestDaemon_DownServer_NotFound(t *testing.T) {
	d := NewDaemon()
	req := "nonexist"
	err := d.DownServer(&req, new(bool))
	if err == nil {
		t.Fatal("expected error for nonexistent session")
	}
}

func TestDaemon_UpServer_NotFound(t *testing.T) {
	d := NewDaemon()
	req := "nonexist"
	err := d.UpServer(&req, new(bool))
	if err == nil {
		t.Fatal("expected error for nonexistent session")
	}
}

func TestDaemon_DownServer_AlreadyDown(t *testing.T) {
	d := NewDaemon()
	d.servers = map[string]*ServerSession{
		"test1234": {Info: ServerInfo{ID: "test1234", Port: 2222, Status: "Down"}},
	}
	req := "test1234"
	var res bool
	err := d.DownServer(&req, &res)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !res {
		t.Fatal("expected res=true for already-down session")
	}
}

func TestDaemon_UpServer_AlreadyActive(t *testing.T) {
	d := NewDaemon()
	d.servers = map[string]*ServerSession{
		"test1234": {Info: ServerInfo{ID: "test1234", Port: 2222, Status: "Active"}},
	}
	req := "test1234"
	var res bool
	err := d.UpServer(&req, &res)
	if err == nil {
		t.Fatal("expected error for already-active session")
	}
}

func TestDaemon_ListPeers_NotFound(t *testing.T) {
	d := NewDaemon()
	req := PeersReq{SessionID: "nonexist"}
	var peers []PeerInfo
	err := d.ListPeers(&req, &peers)
	if err == nil {
		t.Fatal("expected error for nonexistent session")
	}
}

func TestDaemon_KickPeer_NotFound(t *testing.T) {
	d := NewDaemon()
	req := KickReq{SessionID: "nonexist", ClientID: "bob-1"}
	var res bool
	err := d.KickPeer(&req, &res)
	if err == nil {
		t.Fatal("expected error for nonexistent session")
	}
}

func TestDaemon_GetTail_NotFound(t *testing.T) {
	d := NewDaemon()
	req := TailReq{SessionID: "nonexist"}
	var res string
	err := d.GetTail(&req, &res)
	if err == nil {
		t.Fatal("expected error for nonexistent session")
	}
}

func TestDaemon_RefreshSession_NotFound(t *testing.T) {
	d := NewDaemon()
	req := RefreshReq{SessionID: "nonexist"}
	err := d.RefreshSession(&req, new(bool))
	if err == nil {
		t.Fatal("expected error for nonexistent session")
	}
}

func TestDaemon_StartServer_PortConflict(t *testing.T) {
	d := NewDaemon()
	d.servers = map[string]*ServerSession{
		"test1234": {Info: ServerInfo{ID: "test1234", Port: 2222, Status: "Active"}},
	}
	req := StartServerReq{Port: 2222}
	err := d.StartServer(&req, new(ServerInfo))
	if err == nil {
		t.Fatal("expected error for port conflict")
	}
}
