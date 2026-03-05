# Challenge L0: Physical Radio Layer (하드웨어 통합)

**목표**: 실제 무선 하드웨어 제어 (GPIO, RF, LoRa)
**규모**: 1,850줄 (1,250줄 구현 + 600줄 테스트)
**철학**: "L1-L3 추상화를 L0 하드웨어로 구현"

---

## 1. L0 아키텍처

### Sovereign-Mesh 5계층 완성

```
┌─────────────────────────────────────┐
│  L3: mesh_api.fl                    │  사용자 API
├─────────────────────────────────────┤
│  L2: kinetic_route.fl               │  라우팅 (Challenge 16) ✅
├─────────────────────────────────────┤
│  L1: ghost_packet.fl                │  암호화 (Challenge 17) ✅
├─────────────────────────────────────┤
│  L0: radio_hal.fl (400줄)           │  ← NEW
│      wifi_direct.fl (450줄)         │  ← NEW
│      lora_modem.fl (400줄)          │  ← NEW
└─────────────────────────────────────┘
```

---

## 2. Radio HAL (400줄) - GPIO/I2C/SPI 제어

### 저수준 인터페이스

```fl
pub struct RadioHAL {
    gpio_pins: [GPIOPin; 32],         // GPIO 0-31
    i2c_bus: I2CBus,                  // I2C (address 7-bit)
    spi_bus: SPIBus,                  // SPI (CLK, MOSI, MISO)
    interrupt_handlers: [fn(); 32],   // ISR 핸들러
    initialized: bool,
}

pub struct GPIOPin {
    pin_id: u8,                       // GPIO 번호 (0-31)
    mode: PinMode,                    // INPUT, OUTPUT, ALT
    level: bool,                      // 전류 상태 (HIGH/LOW)
    irq_enabled: bool,
}

pub enum PinMode {
    INPUT,                            // 입력 (고임피던스)
    OUTPUT,                           // 출력 (드라이브)
    ALT,                              // 대체 함수 (I2C, SPI, UART)
}

impl RadioHAL {
    // GPIO 제어
    pub fn gpio_set_mode(pin: u8, mode: PinMode) {
        // 1. 레지스터 매핑 (ARM MMIO)
        // 2. 모드 비트 설정
        // 3. 상태 기록
    }

    pub fn gpio_write(pin: u8, level: bool) {
        // 1. 핀 모드 확인 (OUTPUT이어야 함)
        // 2. GPIO 레지스터 쓰기
        // 3. 타이밍: < 100ns
    }

    pub fn gpio_read(pin: u8) -> bool {
        // 1. 레지스터 읽기
        // 2. 비트 추출
        // 3. 반환
    }

    pub fn gpio_irq_enable(pin: u8, handler: fn()) {
        // 1. IRQ 핸들러 등록
        // 2. 인터럽트 활성화
        // 3. 엣지 설정 (RISING, FALLING, BOTH)
    }

    // I2C 통신 (센서, 디바이스 제어)
    pub fn i2c_write(addr: u8, data: &[u8]) -> Result {
        // 1. START 신호
        // 2. 주소 + WRITE 비트
        // 3. 데이터 송신 (8비트 × N)
        // 4. STOP 신호
        // 시간: < 10ms
    }

    pub fn i2c_read(addr: u8, len: usize) -> Result<[u8; 256]> {
        // 1. START 신호
        // 2. 주소 + READ 비트
        // 3. 데이터 수신 (8비트 × len)
        // 4. STOP 신호
    }

    // SPI 통신 (고속, RF 칩)
    pub fn spi_transfer(data: &[u8]) -> [u8; 256] {
        // 1. CS 활성화 (LOW)
        // 2. 클록 생성, 데이터 송수신 (MSB first)
        // 3. CS 비활성화 (HIGH)
        // 시간: < 1ms (1MHz 기준)
    }

    pub fn spi_set_speed(speed_hz: u32) {
        // 1MHz - 10MHz 범위 설정
    }
}
```

