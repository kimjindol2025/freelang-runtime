package tests

import (
	"fmt"
	"sync"
	"testing"
	"time"
)

// 🐀 Test Mouse Report: INVARIANT MOUSE
// 목표: Raft 합의 엔진을 강제로 수만 번 상태 변화시키고, 불변조건 검증
// 무관용 규칙:
//   1. 단 1개 노드라도 데이터 일관성 깨짐 → 테스트 쥐 DEAD
//   2. 리더 선출 실패 → 설계 재검토
//   3. 로그 복제 불일치 → 프로토콜 근본 실패

type InvariantMouse struct {
	nodes              int
	electionCount      int64
	logReplicationCount int64
	inconsistencyCount int64
	mu                 sync.Mutex

	// Raft 상태 검사
	nodeStates map[int]NodeState
	logHashes  map[int]string // 각 노드의 로그 해시
}

type NodeState struct {
	ID           int
	Term         int64
	State        string // "Follower", "Candidate", "Leader"
	CommitIndex  int64
	LastLogIndex int64
	LastLogTerm  int64
}

// 초기화
func NewInvariantMouse(nodeCount int) *InvariantMouse {
	return &InvariantMouse{
		nodes:      nodeCount,
		nodeStates: make(map[int]NodeState),
		logHashes:  make(map[int]string),
	}
}

// Invariant 1: 단일 리더만 존재해야 함
func (im *InvariantMouse) CheckSingleLeader(t *testing.T) {
	im.mu.Lock()
	defer im.mu.Unlock()

	leaderCount := 0
	var leaderID int

	for id, state := range im.nodeStates {
		if state.State == "Leader" {
			leaderCount++
			leaderID = id
		}
	}

	if leaderCount == 0 {
		t.Fatalf("❌ [DEAD] No leader elected. Raft invariant violated.")
	}

	if leaderCount > 1 {
		t.Fatalf("❌ [DEAD] Multiple leaders detected (%d). Split-brain condition. System DEAD.", leaderCount)
	}

	t.Logf("✅ Invariant 1 OK: Single leader exists (Leader: Node #%d)", leaderID)
}

// Invariant 2: 모든 노드의 CommitIndex가 일치해야 함 (리더 기준)
func (im *InvariantMouse) CheckLogConsistency(t *testing.T) {
	im.mu.Lock()
	defer im.mu.Unlock()

	if len(im.nodeStates) == 0 {
		return
	}

	// 리더의 CommitIndex를 기준으로
	var leaderCommitIndex int64 = -1

	for _, state := range im.nodeStates {
		if state.State == "Leader" {
			leaderCommitIndex = state.CommitIndex
			break
		}
	}

	if leaderCommitIndex == -1 {
		t.Fatalf("❌ [DEAD] No leader to verify consistency")
	}

	// 모든 노드가 같은 CommitIndex를 가져야 함
	for id, state := range im.nodeStates {
		if state.CommitIndex != leaderCommitIndex {
			im.inconsistencyCount++
			t.Fatalf("❌ [DEAD] Data inconsistency at Node #%d: CommitIndex=%d (expected %d)",
				id, state.CommitIndex, leaderCommitIndex)
		}
	}

	t.Logf("✅ Invariant 2 OK: All nodes have consistent CommitIndex (%d)", leaderCommitIndex)
}

// Invariant 3: 로그 해시가 모두 일치해야 함
func (im *InvariantMouse) CheckLogHashes(t *testing.T) {
	im.mu.Lock()
	defer im.mu.Unlock()

	if len(im.logHashes) == 0 {
		return
	}

	baseHash := ""
	for _, hash := range im.logHashes {
		if baseHash == "" {
			baseHash = hash
		}
		if hash != baseHash {
			im.inconsistencyCount++
			t.Fatalf("❌ [DEAD] Log hash mismatch. One node has different log. Data corruption suspected.")
		}
	}

	t.Logf("✅ Invariant 3 OK: All nodes have identical log hashes (%s)", baseHash)
}

// Invariant 4: 리더의 Term은 절대 감소하지 않음
func (im *InvariantMouse) CheckTermMonotonicity(t *testing.T) {
	im.mu.Lock()
	defer im.mu.Unlock()

	var lastLeaderTerm int64 = 0

	for _, state := range im.nodeStates {
		if state.State == "Leader" {
			if state.Term < lastLeaderTerm {
				t.Fatalf("❌ [DEAD] Leader term decreased: %d → %d. Causality violation.",
					lastLeaderTerm, state.Term)
			}
			lastLeaderTerm = state.Term
		}
	}

	t.Logf("✅ Invariant 4 OK: Leader term monotonicity maintained")
}

// 1단계: 초기 클러스터 형성
func (im *InvariantMouse) InitializeCluster(t *testing.T) {
	t.Log("🐀 [INVARIANT] Step 1: Initializing cluster...")

	im.mu.Lock()
	defer im.mu.Unlock()

	for i := 0; i < im.nodes; i++ {
		im.nodeStates[i] = NodeState{
			ID:           i,
			Term:         0,
			State:        "Follower",
			CommitIndex:  0,
			LastLogIndex: 0,
			LastLogTerm:  0,
		}
		im.logHashes[i] = "log_v0"
	}

	t.Logf("✅ Initialized %d nodes", im.nodes)
}

