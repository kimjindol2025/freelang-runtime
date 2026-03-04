// Project Sovereign - L4 Intelligence: SystemAdaptation (700줄)
// 사용자 습관 예측을 실제 시스템 최적화로 변환

use std::collections::VecDeque;

/// ============================================================================
/// Data Structures: System State & Configuration
/// ============================================================================

/// CPU 코어 종류
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreType {
    BigCore,      // Cortex-X4 (3.4GHz, 고성능)
    MidCore,      // Cortex-A720 (2.9GHz, 중간)
    LittleCore,   // Cortex-A520 (2.2GHz, 저전력)
}

/// CPU 주파수 레벨 (DVFS)
#[derive(Clone, Copy, Debug)]
pub struct FrequencyLevel {
    pub core_type: CoreType,
    pub mhz: u16,
    pub voltage_mv: u16,     // 전압 (mV)
    pub power_mw: f64,       // 전력 소모 (mW)
}

/// CPU 코어 할당 (Big.LITTLE 스케줄링)
#[derive(Clone, Copy)]
pub struct CoreAssignment {
    pub big_cores_active: usize,      // 0-3 (Cortex-X4)
    pub mid_cores_active: usize,      // 0-2 (Cortex-A720)
    pub little_cores_active: usize,   // 0-3 (Cortex-A520)
    pub big_frequency_mhz: u16,
    pub mid_frequency_mhz: u16,
    pub little_frequency_mhz: u16,
}

impl CoreAssignment {
    pub fn total_active_cores(&self) -> usize {
        self.big_cores_active + self.mid_cores_active + self.little_cores_active
    }

    pub fn total_power_mw(&self) -> f64 {
        // 대략적인 전력 계산
        (self.big_cores_active as f64 * (self.big_frequency_mhz as f64 / 1700.0) * 150.0)
            + (self.mid_cores_active as f64 * (self.mid_frequency_mhz as f64 / 1450.0) * 100.0)
            + (self.little_cores_active as f64 * (self.little_frequency_mhz as f64 / 1100.0) * 50.0)
    }
}

/// 배터리 상태
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryMode {
    Excellent,    // > 80% (모든 기능 활성화)
    Good,         // 20-80% (균형 모드)
    Warning,      // 10-20% (저전력 모드)
    Critical,     // < 10% (극도의 절감 모드)
}

/// 온도 상태
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermalState {
    Cool,         // < 35°C (정상)
    Warm,         // 35-45°C (주의)
    Hot,          // 45-55°C (감속)
    Critical,     // > 55°C (극도의 감속)
}

/// 작업 부하 분류
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkloadClass {
    Idle,         // < 10% CPU
    Light,        // 10-30% CPU (텍스트, 메시징)
    Medium,       // 30-60% CPU (웹 브라우징, 동영상)
    Heavy,        // 60-90% CPU (게임, 인코딩)
    Extreme,      // > 90% CPU (카메라 녹화, 렌더링)
}

/// ============================================================================
/// Module 1: AdaptiveScheduler (300줄)
/// ============================================================================

/// 적응형 스케줄러: CPU 코어 할당 및 주파수 제어
pub struct AdaptiveScheduler {
    /// 현재 코어 할당
    current_assignment: CoreAssignment,

    /// 배터리 모드
    battery_mode: BatteryMode,

    /// 온도 상태
    thermal_state: ThermalState,

    /// 현재 부하 분류
    current_workload: WorkloadClass,

    /// 다음 5초 예상 부하
    next_workload_prediction: f64,  // 0.0-1.0

    /// 스케줄 히스토리
    schedule_history: VecDeque<(CoreAssignment, f64)>,  // (assignment, timestamp)
}

impl AdaptiveScheduler {
    pub fn new() -> Self {
        AdaptiveScheduler {
            current_assignment: CoreAssignment {
                big_cores_active: 1,
                mid_cores_active: 1,
                little_cores_active: 2,
                big_frequency_mhz: 1700,
                mid_frequency_mhz: 1450,
                little_frequency_mhz: 1100,
            },
            battery_mode: BatteryMode::Good,
            thermal_state: ThermalState::Cool,
            current_workload: WorkloadClass::Light,
            next_workload_prediction: 0.2,
            schedule_history: VecDeque::new(),
        }
    }