**성능 목표**:
- GPIO: < 100ns 응답
- I2C: < 10ms 전송
- SPI: < 1ms 전송 (1MHz 기준)

---

## 3. Wi-Fi Direct (450줄) - P2P 연결

### Wi-Fi Direct 기본 통신

```fl
pub struct WiFiDirectInterface {
    mac_address: [u8; 6],             // 자신의 MAC
    peers: [WiFiPeer; 32],            // 페어링된 피어
    peer_count: usize,
    power_state: PowerState,          // ACTIVE, SLEEP, OFF
    signal_strength_dbm: i32,
    connected_peer: Option<[u8; 6]>,  // 현재 연결 피어
}

pub struct WiFiPeer {
    mac_address: [u8; 6],
    ssid: [u8; 32],
    signal_strength: i32,             // dBm
    last_seen_ns: u64,
    connection_priority: u8,          // 1-255
}

impl WiFiDirectInterface {
    pub fn scan_peers(duration_ms: u32) -> [WiFiPeer; 32] {
        // 1. Wi-Fi 스캔 시작 (채널 1-13)
        // 2. 비콘 수신 및 파싱
        // 3. SSID, 신호 강도 기록
        // 4. 결과 반환 (최대 32개)
        // 시간: < 5000ms
    }

    pub fn connect_to_peer(mac: [u8; 6]) -> Result {
        // 1. WPA3 핸드셰이크
        // 2. 4-way handshake
        // 3. DHCP 또는 정적 IP
        // 4. 연결 확인
        // 시간: < 3000ms
    }

    pub fn send_data(data: &[u8; 512]) -> Result {
        // 1. 802.11ac 프레임 생성
        // 2. 암호화 (WPA3)
        // 3. 전송
        // 4. ACK 수신 대기 (< 100ms)
        // 시간: < 100ms
    }

    pub fn receive_data() -> Option<[u8; 512]> {
        // 1. 수신 큐 확인
        // 2. 프레임 복호화
        // 3. CRC 검증
        // 4. 반환
    }

    pub fn set_power_state(state: PowerState) {
        // ACTIVE: 항상 켜짐
        // SLEEP: 10초마다 깨어남 (배터리 절약)
        // OFF: 완전 종료
    }

    pub fn get_signal_strength() -> i32 {
        // -100 ~ -30 dBm 범위
        // RSSI (Received Signal Strength Indicator)
    }

    pub fn disconnect() {
        // 1. 4-way handshake 해제
        // 2. Wi-Fi 끄기
    }
}

pub enum PowerState {
    ACTIVE,   // 100% 전력
    SLEEP,    // 10% 전력
    OFF,      // 0% 전력
}
```

**성능 목표**:
- 스캔: < 5s
- 연결: < 3s
- 송신: < 100ms/패킷
- 수신: 지연 < 50ms
- 처리량: 54 Mbps (Wi-Fi 6 기준)

---

## 4. LoRa Modem (400줄) - 장거리 통신

### LoRa 변조 및 통신

