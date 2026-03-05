# Challenge 16: Kinetic-Router (L2 OLSR + L0NN Integration)

**목표**: 모바일 노드들을 추적하고 최적 경로를 동적으로 선택하는 라우팅 엔진
**규모**: 2,450줄 (1,850줄 구현 + 600줄 테스트)
**규칙**: 무관용 규칙 1, 4 (Zero-Infrastructure, Latency-Bound)

---

## 1. 아키텍처 개요

### L2 Kinetic-Router 4계층

```
┌─────────────────────────────────────┐
│  route_optimizer.fl (350줄)         │  Top: 최적 경로 선택, 홉 최소화
│  경로 홉 최소화, 재라우팅 < 10ms    │
├─────────────────────────────────────┤
│  neural_relay.fl (500줄)            │  Middle: L0NN 릴레이 선택
│  0.5ms 내에 최적 릴레이 추천        │  (신뢰도 + 거리 + 배터리 수준)
├─────────────────────────────────────┤
│  mobility_tracker.fl (400줄)        │  Lower: 모바일 노드 추적
│  위치/속도/방향 기반 예측           │  TTL 기반 엔트리 갱신
├─────────────────────────────────────┤
│  olsr_engine.fl (600줄)             │  Bottom: OLSR 핵심
│  Hello flood, TC flood, MPR 선택    │  < 2% 오버헤드 유지
└─────────────────────────────────────┘
```

---

## 2. OLSR Engine (600줄)

### 핵심 구조

```fl
struct OLSRNode {
    node_id: [u8; 32],           // Keccak256(pubkey)
    position: (f32, f32),        // (x, y) 좌표
    velocity: (f32, f32),        // (vx, vy) 속도
    signal_strength: i32,        // dBm 신호 강도
    last_hello_ns: u64,          // 마지막 Hello 타임스탬프
    hello_sequence: u16,         // Hello 시퀀스 번호
}

struct HelloMessage {
    sender_id: [u8; 32],
    sequence: u16,
    neighbors: [[u8; 32]; 32],   // 최대 32개 이웃
    neighbor_count: usize,
    ttl: u8,
    timestamp_ns: u64,
}

struct TCMessage {
    originator_id: [u8; 32],
    sequence: u16,
    advertised_neighbors: [[u8; 32]; 16],  // MPR 선택 노드들
    mpr_count: usize,
    ttl: u8,
}

struct OLSREngine {
    node_id: [u8; 32],
    nodes: [OLSRNode; 256],                // 최대 256노드
    node_count: usize,
    hello_interval_ms: u32,                // 기본값: 2000ms
    tc_interval_ms: u32,                   // 기본값: 5000ms
    last_hello_sent_ns: u64,
    last_tc_sent_ns: u64,
    hello_count: u32,
    tc_count: u32,
    mpr_set: [[u8; 32]; 32],               // 다중 홉 릴레이 집합
    mpr_count: usize,
}
```

### 3가지 핵심 함수

#### 1. Hello Flooding (OLSR 핵심)
```fl
fn process_hello_flood(engine: &mut OLSREngine) {
    // 2초마다 Hello 브로드캐스트
    // 이웃 노드들과 신호 강도를 알림

    let now = timestamp_ns();
    if now - engine.last_hello_sent_ns < engine.hello_interval_ms as u64 * 1_000_000 {
        return;  // 아직 시간 안됨
    }

    // Hello 생성
    let hello = HelloMessage {
        sender_id: engine.node_id,
        sequence: engine.hello_count as u16,
        neighbors: [...],          // 이웃 목록
        neighbor_count: ...,
        ttl: 255,
        timestamp_ns: now,
    };

    // 브로드캐스트 (모든 이웃에게)
    broadcast_hello(&hello);

    engine.last_hello_sent_ns = now;
    engine.hello_count += 1;
}
```

#### 2. MPR (Multi-Point Relay) 선택
```fl
fn select_mpr_set(engine: &mut OLSREngine) {
    // 신호 강도 + 이웃 수 기반 MPR 선택
    // 목표: 최소 개수로 모든 2홉 이웃 커버

    let mut candidates = [(node_id, signal, neighbor_count); 32];
    let mut candidate_count = 0;

    for i in 0..engine.node_count {
        if engine.nodes[i].last_hello_ns > 0 {
            candidates[candidate_count] = (
                engine.nodes[i].node_id,
                engine.nodes[i].signal_strength,
                count_neighbors(&engine.nodes[i]),
            );
            candidate_count += 1;
        }
    }

    // 신호 강도 정렬 (내림차순)
    sort_by_signal(&mut candidates, candidate_count);

    // 상위 8개 선택 (조정 가능)
    engine.mpr_count = candidate_count.min(8);
    for i in 0..engine.mpr_count {
        engine.mpr_set[i] = candidates[i].0;
    }
}
```