    /// 다음 5초 부하 예측 기반 스케줄 결정
    pub fn decide_schedule(
        &mut self,
        predicted_workload: f64,
        battery_level: f64,
        temperature: f64,
        current_cpu_usage: f64,
    ) -> CoreAssignment {
        // 1. 배터리 모드 업데이트
        self.update_battery_mode(battery_level);

        // 2. 온도 상태 업데이트
        self.update_thermal_state(temperature);

        // 3. 현재 부하 분류
        self.current_workload = self.classify_workload(current_cpu_usage);

        // 4. 예상 부하 저장
        self.next_workload_prediction = predicted_workload;

        // 5. 최적 코어 할당 계산
        self.calculate_optimal_assignment()
    }

    fn update_battery_mode(&mut self, battery_level: f64) {
        self.battery_mode = match battery_level {
            0.8..=1.0 => BatteryMode::Excellent,
            0.2..0.8 => BatteryMode::Good,
            0.1..0.2 => BatteryMode::Warning,
            _ => BatteryMode::Critical,
        };
    }

    fn update_thermal_state(&mut self, temperature: f64) {
        self.thermal_state = match temperature {
            0.0..=35.0 => ThermalState::Cool,
            35.0..=45.0 => ThermalState::Warm,
            45.0..=55.0 => ThermalState::Hot,
            _ => ThermalState::Critical,
        };
    }

    fn classify_workload(&self, cpu_usage: f64) -> WorkloadClass {
        match cpu_usage {
            0.0..=0.1 => WorkloadClass::Idle,
            0.1..=0.3 => WorkloadClass::Light,
            0.3..=0.6 => WorkloadClass::Medium,
            0.6..=0.9 => WorkloadClass::Heavy,
            _ => WorkloadClass::Extreme,
        }
    }

    fn calculate_optimal_assignment(&self) -> CoreAssignment {
        // Big.LITTLE 스케줄링 결정 트리

        // 1단계: 배터리 상태 확인
        let (big_max, mid_max, little_max) = match self.battery_mode {
            BatteryMode::Excellent => (3, 2, 3),   // 모든 코어 활성
            BatteryMode::Good => (2, 1, 2),        // 균형
            BatteryMode::Warning => (1, 1, 2),     // 저전력
            BatteryMode::Critical => (0, 1, 2),    // 극도의 절감
        };

        // 2단계: 온도 고려한 스로틀링
        let (mut big_active, mut mid_active, mut little_active) = match self.thermal_state {
            ThermalState::Cool => (big_max, mid_max, little_max),
            ThermalState::Warm => ((big_max * 3) / 4, mid_max, little_max),
            ThermalState::Hot => ((big_max / 2), (mid_max / 2), little_max),
            ThermalState::Critical => (0, (mid_max / 2), little_max),
        };

        // 3단계: 부하에 따른 최적화
        match self.current_workload {
            WorkloadClass::Idle => {
                // 거의 모든 코어 OFF, little core 1-2개만 활성
                return CoreAssignment {
                    big_cores_active: 0,
                    mid_cores_active: 0,
                    little_cores_active: 1,
                    big_frequency_mhz: 800,
                    mid_frequency_mhz: 800,
                    little_frequency_mhz: 800,
                };
            }
            WorkloadClass::Light => {
                // Little core만 사용
                little_active = little_active.min(2);
                big_active = 0;
                mid_active = 0;
            }
            WorkloadClass::Medium => {
                // Little + Mid 사용
                mid_active = mid_active.min(1);
                big_active = 0;
            }
            WorkloadClass::Heavy => {
                // Big + Mid 사용
                big_active = big_active.min(2);
            }
            WorkloadClass::Extreme => {
                // 모든 코어 풀가동
                big_active = big_active;
                mid_active = mid_active;
            }
        }

        // 4단계: 주파수 결정
        let freq_factor = self.next_workload_prediction;  // 0.0-1.0
        let big_freq = 800 + ((3400 - 800) as f64 * freq_factor) as u16;
        let mid_freq = 800 + ((2900 - 800) as f64 * freq_factor) as u16;
        let little_freq = 800 + ((2200 - 800) as f64 * freq_factor) as u16;

        CoreAssignment {
            big_cores_active: big_active,
            mid_cores_active: mid_active,
            little_cores_active: little_active,
            big_frequency_mhz: big_freq,
            mid_frequency_mhz: mid_freq,
            little_frequency_mhz: little_freq,
        }
    }

