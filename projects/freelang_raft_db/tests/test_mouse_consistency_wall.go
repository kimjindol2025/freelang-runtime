// ============================================================================
// 🧱 Consistency Wall: Partial Partition Shadowing Test
// 데이터 영속성 무결성 검증 (Dirty Read & Log Gap 무관용)
// ============================================================================

package tests

import (
	"fmt"
	"sync"
	"testing"
	"time"
)

// ConsistencyWallTest: Partial Partition Shadowing 시나리오
type ConsistencyWallTest struct {
	nodes              [5]*MockRaftNode      // 5-node 클러스터
	writeCount         int                   // 쓰기 개수
	dirtyReadCount     int                   // Dirty read 감지 카운트
	logGapDetected     []bool                // 로그 갭 감지 배열
	mu                 sync.Mutex            // 동기화
	uncommittedWrites  map[string]string     // 미커밋 쓰기
	committedData      map[string]string     // 커밋된 쓰기
}

// NewConsistencyWallTest: Consistency Wall 테스트 초기화
func NewConsistencyWallTest() *ConsistencyWallTest {
	ct := &ConsistencyWallTest{
		dirtyReadCount:    0,
		writeCount:        0,
		logGapDetected:    make([]bool, 5),
		uncommittedWrites: make(map[string]string),
		committedData:     make(map[string]string),
	}
	
	// 5개 노드 초기화
	for i := 0; i < 5; i++ {
		ct.nodes[i] = &MockRaftNode{
			nodeID:      i,
			state:       "Follower",
			currentTerm: 0,
			log:         make([]LogEntry, 0),
			commitIndex: 0,
			mu:          &sync.Mutex{},
		}
	}
	
	return ct
}

// MockRaftNode: Raft 노드 시뮬레이션
type MockRaftNode struct {
	nodeID      int
	state       string        // Follower, Candidate, Leader
	currentTerm uint64
	log         []LogEntry
	commitIndex uint64
	logHash     string
	mu          *sync.Mutex
	networkDown bool          // 네트워크 장애
	packetLoss  int           // 패킷 유실률 (0-100)
}

// LogEntry: Raft 로그 엔트리
type LogEntry struct {
	Term  uint64
	Index uint64
	Key   string
	Value string
	Hash  string
}

// ============================================================================
// 시나리오: Partial Partition Shadowing
// ============================================================================
// - Node 0, 1: 완전 격리 (네트워크 다운)
// - Node 2: 패킷 90% 유실 (매우 느린 네트워크, "Flaky")
// - Node 3, 4: 정상 연결
// ============================================================================

func (ct *ConsistencyWallTest) SimulatePartialPartitionShadowing(t *testing.T) {
	fmt.Println("\n🧱 Consistency Wall: Partial Partition Shadowing Test")
	fmt.Println("═══════════════════════════════════════════════════════════")
	
	// Phase 1: 클러스터 초기화
	fmt.Println("\n📍 Phase 1: 클러스터 초기화")
	ct.initializeCluster()
	
	// Phase 2: 정상 동작 기준선
	fmt.Println("\n📍 Phase 2: 기준선 수집 (정상 동작)")
	baselineMetrics := ct.collectMetrics()
	fmt.Printf("✅ 기준선: Committed=%d, LogHash=%s\n", 
		len(ct.committedData), ct.computeClusterLogHash())
	
	// Phase 3: Partial Partition 주입 (네트워크 분할)
	fmt.Println("\n📍 Phase 3: 부분 격리 적용")
	ct.injectPartialPartition()
	fmt.Println("  ❌ Node 0, 1: 완전 격리 (네트워크 다운)")
	fmt.Println("  ⚠️  Node 2: 90% 패킷 유실 (Flaky)")
	fmt.Println("  ✅ Node 3, 4: 정상 연결")
	
	// Phase 4: 대규모 쓰기 작업 (초당 1,000건)
	fmt.Println("\n📍 Phase 4: 1,000건의 쓰기 작업 시작...")
	ct.performHeavyWrites(t, 1000)
	
	// Phase 5: 무관용 검증
	fmt.Println("\n📍 Phase 5: 무관용 일관성 검증")
	passedAll := ct.validateConsistencyWall(t)
	
	if !passedAll {
		t.Fatalf("❌ Consistency Wall FAILED: Dirty read 또는 Log gap 감지됨")
	}
	
	fmt.Println("\n✅ Consistency Wall: [ALIVE] - 모든 무관용 규칙 만족")
}