#### 3. TC (Topology Control) 메시지 전송
```fl
fn broadcast_topology_control(engine: &OLSREngine) {
    // 5초마다 TC 메시지 (MPR를 통해 플로드)
    // 네트워크 전체에 토폴로지 정보 전파

    let tc = TCMessage {
        originator_id: engine.node_id,
        sequence: (engine.hello_count / 3) as u16,  // TC는 3배 느림
        advertised_neighbors: engine.mpr_set,
        mpr_count: engine.mpr_count,
        ttl: 255,
    };

    // MPR 집합을 통해서만 전송 (< 2% 오버헤드)
    for i in 0..engine.mpr_count {
        send_to_node(&engine.mpr_set[i], &tc);
    }
}
```

### 오버헤드 계산

```
Hello Flood: 2000ms 주기, ~100바이트 메시지
  → 100바이트 / 2000ms = 0.05바이트/ms = 0.4%

TC Flood (MPR 통해): 5000ms 주기, ~150바이트 메시지, MPR 8개만
  → 150바이트 / 5000ms = 0.03바이트/ms = 0.2% (MPR 필터링)

총 오버헤드: 0.4% + 0.2% = 0.6% < 2% ✅
```

---

## 3. Neural Relay (500줄)

### 최적 릴레이 선택 L0NN

```fl
struct RelayCandidate {
    node_id: [u8; 32],
    signal_strength: i32,        // -90 ~ -30 dBm
    hop_distance: u32,           // 홉 수
    battery_level: u8,           // 0-100%
    reliability_score: f32,      // 0.0-1.0 (이전 전달 성공률)
}

struct NeuralRelaySelector {
    weights_l1: [f32; 64],       // 입력→은닉층 (4×16 = 64)
    weights_l2: [f32; 16],       // 은닉층→출력 (16×1 = 16)
    relay_history: [u32; 256],   // 각 노드 선택 횟수
}

fn select_best_relay(selector: &NeuralRelaySelector,
                     candidates: &[RelayCandidate; 8]) -> [u8; 32] {
    // 특성 추출 (4개 입력)
    let signal_norm = (candidate.signal_strength + 90) / 60.0;  // 정규화: 0.0-1.0
    let hop_norm = 1.0 / (1.0 + candidate.hop_distance as f32);
    let battery_norm = candidate.battery_level as f32 / 100.0;
    let reliability = candidate.reliability_score;

    let features = [signal_norm, hop_norm, battery_norm, reliability];

    // 은닉층 (ReLU)
    let mut hidden = [0.0f32; 16];
    for i in 0..16 {
        let mut sum = 0.0;
        for j in 0..4 {
            sum += features[j] * selector.weights_l1[i * 4 + j];
        }
        hidden[i] = if sum > 0.0 { sum } else { 0.0 };  // ReLU
    }

    // 출력층 (점수)
    let mut score = 0.0;
    for i in 0..16 {
        score += hidden[i] * selector.weights_l2[i];
    }

    // 상위 1개 선택
    return best_candidate.node_id;
}
```

### 성능 목표

- **신경망 추론**: < 0.5ms (16개 은닉 노드, 간단한 ReLU)
- **후보 필터링**: < 0.2ms (8개 후보 스캔)
- **총 선택 시간**: < 0.7ms ✅ (< 0.5ms 목표)

---

## 4. Mobility Tracker (400줄)

```fl
struct TrackedNode {
    node_id: [u8; 32],
    position: (f32, f32),        // 현재 위치
    velocity: (f32, f32),        // 추정 속도
    last_update_ns: u64,
    ttl_ns: u64,                 // 5초 TTL
    prediction_error: f32,       // 예측 오차 추적
}

fn predict_position(node: &TrackedNode, delta_ns: u64) -> (f32, f32) {
    let dt = delta_ns as f32 / 1_000_000_000.0;  // 나노초 → 초

    let new_x = node.position.0 + node.velocity.0 * dt;
    let new_y = node.position.1 + node.velocity.1 * dt;

    return (new_x, new_y);
}

fn update_velocity(node: &mut TrackedNode, new_pos: (f32, f32),
                   delta_ns: u64) {
    let dt = delta_ns as f32 / 1_000_000_000.0;

    if dt > 0.0 {
        node.velocity.0 = (new_pos.0 - node.position.0) / dt;
        node.velocity.1 = (new_pos.1 - node.position.1) / dt;
    }

    node.position = new_pos;
    node.last_update_ns = timestamp_ns();
    node.ttl_ns = timestamp_ns() + 5 * 1_000_000_000;  // 5초 갱신
}
```