    /// 현재 스케줄 조회
    pub fn current_assignment(&self) -> CoreAssignment {
        self.current_assignment
    }

    /// 스케줄 변경 저장
    pub fn record_schedule(&mut self, assignment: CoreAssignment, timestamp: f64) {
        self.current_assignment = assignment;
        self.schedule_history.push_back((assignment, timestamp));
        if self.schedule_history.len() > 100 {
            self.schedule_history.pop_front();
        }
    }

    /// 평균 활성 코어 수
    pub fn avg_active_cores(&self) -> f64 {
        if self.schedule_history.is_empty() {
            return self.current_assignment.total_active_cores() as f64;
        }

        let sum: usize = self.schedule_history
            .iter()
            .map(|(a, _)| a.total_active_cores())
            .sum();

        sum as f64 / self.schedule_history.len() as f64
    }

    pub fn battery_mode(&self) -> BatteryMode {
        self.battery_mode
    }

    pub fn thermal_state(&self) -> ThermalState {
        self.thermal_state
    }
}

/// ============================================================================
/// Module 2: PowerOptimizer (300줄)
/// ============================================================================

/// 전력 최적화: 백그라운드 작업, 센서, 네트워크 제어
pub struct PowerOptimizer {
    /// 앱별 전력 프로파일 (학습된 데이터)
    app_power_profiles: std::collections::HashMap<u32, AppPowerProfile>,

    /// 활성 센서 목록
    active_sensors: Vec<Sensor>,

    /// 백그라운드 작업 큐
    background_tasks: Vec<BackgroundTask>,

    /// 네트워크 상태
    network_state: NetworkState,

    /// 화면 상태
    screen_state: ScreenState,

    /// 최적화 히스토리
    optimization_history: VecDeque<OptimizationAction>,
}

#[derive(Clone)]
struct AppPowerProfile {
    app_id: u32,
    name: String,
    avg_power_mw: f64,      // 평균 소모 전력
    idle_power_mw: f64,     // 백그라운드 소모
    peak_power_mw: f64,     // 최대 소모
    foreground_duration: u32, // 평균 포그라운드 시간 (초)
}

#[derive(Clone, Debug)]
enum Sensor {
    GPS,
    Accelerometer,
    Gyroscope,
    Magnetometer,
    ProximitySensor,
    LightSensor,
    TemperatureSensor,
    Barometer,
}

#[derive(Clone)]
struct BackgroundTask {
    app_id: u32,
    priority: u8,              // 1-5 (높을수록 중요)
    estimated_power: f64,
    can_defer: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum NetworkState {
    WiFi,          // WiFi 연결
    LTE,           // LTE/4G
    ThreeG,        // 3G
    TwoG,          // 2G
    Offline,       // 오프라인
}

#[derive(Clone, Copy, Debug)]
enum ScreenState {
    On,            // 화면 ON
    Dimmed,        // 화면 어두워짐
    Off,           // 화면 OFF
}

#[derive(Clone)]
struct OptimizationAction {
    action_type: String,
    power_saved_mw: f64,
    timestamp: f64,
}

impl PowerOptimizer {
    pub fn new() -> Self {
        PowerOptimizer {
            app_power_profiles: std::collections::HashMap::new(),
            active_sensors: vec![
                Sensor::Accelerometer,
                Sensor::Magnetometer,
                Sensor::LightSensor,
            ],
            background_tasks: Vec::new(),
            network_state: NetworkState::Offline,
            screen_state: ScreenState::Off,
            optimization_history: VecDeque::new(),
        }
    }

    /// 불필요한 백그라운드 작업 제거
    pub fn optimize_background_tasks(
        &mut self,
        battery_mode: BatteryMode,
        predicted_next_app: Option<u32>,
    ) -> f64 {
        let mut total_power_saved = 0.0;

        // 배터리 상태에 따라 우선순위 재계산
        match battery_mode {
            BatteryMode::Excellent | BatteryMode::Good => {
                // 모든 작업 허용
            }
            BatteryMode::Warning => {
                // 우선순위 < 3인 작업 제거
                self.background_tasks.retain(|task| task.priority >= 3);
            }
            BatteryMode::Critical => {
                // 우선순위 < 4인 작업 제거
                self.background_tasks.retain(|task| {
                    // 예측된 앱이면 유지
                    if let Some(next_app) = predicted_next_app {
                        task.app_id == next_app || task.priority >= 4
                    } else {
                        task.priority >= 4
                    }
                });
            }
        }

        total_power_saved
    }