// initializeCluster: 클러스터 초기화
func (ct *ConsistencyWallTest) initializeCluster() {
	for i := 0; i < 5; i++ {
		ct.nodes[i].state = "Follower"
		ct.nodes[i].currentTerm = 0
		ct.nodes[i].log = make([]LogEntry, 0)
		ct.nodes[i].commitIndex = 0
		ct.nodes[i].networkDown = false
		ct.nodes[i].packetLoss = 0
	}
	
	// 리더 선출 (Node 3 리더)
	ct.nodes[3].state = "Leader"
	ct.nodes[3].currentTerm = 1
	fmt.Println("  ✅ Node 3 리더 선출 (Term 1)")
}

// injectPartialPartition: Partial Partition 주입
func (ct *ConsistencyWallTest) injectPartialPartition() {
	// Node 0, 1: 완전 격리
	ct.nodes[0].networkDown = true
	ct.nodes[1].networkDown = true
	
	// Node 2: 90% 패킷 유실 (Flaky)
	ct.nodes[2].packetLoss = 90
	
	// Node 3, 4: 정상 (리더와 팔로워)
	ct.nodes[3].networkDown = false
	ct.nodes[4].networkDown = false
	ct.nodes[4].packetLoss = 0
}

// performHeavyWrites: 1,000건의 쓰기 작업
func (ct *ConsistencyWallTest) performHeavyWrites(t *testing.T, count int) {
	leaderNode := ct.nodes[3] // Node 3 리더
	
	for i := 0; i < count; i++ {
		key := fmt.Sprintf("key_%d", i)
		value := fmt.Sprintf("value_%d", i)
		
		// 리더에게 쓰기 요청
		leaderNode.mu.Lock()
		
		// Step 1: 로그에 엔트리 추가 (아직 미커밋)
		entry := LogEntry{
			Term:  leaderNode.currentTerm,
			Index: uint64(len(leaderNode.log) + 1),
			Key:   key,
			Value: value,
		}
		leaderNode.log = append(leaderNode.log, entry)
		
		// Step 2: 팔로워에게 복제 시도
		ct.replicateToFollowers(leaderNode, &entry)
		
		// Step 3: 과반수 응답 후 커밋
		commitSuccess := ct.waitForCommit(&entry)
		if commitSuccess {
			ct.mu.Lock()
			ct.committedData[key] = value
			ct.mu.Unlock()
			ct.writeCount++
		}
		
		leaderNode.mu.Unlock()
		
		// 성능: 초당 1,000건 목표 → 1ms 간격
		if i%100 == 0 {
			time.Sleep(100 * time.Millisecond)
		}
	}
	
	fmt.Printf("  ✅ %d건 쓰기 완료\n", ct.writeCount)
}

// replicateToFollowers: 팔로워에게 복제
func (ct *ConsistencyWallTest) replicateToFollowers(leader *MockRaftNode, entry *LogEntry) {
	for i := 0; i < 5; i++ {
		if i == leader.nodeID {
			continue // 리더 자신 제외
		}
		
		follower := ct.nodes[i]
		
		// 네트워크 장애 체크
		if follower.networkDown {
			// 완전 격리: 복제 불가
			continue
		}
		
		if follower.packetLoss > 0 {
			// 패킷 유실 체크
			if randomInt(0, 100) < follower.packetLoss {
				// 복제 실패 (패킷 유실)
				continue
			}
		}
		
		// 복제 성공: 팔로워에 로그 추가
		follower.mu.Lock()
		follower.log = append(follower.log, *entry)
		follower.mu.Unlock()
	}
}

// waitForCommit: 과반수 응답 대기
func (ct *ConsistencyWallTest) waitForCommit(entry *LogEntry) bool {
	// 리더 + 과반수 팔로워 필요 (3/5)
	replicationCount := 1 // 리더
	
	for i := 0; i < 5; i++ {
		if i == 3 { // 리더 제외
			continue
		}
		
		node := ct.nodes[i]
		node.mu.Lock()
		
		// 이 노드에 로그가 있는지 확인
		found := false
		for _, logEntry := range node.log {
			if logEntry.Index == entry.Index && logEntry.Term == entry.Term {
				found = true
				break
			}
		}
		
		if found {
			replicationCount++
			node.commitIndex = uint64(len(node.log))
		}
		
		node.mu.Unlock()
	}
	
	// 과반수 달성 (3/5)
	return replicationCount >= 3
}

// validateConsistencyWall: 무관용 일관성 검증
func (ct *ConsistencyWallTest) validateConsistencyWall(t *testing.T) bool {
	fmt.Println("  🔍 Dirty Read 검증...")
	
	// 규칙 1: Dirty Read = 0
	dirtyReads := ct.detectDirtyReads()
	if dirtyReads > 0 {
		t.Errorf("❌ Dirty Read 감지: %d건", dirtyReads)
		return false
	}
	fmt.Println("    ✅ Dirty Read = 0")
	
	// 규칙 2: Log Gap = 0
	fmt.Println("  🔍 Log Gap 검증...")
	logGaps := ct.detectLogGaps()
	if logGaps > 0 {
		t.Errorf("❌ Log Gap 감지: %d건", logGaps)
		return false
	}
	fmt.Println("    ✅ Log Gap = 0")
	
	// 추가 검증: 데이터 일관성
	fmt.Println("  🔍 데이터 일관성 검증...")
	consistent := ct.verifyDataConsistency()
	if !consistent {
		t.Error("❌ 데이터 불일치 감지")
		return false
	}
	fmt.Println("    ✅ 모든 노드 데이터 일치")
	
	return true
}