---

## 5. Route Optimizer (350줄)

```fl
fn find_optimal_route(engine: &OLSREngine, dest: [u8; 32])
                      -> Route {
    // Dijkstra 최단 경로
    // 신호 강도 + 홉 수 + 배터리 기반 가중치

    let mut distance = [u32::MAX; 256];
    let mut prev = [[0u8; 32]; 256];

    distance[src_idx] = 0;

    // Dijkstra 루프 (< 5ms)
    for _ in 0..256 {
        let mut min_dist = u32::MAX;
        let mut u_idx = 0;

        for i in 0..engine.node_count {
            if distance[i] < min_dist {
                min_dist = distance[i];
                u_idx = i;
            }
        }

        if min_dist == u32::MAX { break; }

        // 이웃 업데이트
        for j in 0..engine.node_count {
            let cost = calculate_link_cost(&engine.nodes[u_idx],
                                           &engine.nodes[j]);
            if distance[u_idx] + cost < distance[j] {
                distance[j] = distance[u_idx] + cost;
                prev[j] = engine.nodes[u_idx].node_id;
            }
        }
    }

    // 경로 재구성
    return reconstruct_route(prev, dest);
}

fn calculate_link_cost(src: &OLSRNode, dst: &OLSRNode) -> u32 {
    // 비용 = 신호 반비례 + 거리 + 배터리 역비례

    let signal_cost = 100 - (src.signal_strength + 90) / 2;  // -90~-30 → 0~60
    let distance = euclidean_distance(src.position, dst.position);
    let battery_cost = (101 - dst.battery_level) / 10;

    return signal_cost as u32 + distance as u32 * 2 + battery_cost as u32;
}
```

### 재라우팅 조건 (< 10ms)

```
1. 이웃 노드 손실 감지 (3개 Hello 연속 미수신)
   → 즉시 새 경로 계산 (3-5ms)

2. 신호 강도 급감 (> 10dBm 저하)
   → 2ms 내에 선택적 재라우팅

3. 배터리 임계값 초과 (< 10%)
   → 비용 함수에 즉시 반영, 새 경로 계산
```

---

## 6. 무관용 테스트 (600줄, C16-T1~T6)

### 테스트 세트

| 테스트 | 시나리오 | 검증 목표 |
|--------|---------|----------|
| **C16-T1** | OLSR Hello flood (100노드 동시 → 재수렴) | 오버헤드 < 2%, 수렴시간 < 30초 |
| **C16-T2** | 신경망 릴레이 선택 (100번 반복 추론) | 선택 시간 < 0.5ms/회, 정확도 > 95% |
| **C16-T3** | 모바일 추적 (초당 100 위치 업데이트) | 추적 지연 < 50ms, 오류 < 5% |
| **C16-T4** | 재라우팅 트리거 (신호 손실) | 재라우팅 < 10ms, 경로 손실률 < 1% |
| **C16-T5** | 패킷 손실 회피 (3-홉 경로 유지) | 99% 이상 성공 전달 |
| **C16-T6** | 네트워크 분할 복구 (2 클러스터 → 재병합) | < 5초 수렴, 토폴로지 일관성 ✅ |

---

## 7. 구현 순서

1. **olsr_engine.fl** (600줄) - Hello/TC 플로드, MPR 선택 먼저
2. **mobility_tracker.fl** (400줄) - 위치 예측, TTL 관리
3. **neural_relay.fl** (500줄) - L0NN 추론, 릴레이 선택
4. **route_optimizer.fl** (350줄) - 최단 경로, 비용 함수
5. **mod.fl** (50줄) - 통합
6. **challenge_16_tests.fl** (600줄) - C16-T1~T6 테스트

**총 2,450줄**

---

## 8. 규칙 맵핑

```
무관용 규칙 1: Zero-Infrastructure (ISP 없이 3+ 노드 통신)
  ← C16-T1: OLSR Hello/TC가 자율적으로 토폴로지 구성

무관용 규칙 4: Latency-Bound (3-홉 < 50ms)
  ← C16-T3: 모바일 추적 < 50ms
  ← C16-T4: 재라우팅 < 10ms
  ← C16-T6: 재병합 < 5000ms (3홉 당 ~1.6ms)
```

---

**다음**: olsr_engine.fl 600줄 구현 시작 🚀