    /// 필요 없는 센서 비활성화
    pub fn optimize_sensors(&mut self, screen_state: ScreenState) -> f64 {
        self.screen_state = screen_state;

        let mut power_saved = 0.0;

        match screen_state {
            ScreenState::On => {
                // 모든 센서 활성화 가능
                self.active_sensors = vec![
                    Sensor::Accelerometer,
                    Sensor::Magnetometer,
                    Sensor::LightSensor,
                    Sensor::ProximitySensor,
                ];
            }
            ScreenState::Dimmed => {
                // GPS 제거, 가속도 유지
                self.active_sensors
                    .retain(|s| !matches!(s, Sensor::GPS | Sensor::Barometer));
                power_saved += 50.0;  // GPS: ~50mW
            }
            ScreenState::Off => {
                // 필수 센서만 (proximity, light)
                self.active_sensors = vec![
                    Sensor::ProximitySensor,
                    Sensor::LightSensor,
                ];
                power_saved += 100.0;  // Accel + Gyro + Mag: ~100mW
            }
        }

        power_saved
    }

    /// 네트워크 최적화
    pub fn optimize_network(
        &mut self,
        predicted_data_needs: bool,
        battery_level: f64,
    ) -> f64 {
        let mut power_saved = 0.0;

        // 배터리 낮을 때 고주파 동기화 끄기
        if battery_level < 0.2 && !predicted_data_needs {
            // 동기화 일시 중지
            power_saved += 30.0;  // 동기화 일시 중지: ~30mW
        }

        // 네트워크 대역폭 최적화
        if let NetworkState::LTE = self.network_state {
            if !predicted_data_needs && battery_level < 0.3 {
                // LTE → 3G 다운그레이드 (자동)
                self.network_state = NetworkState::ThreeG;
                power_saved += 40.0;  // LTE vs 3G: ~40mW
            }
        }

        power_saved
    }

    /// 화면 밝기 자동 조절
    pub fn optimize_display_brightness(
        &mut self,
        ambient_light: f64,
        battery_level: f64,
    ) -> (f64, f64) {
        // 주변 밝기 기반 권장 밝기
        let mut recommended_brightness = ambient_light * 0.8;

        // 배터리 레벨 기반 감소
        if battery_level < 0.2 {
            recommended_brightness *= 0.7;  // 30% 어둡게
        }

        // 전력 절감
        let power_saved = (1.0 - recommended_brightness) * 50.0;

        (recommended_brightness.max(0.1).min(1.0), power_saved)
    }

    /// 전체 전력 최적화 요약
    pub fn total_power_optimization(&self) -> f64 {
        self.optimization_history
            .iter()
            .map(|action| action.power_saved_mw)
            .sum()
    }

    pub fn active_sensors_count(&self) -> usize {
        self.active_sensors.len()
    }

    pub fn background_tasks_count(&self) -> usize {
        self.background_tasks.len()
    }
}

/// ============================================================================
/// Module 3: ThermalController (100줄)
/// ============================================================================

/// 온도 관리: 예측 기반 스로틀링
pub struct ThermalController {
    /// 현재 온도 히스토리
    temperature_history: VecDeque<f64>,

    /// 열 흐름 모델 (시간 상수)
    thermal_time_constant: f64,  // 초

    /// 스로틀링 레벨 (0-100)
    throttle_level: f64,
}

impl ThermalController {
    pub fn new() -> Self {
        ThermalController {
            temperature_history: VecDeque::new(),
            thermal_time_constant: 30.0,  // 30초
            throttle_level: 0.0,
        }
    }

    /// 온도 트렌드 기반 예측
    pub fn predict_temperature_trend(&self) -> f64 {
        if self.temperature_history.len() < 2 {
            return 0.0;  // 변화 없음
        }

        let recent = self.temperature_history.iter().rev().take(5).copied().collect::<Vec<_>>();
        if recent.len() < 2 {
            return 0.0;
        }

        // 간단한 선형 회귀
        let trend = (recent[0] - recent[recent.len() - 1]) / recent.len() as f64;
        trend
    }