```fl
pub struct LoRaModem {
    frequency_hz: u32,                // 915MHz (US), 868MHz (EU)
    bandwidth: LoRaBandwidth,         // 125kHz, 250kHz, 500kHz
    spreading_factor: u8,             // 7-12 (더 높을수록 느림/먼거리)
    coding_rate: u8,                  // 5-8
    transmit_power_dbm: i8,           // 0-20 dBm
    modem_state: ModemState,
    packets_sent: u32,
    packets_received: u32,
    range_km: f32,                    // 최대 범위 (이론값)
}

pub enum LoRaBandwidth {
    BW125,   // 125 kHz (먼거리)
    BW250,   // 250 kHz (중간거리)
    BW500,   // 500 kHz (근거리, 고속)
}

pub enum ModemState {
    IDLE,
    RX,
    TX,
    CAD,     // Channel Activity Detection
}

impl LoRaModem {
    pub fn configure(freq: u32,
                    bw: LoRaBandwidth,
                    sf: u8,
                    power: i8) {
        // 1. SPI를 통해 모뎀 설정
        // 2. 주파수, 대역폭, SF 설정
        // 3. 전송 전력 설정
        // 4. 상태: IDLE
    }

    pub fn transmit(data: &[u8; 256]) -> Result {
        // 1. FIFO에 데이터 로드
        // 2. 전송 시작 (< 2000ms, SF에 따라 다름)
        // 3. 전송 완료 대기
        // 시간: 41ms (SF7) ~ 2000ms (SF12)
    }

    pub fn receive(timeout_ms: u32) -> Option<[u8; 256]> {
        // 1. RX 모드 시작
        // 2. 신호 대기
        // 3. 프레임 수신 및 복호화
        // 4. CRC 검증
        // 시간: timeout_ms까지
    }

    pub fn cad_scan() -> bool {
        // Channel Activity Detection
        // 채널에 신호가 있는지 빠르게 확인
        // 시간: < 100ms
    }

    pub fn get_signal_strength() -> i32 {
        // -140 ~ 10 dBm 범위
        // RSSI
    }

    pub fn get_snr() -> f32 {
        // Signal-to-Noise Ratio
        // -20 ~ 10 dB
    }

    pub fn calculate_range(&self) -> f32 {
        // Friis 방정식: 거리 = 10^((TX_POWER - RX_THRESHOLD) / 20)
        // 예: SF12, BW125 @ 915MHz → 최대 40km
        // SF7, BW500 @ 915MHz → 최대 2km
        return self.estimate_range();
    }

    fn estimate_range(&self) -> f32 {
        // Spreading Factor에 따른 범위 계산
        match self.spreading_factor {
            7 => 2.0,
            8 => 4.0,
            9 => 8.0,
            10 => 16.0,
            11 => 25.0,
            12 => 40.0,
            _ => 0.0,
        }
    }
}
```

**성능 목표**:
- 범위: 2-40km (SF에 따라)
- 처리량: 50bps (SF12) ~ 5.5kbps (SF7)
- 전력: < 200mW TX, < 10mW RX
- 패킷: < 2000ms 전송

---

## 5. 무관용 테스트 (600줄)

| 테스트 | 검증 내용 | 목표 |
|--------|---------|------|
| **L0-T1** | GPIO 기본 동작 (100회 입출력) | 100% 성공 |
| **L0-T2** | I2C 통신 (센서 읽기) | < 10ms, 100% 정확성 |
| **L0-T3** | SPI 전송 (RF 칩 제어) | < 1ms, 데이터 무결성 100% |
| **L0-T4** | Wi-Fi Direct 스캔 (5회) | < 5s/회, 최소 1개 피어 감지 |
| **L0-T5** | Wi-Fi Direct 연결 (3회) | < 3s/회, 100% 성공 |
| **L0-T6** | LoRa 송수신 (10개 패킷) | < 2000ms/TX, 99%+ RX |

---

## 6. 구현 순서

1. **radio_hal.fl** (400줄) - GPIO/I2C/SPI 기본 레이어
2. **wifi_direct.fl** (450줄) - Wi-Fi 스캔/연결/통신
3. **lora_modem.fl** (400줄) - LoRa 모듈 제어
4. **l0_tests.fl** (600줄) - L0-T1~T6 테스트

**총 1,850줄**

---

## 7. Sovereign-Mesh 완전 통합

```
User Application
       ↓
  mesh_api.fl (L3) ← 사용자 API
       ↓
 kinetic_route.fl (L2) ← 라우팅
 ghost_packet.fl (L1) ← 암호화
       ↓
radio_hal.fl (L0) ← 하드웨어 제어 ← NEW
wifi_direct.fl    ← Wi-Fi 직접 통신
lora_modem.fl     ← LoRa 원거리 통신
       ↓
  물리 계층 (무선 신호)
```

---

**다음**: radio_hal.fl 400줄 구현 시작 🚀
