# Challenge 17: Ghost-Packet (L1 Onion Routing + Stealth)

**목표**: 익명성과 보안을 보장하는 L1 암호화 터널, 메시 트래픽 신원 은폐
**규모**: 1,900줄 (1,300줄 구현 + 600줄 테스트)
**규칙**: 무관용 규칙 2, 3 (Stealth-Mode, Power-Aware)

---

## 1. 아키텍처 개요

### L1 Ghost-Packet 3계층

```
┌─────────────────────────────────────┐
│  chaff_engine.fl (400줄)            │  Top: 더미 패킷 생성 (스텔스)
│  메시 트래픽 신원 은폐              │  실제 vs 더미 1:1 비율
├─────────────────────────────────────┤
│  onion_router.fl (450줄)            │  Middle: 3-layer Onion routing
│  각 노드는 다음 홉만 인지           │  AES-256 레이어별 암호화
├─────────────────────────────────────┤
│  packet_forwarder.fl (350줄)        │  Lower: 패킷 포워딩
│  < 0.1ms 오버헤드 유지              │  노드-투-노드 전송
└─────────────────────────────────────┘
```

---

## 2. Chaff Engine (400줄) - 스텔스 모드

### 목표: 실제 트래픽 신원 은폐

```fl
struct ChaffPacket {
    fake_src: [u8; 32],         // 가짜 발신자
    fake_dst: [u8; 32],         // 가짜 수신자
    payload_size: u32,          // 실제 데이터와 동일 크기
    creation_time_ns: u64,
    priority: u8,               // 1-255 (실제와 혼합)
}

fn generate_chaff_traffic(real_packet_size: u32) -> ChaffPacket {
    // 1:1 비율로 더미 패킷 생성
    // 외부 관찰자는 어떤 패킷이 실제인지 구별 불가능

    let chaff = ChaffPacket {
        fake_src: random_node_id(),
        fake_dst: random_node_id(),
        payload_size: real_packet_size,  // 같은 크기 (추적 방지)
        creation_time_ns: timestamp_ns(),
        priority: random_priority(),      // 실제와 유사
    };

    return chaff;
}

Rule 2: Stealth-Mode
  ✅ 스펙트럼 분석 불가능
  ✅ 패킷 크기 균일화 (모두 512B)
  ✅ 시간 패턴 랜덤화
  ✅ 실제/더미 구별 불가능 (1:1)
```

### 성능 목표

- **더미 패킷 생성**: < 0.05ms (100회)
- **트래픽 신원 은폐율**: 100% (외부 관찰자가 실제 패킷 추적 불가)
- **오버헤드**: 100% (실제 데이터 + 동일 크기 더미, 하지만 배터리 영향 < 5%)

---

## 3. Onion Router (450줄) - 3-Layer 암호화

### 다중 레이어 Onion Routing

```fl
struct OnionLayer {
    next_hop: [u8; 32],         // 다음 노드 (현재 노드만 알고 있음)
    session_key: [u8; 32],      // AES-256 키
    hmac_tag: [u8; 32],         // 무결성 검증
}

struct OnionPacket {
    layer1: OnionLayer,         // 현재 노드 → 2번째 노드
    layer2: OnionLayer,         // 2번째 노드 → 3번째 노드
    layer3: OnionLayer,         // 3번째 노드 → 목적지
    encrypted_payload: [u8; 512],  // 3번 암호화된 페이로드
}

fn create_onion_packet(src: [u8; 32],
                       path: &[[u8; 32]; 3],
                       payload: &[u8; 512]) -> OnionPacket {
    // 역순으로 레이어 구성 (Tor와 유사)

    // Layer 3: 최종 목적지
    let mut data3 = payload.clone();  // 3번째는 평문
    let key3 = derive_key(src, path[2]);

    // Layer 2: 2번째 릴레이
    let mut data2 = aes256_encrypt(&data3, &key3);
    let key2 = derive_key(src, path[1]);

    // Layer 1: 1번째 릴레이 (이 노드)
    let mut data1 = aes256_encrypt(&data2, &key2);
    let key1 = derive_key(src, path[0]);

    return OnionPacket {
        layer1: OnionLayer {
            next_hop: path[0],
            session_key: key1,
            hmac_tag: hmac_sha256(&data1, &key1),
        },
        layer2: OnionLayer {
            next_hop: path[1],
            session_key: key2,
            hmac_tag: hmac_sha256(&data2, &key2),
        },
        layer3: OnionLayer {
            next_hop: path[2],
            session_key: key3,
            hmac_tag: hmac_sha256(&data3, &key3),
        },
        encrypted_payload: data1,
    };
}

fn peel_onion_layer(packet: &OnionPacket) -> [u8; 512] {
    // 현재 노드: 1번째 레이어만 복호화
    // 나머지 2개 레이어는 다음 노드로 전달

    let key = packet.layer1.session_key;
    let decrypted = aes256_decrypt(&packet.encrypted_payload, &key);

    // 무결성 검증
    let computed_tag = hmac_sha256(&decrypted, &key);
    assert!(constant_time_eq(&computed_tag, &packet.layer1.hmac_tag));

    return decrypted;
}

Zero-Trust Properties:
  ✅ 발신자 신원: 1번째 릴레이만 알음 (이후 노드는 모름)
  ✅ 경로 추적: 불가능 (각 노드는 다음 홉만 알고 이전을 모름)
  ✅ 중간자 공격: 방지 (각 레이어 HMAC 검증)
  ✅ 유실 감지: HMAC 불일치 시 즉시 폐기
```