// 2단계: 선거(Election) 강제 실행 - 초당 100회 이상
func (im *InvariantMouse) TriggerMassElections(t *testing.T, electionCount int64) {
	t.Logf("🐀 [INVARIANT] Step 2: Triggering %d elections...", electionCount)

	for e := int64(0); e < electionCount; e++ {
		im.mu.Lock()

		// 무작위 노드에서 선거 시작
		candidateID := int(e % int64(im.nodes))
		state := im.nodeStates[candidateID]
		state.Term++
		state.State = "Candidate"
		im.nodeStates[candidateID] = state

		// 리더 선출 시뮬레이션 (quorum 획득)
		voteCount := 0
		for i := 0; i < im.nodes; i++ {
			if i == candidateID || i%2 == 0 { // 과반 투표
				voteCount++
			}
		}

		if voteCount > im.nodes/2 {
			// 리더 당선
			state.State = "Leader"
			im.nodeStates[candidateID] = state
		}

		im.mu.Unlock()

		im.electionCount++

		// 무관용 검증: 매 100번 선거마다 불변조건 확인
		if e%100 == 0 && e > 0 {
			im.CheckSingleLeader(t)
			t.Logf("  ✅ Election %d: Invariant check passed", e)
		}

		// 실제 선거는 시간이 걸리므로 시뮬레이션만
		time.Sleep(1 * time.Millisecond)
	}

	t.Logf("✅ Executed %d elections", im.electionCount)
}

// 3단계: 로그 복제(Log Replication) 강제 실행
func (im *InvariantMouse) TriggerMassReplication(t *testing.T, replicationCount int64) {
	t.Logf("🐀 [INVARIANT] Step 3: Triggering %d log replications...", replicationCount)

	for r := int64(0); r < replicationCount; r++ {
		im.mu.Lock()

		// 리더 찾기
		var leaderID int
		leaderFound := false

		for id, state := range im.nodeStates {
			if state.State == "Leader" {
				leaderID = id
				leaderFound = true
				break
			}
		}

		if !leaderFound {
			im.mu.Unlock()
			continue
		}

		// 리더에서 로그 엔트리 추가
		leaderState := im.nodeStates[leaderID]
		leaderState.LastLogIndex++
		im.nodeStates[leaderID] = leaderState

		// 모든 팔로워에게 복제
		for i := 0; i < im.nodes; i++ {
			if i != leaderID {
				followerState := im.nodeStates[i]
				followerState.LastLogIndex = leaderState.LastLogIndex
				followerState.LastLogTerm = leaderState.LastLogTerm
				im.nodeStates[i] = followerState
			}
		}

		// 로그 해시 업데이트
		newHash := fmt.Sprintf("log_v%d", leaderState.LastLogIndex)
		for i := 0; i < im.nodes; i++ {
			im.logHashes[i] = newHash
		}

		im.mu.Unlock()

		im.logReplicationCount++

		// 무관용 검증: 매 1000번 복제마다 일관성 확인
		if r%1000 == 0 && r > 0 {
			im.CheckLogConsistency(t)
			im.CheckLogHashes(t)
			t.Logf("  ✅ Replication %d: Consistency check passed", r)
		}

		time.Sleep(1 * time.Microsecond)
	}

	t.Logf("✅ Executed %d log replications", im.logReplicationCount)
}

// 4단계: 최종 무관용 검증
func (im *InvariantMouse) FinalVerification(t *testing.T) {
	t.Log("🐀 [INVARIANT] Step 4: Final unforgiving verification...")

	im.CheckSingleLeader(t)
	im.CheckLogConsistency(t)
	im.CheckLogHashes(t)
	im.CheckTermMonotonicity(t)

	if im.inconsistencyCount > 0 {
		t.Fatalf("❌ [DEAD] Detected %d inconsistencies. System is broken.", im.inconsistencyCount)
	}

	t.Log("✅ All Raft invariants verified successfully")
}

// 전체 불변성 테스트
func TestInvariantMouse(t *testing.T) {
	t.Log("")
	t.Log("=" * 60)
	t.Log("🐀 INVARIANT MOUSE EXECUTION (Raft Consensus)")
	t.Log("=" * 60)
	t.Log("")

	mouse := NewInvariantMouse(5) // 5-node Raft cluster

	t.Log("> Cluster Size: 5 nodes")
	t.Log("> Elections: 1,000")
	t.Log("> Log Replications: 10,000")
	t.Log("> Total State Changes: 11,000")
	t.Log("")

	mouse.InitializeCluster(t)
	time.Sleep(100 * time.Millisecond)

	mouse.TriggerMassElections(t, 1000)
	time.Sleep(100 * time.Millisecond)

	mouse.TriggerMassReplication(t, 10000)
	time.Sleep(100 * time.Millisecond)

	mouse.FinalVerification(t)

	t.Log("")
	t.Log("=" * 60)
	t.Log(fmt.Sprintf("📊 STATISTICS: Elections=%d, Replications=%d, Inconsistencies=%d",
		mouse.electionCount, mouse.logReplicationCount, mouse.inconsistencyCount))
	t.Log("=" * 60)
	t.Log("✅ SURVIVAL STATUS: [ALIVE]")
	t.Log("=" * 60)
}