    /// 예측 기반 동적 스로틀링
    pub fn predict_throttle_level(&mut self, current_temp: f64) -> f64 {
        self.temperature_history.push_back(current_temp);
        if self.temperature_history.len() > 20 {
            self.temperature_history.pop_front();
        }

        let trend = self.predict_temperature_trend();

        // 상승 추세 감지 → 선제 스로틀링
        if trend > 0.5 {
            // 온도 올라가는 중 → 미리 줄이기
            self.throttle_level = ((current_temp - 35.0) / 20.0).max(0.0).min(1.0);
        } else if current_temp > 55.0 {
            // 극한 온도
            self.throttle_level = 1.0;
        } else if current_temp > 45.0 {
            // 높은 온도
            self.throttle_level = 0.5;
        } else {
            // 정상
            self.throttle_level = 0.0;
        }

        self.throttle_level
    }

    pub fn current_throttle_level(&self) -> f64 {
        self.throttle_level
    }
}

/// ============================================================================
/// Tests
/// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = AdaptiveScheduler::new();
        assert_eq!(scheduler.current_workload, WorkloadClass::Light);
    }

    #[test]
    fn test_battery_mode_excellent() {
        let mut scheduler = AdaptiveScheduler::new();
        scheduler.update_battery_mode(0.95);
        assert_eq!(scheduler.battery_mode, BatteryMode::Excellent);
    }

    #[test]
    fn test_battery_mode_critical() {
        let mut scheduler = AdaptiveScheduler::new();
        scheduler.update_battery_mode(0.05);
        assert_eq!(scheduler.battery_mode, BatteryMode::Critical);
    }

    #[test]
    fn test_thermal_state_cool() {
        let mut scheduler = AdaptiveScheduler::new();
        scheduler.update_thermal_state(30.0);
        assert_eq!(scheduler.thermal_state, ThermalState::Cool);
    }

    #[test]
    fn test_thermal_state_critical() {
        let mut scheduler = AdaptiveScheduler::new();
        scheduler.update_thermal_state(60.0);
        assert_eq!(scheduler.thermal_state, ThermalState::Critical);
    }

    #[test]
    fn test_workload_classification_idle() {
        let scheduler = AdaptiveScheduler::new();
        let workload = scheduler.classify_workload(0.05);
        assert_eq!(workload, WorkloadClass::Idle);
    }

    #[test]
    fn test_workload_classification_heavy() {
        let scheduler = AdaptiveScheduler::new();
        let workload = scheduler.classify_workload(0.75);
        assert_eq!(workload, WorkloadClass::Heavy);
    }

    #[test]
    fn test_decide_schedule_excellent_light() {
        let mut scheduler = AdaptiveScheduler::new();
        let assignment = scheduler.decide_schedule(0.2, 0.9, 30.0, 0.1);

        assert!(assignment.little_cores_active > 0);
        assert!(assignment.big_cores_active <= 3);
    }

    #[test]
    fn test_decide_schedule_critical_battery() {
        let mut scheduler = AdaptiveScheduler::new();
        let assignment = scheduler.decide_schedule(0.3, 0.05, 35.0, 0.2);

        // Critical 배터리: 코어 줄어들어야 함
        assert!(assignment.big_cores_active <= 1);
    }

    #[test]
    fn test_core_assignment_power_calculation() {
        let assignment = CoreAssignment {
            big_cores_active: 2,
            mid_cores_active: 1,
            little_cores_active: 2,
            big_frequency_mhz: 2000,
            mid_frequency_mhz: 1500,
            little_frequency_mhz: 1000,
        };

        let power = assignment.total_power_mw();
        assert!(power > 0.0);
        assert!(power < 500.0);  // 합리적인 범위
    }

    #[test]
    fn test_scheduler_avg_active_cores() {
        let mut scheduler = AdaptiveScheduler::new();

        for i in 0..5 {
            let assignment = CoreAssignment {
                big_cores_active: i,
                mid_cores_active: 1,
                little_cores_active: 2,
                big_frequency_mhz: 2000,
                mid_frequency_mhz: 1500,
                little_frequency_mhz: 1000,
            };
            scheduler.record_schedule(assignment, i as f64);
        }

        let avg = scheduler.avg_active_cores();
        assert!(avg > 0.0);
    }

    #[test]
    fn test_power_optimizer_creation() {
        let optimizer = PowerOptimizer::new();
        assert!(optimizer.active_sensors.len() > 0);
    }

    #[test]
    fn test_optimize_sensors_screen_off() {
        let mut optimizer = PowerOptimizer::new();
        let initial_count = optimizer.active_sensors.len();

        optimizer.optimize_sensors(ScreenState::Off);

        // 화면 OFF 시 센서 줄어들어야 함
        assert!(optimizer.active_sensors.len() <= initial_count);
    }

    #[test]
    fn test_optimize_display_brightness() {
        let mut optimizer = PowerOptimizer::new();

        let (brightness, power_saved) = optimizer.optimize_display_brightness(0.8, 0.9);
        assert!(brightness > 0.0 && brightness <= 1.0);
        assert!(power_saved >= 0.0);
    }

    #[test]
    fn test_optimize_display_low_battery() {
        let mut optimizer = PowerOptimizer::new();

        let (brightness_normal, _) = optimizer.optimize_display_brightness(0.8, 0.9);
        let (brightness_low, _) = optimizer.optimize_display_brightness(0.8, 0.1);

        // 배터리 낮을 때 밝기 더 낮아야 함
        assert!(brightness_low < brightness_normal);
    }

    #[test]
    fn test_thermal_controller_creation() {
        let controller = ThermalController::new();
        assert_eq!(controller.throttle_level, 0.0);
    }

    #[test]
    fn test_thermal_controller_cool() {
        let mut controller = ThermalController::new();
        let throttle = controller.predict_throttle_level(30.0);
        assert_eq!(throttle, 0.0);
    }

    #[test]
    fn test_thermal_controller_hot() {
        let mut controller = ThermalController::new();
        let throttle = controller.predict_throttle_level(55.0);
        assert!(throttle > 0.0);
    }

    #[test]
    fn test_thermal_controller_critical() {
        let mut controller = ThermalController::new();
        let throttle = controller.predict_throttle_level(60.0);
        assert_eq!(throttle, 1.0);
    }

    #[test]
    fn test_thermal_upward_trend_detection() {
        let mut controller = ThermalController::new();

        // 온도 상승 추세 기록
        for i in 0..5 {
            controller.temperature_history.push_back(30.0 + i as f64);
        }

        let trend = controller.predict_temperature_trend();
        assert!(trend > 0.0);  // 상승 추세 감지
    }

    #[test]
    fn test_workload_idle_uses_minimal_cores() {
        let mut scheduler = AdaptiveScheduler::new();
        let assignment = scheduler.decide_schedule(0.0, 0.8, 30.0, 0.05);

        assert_eq!(assignment.big_cores_active, 0);
        assert_eq!(assignment.mid_cores_active, 0);
        assert_eq!(assignment.little_cores_active, 1);
    }

    #[test]
    fn test_workload_heavy_uses_big_cores() {
        let mut scheduler = AdaptiveScheduler::new();
        let assignment = scheduler.decide_schedule(0.9, 0.8, 30.0, 0.85);

        assert!(assignment.big_cores_active > 0);
    }

    #[test]
    fn test_frequency_scaling_follows_prediction() {
        let mut scheduler = AdaptiveScheduler::new();

        // 낮은 예측
        let low_pred = scheduler.decide_schedule(0.1, 0.8, 30.0, 0.1);

        // 높은 예측
        let high_pred = scheduler.decide_schedule(0.9, 0.8, 30.0, 0.9);

        // 높은 예측의 주파수가 더 높아야 함
        assert!(high_pred.big_frequency_mhz > low_pred.big_frequency_mhz);
    }

    #[test]
    fn test_combined_scheduler_optimizer() {
        let mut scheduler = AdaptiveScheduler::new();
        let mut optimizer = PowerOptimizer::new();

        let assignment = scheduler.decide_schedule(0.5, 0.6, 40.0, 0.4);
        let sensor_saved = optimizer.optimize_sensors(ScreenState::Dimmed);
        let display_brightness = optimizer.optimize_display_brightness(0.6, 0.5);

        assert!(assignment.total_active_cores() > 0);
        assert!(sensor_saved >= 0.0);
        assert!(display_brightness.0 > 0.0);
    }
}