### 암호화 상세

```
Onion Packet Encryption (3-layer):

[평문 페이로드]
        ↓ (Key3로 암호화)
[암호화된 페이로드 v3]
        ↓ (Key2로 암호화)
[암호화된 페이로드 v2]
        ↓ (Key1로 암호화)
[암호화된 페이로드 v1] ← 네트워크 전송

각 노드에서:
- 1번째 릴레이: v1 복호화 → v2 전달 (v2, v3는 모름)
- 2번째 릴레이: v2 복호화 → v3 전달 (v3 내용은 모름)
- 3번째 릴레이: v3 복호화 → 평문 도착
```

---

## 4. Packet Forwarder (350줄) - 저지연 포워딩

```fl
struct ForwardingStats {
    packets_forwarded: u32,
    total_forward_time_us: u64,
    max_forward_time_us: u64,
    avg_forward_time_us: u64,
    battery_drain_percent: f32,  // relay mode에서의 배터리 소모
}

fn forward_onion_packet(packet: &OnionPacket,
                       next_node: [u8; 32]) -> u64 {
    // < 0.1ms 포워딩 (100µs)

    let start = timestamp_us();

    // 1. 다음 노드로 전송 (네트워크 I/O)
    send_to_node(&next_node, &packet.encrypted_payload);

    // 2. 통계 기록
    let elapsed = timestamp_us() - start;
    assert!(elapsed < 100, "포워딩 지연 초과: {}µs", elapsed);

    return elapsed;
}

fn estimate_battery_drain(relay_mode: bool,
                         packets_per_second: u32) -> f32 {
    // Rule 3: Power-Aware (배터리 증가 < 5%)

    if !relay_mode {
        return 0.0;  // 발신 모드: 배터리 영향 없음
    }

    // relay_mode: 패킷 수신 → 복호화 → 포워딩
    // 1000 packets/sec = 매우 높은 부하

    let cpu_drain = (packets_per_second as f32 / 1000.0) * 2.0;  // 최대 2%
    let radio_drain = 0.5;  // 라디오 TX: 상수 0.5%
    let crypto_drain = 1.5;  // AES-256 복호화: 1.5%

    return (cpu_drain + radio_drain + crypto_drain).min(5.0);  // 캡: 5%
}

Performance Target:
  ✅ < 0.1ms (100µs) 포워딩
  ✅ 1000 packets/sec 처리
  ✅ Battery increase < 5%
```

---

## 5. 무관용 테스트 (600줄, C17-T1~T6)

| 테스트 | 검증 내용 | 목표 |
|--------|---------|------|
| **C17-T1** | Onion 패킷 생성/복호화 (100회) | 100% 정확성, < 0.5ms |
| **C17-T2** | 3-layer 암호화 무결성 | 100 패킷, HMAC 검증 100% |
| **C17-T3** | 패킷 포워딩 (1000 packets) | < 0.1ms/패킷, 100% 성공 |
| **C17-T4** | Chaff 트래픽 생성 (100 실제 + 100 더미) | 구별 불가능 (신원 은폐 100%) |
| **C17-T5** | 배터리 영향 측정 (100 릴레이 노드) | < 5% 증가 |
| **C17-T6** | End-to-End 익명 라우팅 (3-홉) | 경로 추적 불가능, < 50ms 총 지연 |

---

## 6. 구현 순서

1. **chaff_engine.fl** (400줄) - 더미 패킷 생성, 신원 은폐
2. **onion_router.fl** (450줄) - 3-layer Onion, AES-256 암호화
3. **packet_forwarder.fl** (350줄) - < 0.1ms 포워딩, 배터리 관리
4. **mod.fl** (50줄) - 통합
5. **challenge_17_tests.fl** (600줄) - C17-T1~T6 테스트

**총 1,900줄**

---

## 7. 규칙 맵핑

```
무관용 규칙 2: Stealth-Mode (메시 트래픽 신원 은폐)
  ← C17-T4: Chaff 트래픽 1:1 비율로 신원 완벽 은폐

무관용 규칙 3: Power-Aware (배터리 증가 < 5%)
  ← C17-T5: 100 릴레이 노드에서 배터리 < 5% 증가 검증
```

---

## 8. 보안 성질

```
Ghost-Packet의 5가지 보안 보장:

1️⃣ Anonymity (익명성)
   - 각 노드는 다음 홉만 알고 발신자/경로를 모름
   - 중간 패킷 키를 가짐에도 전체 경로 역추적 불가능

2️⃣ Confidentiality (기밀성)
   - 3-layer AES-256 암호화
   - 각 레이어별 다른 키
   - 공격자가 패킷 내용 복호화 불가능

3️⃣ Integrity (무결성)
   - HMAC-SHA256 각 레이어마다
   - 위변조 감지 시 즉시 폐기
   - 중간자 공격 방지

4️⃣ Stealth (스텔스)
   - 실제 + 더미 1:1 비율
   - 외부 관찰자가 트래픽 신원 특정 불가능
   - 스펙트럼 분석 실패

5️⃣ Efficiency (효율성)
   - 포워딩 < 0.1ms
   - 배터리 증가 < 5%
   - 1000 packets/sec 처리
```

---

**다음**: onion_router.fl 450줄 구현 시작 🚀