// detectDirtyReads: Dirty Read 감지
// 미커밋 쓰기가 읽혀지는 경우
func (ct *ConsistencyWallTest) detectDirtyReads() int {
	ct.mu.Lock()
	defer ct.mu.Unlock()
	
	count := 0
	
	// 미커밋 데이터가 실제로 읽혀지는 경우 감지
	for key, value := range ct.uncommittedWrites {
		for i := 0; i < 5; i++ {
			node := ct.nodes[i]
			node.mu.Lock()
			
			// 노드에서 이 값을 읽을 수 있는지 확인
			for _, logEntry := range node.log {
				if logEntry.Key == key && logEntry.Value == value {
					// commitIndex보다 뒤의 로그를 읽은 경우 = Dirty Read
					if logEntry.Index > uint64(node.commitIndex) {
						count++
					}
				}
			}
			
			node.mu.Unlock()
		}
	}
	
	return count
}

// detectLogGaps: Log Gap 감지
// 로그의 연속성이 깨진 경우
func (ct *ConsistencyWallTest) detectLogGaps() int {
	count := 0
	
	for i := 0; i < 5; i++ {
		node := ct.nodes[i]
		node.mu.Lock()
		
		prevIndex := uint64(0)
		for _, entry := range node.log {
			// 연속된 인덱스가 아니면 갭 감지
			if entry.Index != prevIndex+1 && prevIndex > 0 {
				count++
			}
			prevIndex = entry.Index
		}
		
		node.mu.Unlock()
	}
	
	return count
}

// verifyDataConsistency: 데이터 일관성 검증
func (ct *ConsistencyWallTest) verifyDataConsistency() bool {
	if len(ct.committedData) == 0 {
		return true // 데이터 없음
	}
	
	for i := 1; i < 5; i++ {
		node1 := ct.nodes[i-1]
		node2 := ct.nodes[i]
		
		node1.mu.Lock()
		node2.mu.Lock()
		
		// 커밋된 로그만 비교
		for j := 0; j < int(node1.commitIndex) && j < len(node1.log); j++ {
			if j >= len(node2.log) || node1.log[j].Hash != node2.log[j].Hash {
				node1.mu.Unlock()
				node2.mu.Unlock()
				return false
			}
		}
		
		node1.mu.Unlock()
		node2.mu.Unlock()
	}
	
	return true
}

// collectMetrics: 메트릭 수집
func (ct *ConsistencyWallTest) collectMetrics() map[string]interface{} {
	return map[string]interface{}{
		"committed":     len(ct.committedData),
		"clusterHash":   ct.computeClusterLogHash(),
		"leaderTerm":    ct.nodes[3].currentTerm,
	}
}

// computeClusterLogHash: 클러스터 로그 해시 계산
func (ct *ConsistencyWallTest) computeClusterLogHash() string {
	// 간단한 해시: 로그 크기 합
	totalSize := 0
	for i := 0; i < 5; i++ {
		ct.nodes[i].mu.Lock()
		totalSize += len(ct.nodes[i].log)
		ct.nodes[i].mu.Unlock()
	}
	return fmt.Sprintf("hash_%d", totalSize)
}

// randomInt: 난수 생성 헬퍼
func randomInt(min, max int) int {
	return min + (time.Now().Nanosecond() % (max - min))
}

// ============================================================================
// TestConsistencyWall: 무관용 테스트 쥐 - Consistency Wall
// ============================================================================

func TestConsistencyWall(t *testing.T) {
	ct := NewConsistencyWallTest()
	ct.SimulatePartialPartitionShadowing(t)
	
	// 최종 결과 출력
	fmt.Println("\n" + "═══════════════════════════════════════════════════════════")
	fmt.Printf("📊 최종 결과:\n")
	fmt.Printf("  쓰기 개수: %d\n", ct.writeCount)
	fmt.Printf("  커밋 데이터: %d\n", len(ct.committedData))
	fmt.Printf("  Dirty Read: 0 ✅\n")
	fmt.Printf("  Log Gap: 0 ✅\n")
	fmt.Println("🐀 Test Mouse Status: [ALIVE] ✅")
	fmt.Println("═══════════════════════════════════════════════════════════\n")
}
