// Project Sovereign - L4 Intelligence: UserBehaviorModel (700줄)
// 사용자의 시간대별, 위치별, 행동별 습관을 학습하는 핵심 엔진

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// ============================================================================
/// Data Structures: User Habits Model
/// ============================================================================

/// 하루의 시간 (0-23)
pub type Hour = u8;

/// 위치 (GPS 기반 또는 네트워크 기반)
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Location {
    pub latitude: i32,      // 1000배 확대
    pub longitude: i32,
    pub location_type: LocationType,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum LocationType {
    Home,
    Work,
    Cafe,
    Vehicle,
    Unknown,
}

/// 앱 식별자
pub type AppId = u32;

/// 앱 카테고리
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum AppCategory {
    Messaging,          // 카톡, 라인
    Social,             // 인스타, 페북
    Gaming,             // 게임
    Productivity,       // 슬랙, 드라이브
    Entertainment,      // 유튜브, 넷플릭스
    Utility,            // 카메라, 전화
    System,             // 설정, 시스템
    Other,
}

/// 시간대별 사용 프로파일
#[derive(Clone)]
pub struct TimeSlotProfile {
    /// 이 시간대에 가장 자주 사용되는 앱 (상위 5개)
    pub top_apps: Vec<(AppId, f64)>,  // (AppId, probability)

    /// 이 시간대의 평균 배터리 소모율
    pub avg_power_consumption: f64,   // mW

    /// 이 시간대의 평균 화면 밝기 (0-1.0)
    pub avg_brightness: f64,

    /// 화면 ON 확률
    pub screen_on_probability: f64,

    /// CPU 평균 사용률 (0-1.0)
    pub avg_cpu_usage: f64,

    /// 샘플 수 (신뢰도)
    pub sample_count: usize,
}

impl TimeSlotProfile {
    fn new() -> Self {
        TimeSlotProfile {
            top_apps: Vec::new(),
            avg_power_consumption: 50.0,
            avg_brightness: 0.5,
            screen_on_probability: 0.3,
            avg_cpu_usage: 0.2,
            sample_count: 0,
        }
    }
}

/// 위치별 사용 프로파일
#[derive(Clone)]
pub struct LocationProfile {
    /// 이 위치에서 가장 자주 사용되는 앱
    pub common_apps: Vec<(AppId, f64)>,

    /// WiFi 연결 확률
    pub wifi_probability: f64,

    /// 평균 체류 시간 (분)
    pub avg_dwell_time: f64,

    /// 방문 빈도 (주당)
    pub visit_frequency: f64,

    /// 샘플 수
    pub sample_count: usize,
}

impl LocationProfile {
    fn new() -> Self {
        LocationProfile {
            common_apps: Vec::new(),
            wifi_probability: 0.5,
            avg_dwell_time: 60.0,
            visit_frequency: 1.0,
            sample_count: 0,
        }
    }
}

/// 앱별 사용 프로파일
#[derive(Clone)]
pub struct AppProfile {
    pub app_id: AppId,
    pub category: AppCategory,
    pub name: String,

    /// 이 앱이 사용되는 시간대 (확률 분포)
    pub time_distribution: [f64; 24],  // 각 시간대 사용 확률

    /// 이 앱이 주로 사용되는 위치
    pub location_preference: Vec<(LocationType, f64)>,

    /// 평균 사용 시간 (초)
    pub avg_usage_duration: u32,

    /// 하루 평균 실행 횟수
    pub launches_per_day: f64,

    /// 배터리 소모율 (mW)
    pub power_consumption: f64,

    /// 신뢰도 (0-1.0)
    pub confidence: f64,
}

impl AppProfile {
    fn new(app_id: AppId, category: AppCategory, name: String) -> Self {
        AppProfile {
            app_id,
            category,
            name,
            time_distribution: [0.0; 24],
            location_preference: Vec::new(),
            avg_usage_duration: 300,  // 5분 기본값
            launches_per_day: 1.0,
            power_consumption: 50.0,
            confidence: 0.0,
        }
    }
}

/// ============================================================================
/// UserBehaviorModel: 통합 사용자 행동 모델
/// ============================================================================

pub struct UserBehaviorModel {
    /// 시간대별 프로파일 (24개)
    time_profiles: HashMap<Hour, TimeSlotProfile>,

    /// 위치별 프로파일
    location_profiles: HashMap<Location, LocationProfile>,

    /// 앱별 프로파일 (상위 50개)
    app_profiles: HashMap<AppId, AppProfile>,

    /// 요일별 패턴 (0=Monday, 6=Sunday)
    day_patterns: [DayPattern; 7],

    /// 학습 상태
    learning_state: LearningState,

    /// 학습 데이터 누적
    total_samples: usize,
}

#[derive(Clone)]
struct DayPattern {
    /// 요일별 활동 패턴
    activity_level: [f64; 24],  // 0.0-1.0
    avg_screen_time: u32,        // 초
    most_used_apps: Vec<AppId>,
}

#[derive(Clone, Copy, Debug)]
enum LearningState {
    NoData,           // 0 샘플
    Observing,        // 1-500 샘플
    Emerging,         // 500-2000 샘플
    Established,      // 2000-10000 샘플
    Mature,           // 10000+ 샘플
}

impl UserBehaviorModel {
    /// 새로운 모델 생성
    pub fn new() -> Self {
        let mut model = UserBehaviorModel {
            time_profiles: HashMap::new(),
            location_profiles: HashMap::new(),
            app_profiles: HashMap::new(),
            day_patterns: [DayPattern {
                activity_level: [0.0; 24],
                avg_screen_time: 0,
                most_used_apps: Vec::new(),
            }; 7],
            learning_state: LearningState::NoData,
            total_samples: 0,
        };

        // 초기화: 24개 시간대 프로파일
        for hour in 0..24 {
            model.time_profiles.insert(hour, TimeSlotProfile::new());
        }

        model
    }

    /// 사용자 행동 데이터 수집
    pub fn record_event(&mut self, event: UserEvent) {
        let hour = self.get_hour_from_time(event.timestamp);

        // 1. 시간대 프로파일 업데이트
        self.update_time_profile(hour, &event);

        // 2. 위치 프로파일 업데이트
        if let Some(location) = &event.location {
            self.update_location_profile(location.clone(), &event);
        }

        // 3. 앱 프로파일 업데이트
        if let Some(app_id) = event.active_app {
            self.update_app_profile(app_id, &event);
        }

        // 4. 요일 패턴 업데이트
        let day_of_week = self.get_day_of_week(event.timestamp);
        self.update_day_pattern(day_of_week, hour, &event);

        self.total_samples += 1;
        self.update_learning_state();
    }

    fn update_time_profile(&mut self, hour: Hour, event: &UserEvent) {
        let profile = self.time_profiles.entry(hour).or_insert(TimeSlotProfile::new());

        // 누적 평균 계산 (Welford's algorithm)
        let n = profile.sample_count as f64;
        let new_n = n + 1.0;

        profile.avg_power_consumption =
            (profile.avg_power_consumption * n + event.power_consumption) / new_n;
        profile.avg_brightness = (profile.avg_brightness * n + event.brightness) / new_n;
        profile.avg_cpu_usage = (profile.avg_cpu_usage * n + event.cpu_usage) / new_n;

        if event.screen_on {
            profile.screen_on_probability = (profile.screen_on_probability * n + 1.0) / new_n;
        } else {
            profile.screen_on_probability = (profile.screen_on_probability * n) / new_n;
        }

        profile.sample_count += 1;

        // 앱 빈도 업데이트
        if let Some(app_id) = event.active_app {
            let pos = profile.top_apps.iter().position(|(id, _)| *id == app_id);
            match pos {
                Some(idx) => {
                    profile.top_apps[idx].1 = (profile.top_apps[idx].1 * n + 1.0) / new_n;
                }
                None => {
                    if profile.top_apps.len() < 5 {
                        profile.top_apps.push((app_id, 1.0 / new_n));
                    } else {
                        // 낮은 확률의 앱 제거
                        profile.top_apps.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                    }
                }
            }
        }
    }

    fn update_location_profile(&mut self, location: Location, event: &UserEvent) {
        let profile = self.location_profiles.entry(location).or_insert(LocationProfile::new());

        let n = profile.sample_count as f64;
        let new_n = n + 1.0;

        profile.wifi_probability = (profile.wifi_probability * n + if event.wifi_connected { 1.0 } else { 0.0 }) / new_n;
        profile.sample_count += 1;

        if let Some(app_id) = event.active_app {
            let pos = profile.common_apps.iter().position(|(id, _)| *id == app_id);
            match pos {
                Some(idx) => {
                    profile.common_apps[idx].1 = (profile.common_apps[idx].1 * n + 1.0) / new_n;
                }
                None => {
                    if profile.common_apps.len() < 5 {
                        profile.common_apps.push((app_id, 1.0 / new_n));
                    }
                }
            }
        }
    }

    fn update_app_profile(&mut self, app_id: AppId, event: &UserEvent) {
        let profile = self.app_profiles.entry(app_id)
            .or_insert(AppProfile::new(app_id, AppCategory::Other, format!("App_{}", app_id)));

        let hour = self.get_hour_from_time(event.timestamp);
        let n = profile.time_distribution[hour as usize];
        profile.time_distribution[hour as usize] = (n + 1.0) / (n + 2.0);

        // 배터리 소모 업데이트
        let m = profile.launches_per_day;
        profile.power_consumption = (profile.power_consumption * m + event.power_consumption) / (m + 1.0);

        profile.launches_per_day += 0.1;
        profile.confidence = (self.total_samples as f64 / 1000.0).min(1.0);
    }

    fn update_day_pattern(&mut self, day: usize, hour: Hour, event: &UserEvent) {
        if day >= 7 {
            return;
        }

        let pattern = &mut self.day_patterns[day];
        let n = pattern.activity_level[hour as usize];
        pattern.activity_level[hour as usize] = (n + event.cpu_usage) / 2.0;
    }

    fn update_learning_state(&mut self) {
        self.learning_state = match self.total_samples {
            0..=500 => LearningState::Observing,
            501..=2000 => LearningState::Emerging,
            2001..=10000 => LearningState::Established,
            _ => LearningState::Mature,
        };
    }

    /// 다음 앱 예측 (현재 시간, 위치 기반)
    pub fn predict_next_app(&self, hour: Hour, location: Option<&Location>) -> Vec<(AppId, f64)> {
        let mut predictions = Vec::new();

        // 시간대 기반 예측
        if let Some(profile) = self.time_profiles.get(&hour) {
            for (app_id, prob) in &profile.top_apps {
                predictions.push((*app_id, prob * 0.6));  // 60% 가중치
            }
        }

        // 위치 기반 예측
        if let Some(loc) = location {
            if let Some(profile) = self.location_profiles.get(loc) {
                for (app_id, prob) in &profile.common_apps {
                    if let Some(pos) = predictions.iter().position(|(id, _)| *id == app_id) {
                        predictions[pos].1 += prob * 0.4;  // 40% 가중치
                    } else {
                        predictions.push((*app_id, prob * 0.4));
                    }
                }
            }
        }

        predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        predictions.truncate(5);  // 상위 5개만
        predictions
    }

    /// 시간대별 배터리 예상 소모
    pub fn predict_power_consumption(&self, hour: Hour) -> f64 {
        self.time_profiles
            .get(&hour)
            .map(|p| p.avg_power_consumption)
            .unwrap_or(50.0)
    }

    /// 시간대별 화면 켜질 확률
    pub fn predict_screen_on_probability(&self, hour: Hour) -> f64 {
        self.time_profiles
            .get(&hour)
            .map(|p| p.screen_on_probability)
            .unwrap_or(0.3)
    }

    /// 학습 상태 조회
    pub fn learning_state(&self) -> LearningState {
        self.learning_state
    }

    /// 학습 진도 (%)
    pub fn learning_progress(&self) -> f64 {
        (self.total_samples as f64 / 10000.0).min(1.0) * 100.0
    }

    /// 예측 신뢰도 (%)
    pub fn prediction_confidence(&self) -> f64 {
        match self.learning_state {
            LearningState::NoData => 0.0,
            LearningState::Observing => 20.0,
            LearningState::Emerging => 50.0,
            LearningState::Established => 80.0,
            LearningState::Mature => 95.0,
        }
    }

    /// 주간 활동 요약
    pub fn weekly_summary(&self) -> Vec<DayWeeklyStats> {
        self.day_patterns
            .iter()
            .enumerate()
            .map(|(day, pattern)| {
                let day_name = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][day];
                let peak_hour = pattern.activity_level
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                    .map(|(i, _)| i as u8)
                    .unwrap_or(12);

                DayWeeklyStats {
                    day_name: day_name.to_string(),
                    peak_hour,
                    avg_screen_time: pattern.avg_screen_time,
                    most_used_app: pattern.most_used_apps.first().copied(),
                }
            })
            .collect()
    }

    fn get_hour_from_time(&self, timestamp: u64) -> Hour {
        let secs_per_hour = 3600;
        ((timestamp / secs_per_hour) % 24) as u8
    }

    fn get_day_of_week(&self, timestamp: u64) -> usize {
        let secs_per_day = 86400;
        ((timestamp / secs_per_day + 3) % 7) as usize  // +3: Unix epoch was Thursday
    }
}

/// 단일 사용자 이벤트
pub struct UserEvent {
    pub timestamp: u64,              // Unix timestamp
    pub active_app: Option<AppId>,   // 활성 앱
    pub location: Option<Location>,  // 현재 위치
    pub power_consumption: f64,      // 현재 전력 (mW)
    pub brightness: f64,             // 화면 밝기 (0-1.0)
    pub cpu_usage: f64,              // CPU 사용률 (0-1.0)
    pub screen_on: bool,             // 화면 상태
    pub wifi_connected: bool,        // WiFi 연결 상태
    pub temperature: f64,            // 온도 (°C)
}

/// 주간 통계
pub struct DayWeeklyStats {
    pub day_name: String,
    pub peak_hour: Hour,
    pub avg_screen_time: u32,
    pub most_used_app: Option<AppId>,
}

/// ============================================================================
/// Tests
/// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_creation() {
        let model = UserBehaviorModel::new();
        assert_eq!(model.total_samples, 0);
        assert!(matches!(model.learning_state, LearningState::NoData));
    }

    #[test]
    fn test_record_single_event() {
        let mut model = UserBehaviorModel::new();
        let event = UserEvent {
            timestamp: 1000,
            active_app: Some(1),
            location: None,
            power_consumption: 80.0,
            brightness: 0.7,
            cpu_usage: 0.5,
            screen_on: true,
            wifi_connected: false,
            temperature: 35.0,
        };
        model.record_event(event);
        assert_eq!(model.total_samples, 1);
    }

    #[test]
    fn test_learning_state_progression() {
        let mut model = UserBehaviorModel::new();
        for i in 0..600 {
            model.record_event(UserEvent {
                timestamp: (i * 100) as u64,
                active_app: Some((i % 10) as u32),
                location: None,
                power_consumption: 50.0 + (i % 50) as f64,
                brightness: 0.5,
                cpu_usage: 0.3,
                screen_on: true,
                wifi_connected: false,
                temperature: 35.0,
            });
        }
        assert!(!matches!(model.learning_state, LearningState::NoData | LearningState::Observing));
    }

    #[test]
    fn test_time_profile_accuracy() {
        let mut model = UserBehaviorModel::new();

        // 09:00 (hour 9)에만 데이터 기록
        for i in 0..20 {
            let timestamp = (9 * 3600 + 100 * i) as u64;
            model.record_event(UserEvent {
                timestamp,
                active_app: Some(1),
                location: None,
                power_consumption: 100.0,
                brightness: 0.8,
                cpu_usage: 0.6,
                screen_on: true,
                wifi_connected: true,
                temperature: 35.0,
            });
        }

        let profile = model.time_profiles.get(&9).unwrap();
        assert!(profile.sample_count > 0);
        assert!(profile.avg_power_consumption > 0.0);
    }

    #[test]
    fn test_next_app_prediction() {
        let mut model = UserBehaviorModel::new();

        // 10:00에 App 1을 자주 사용
        for i in 0..10 {
            model.record_event(UserEvent {
                timestamp: (10 * 3600 + 100 * i) as u64,
                active_app: Some(1),
                location: None,
                power_consumption: 50.0,
                brightness: 0.5,
                cpu_usage: 0.3,
                screen_on: true,
                wifi_connected: false,
                temperature: 35.0,
            });
        }

        let predictions = model.predict_next_app(10, None);
        assert!(!predictions.is_empty());
        // App 1이 가장 높은 확률
        if let Some((app_id, prob)) = predictions.first() {
            assert_eq!(*app_id, 1);
            assert!(*prob > 0.0);
        }
    }

    #[test]
    fn test_power_consumption_prediction() {
        let mut model = UserBehaviorModel::new();

        // 15:00에 높은 전력 소모 기록
        for i in 0..20 {
            model.record_event(UserEvent {
                timestamp: (15 * 3600 + 100 * i) as u64,
                active_app: Some(1),
                location: None,
                power_consumption: 150.0,  // 높은 전력
                brightness: 1.0,
                cpu_usage: 0.9,
                screen_on: true,
                wifi_connected: false,
                temperature: 40.0,
            });
        }

        let power = model.predict_power_consumption(15);
        assert!(power > 100.0);  // 150에 가까워야 함
    }

    #[test]
    fn test_location_profile() {
        let mut model = UserBehaviorModel::new();
        let location = Location {
            latitude: 37000,
            longitude: 127000,
            location_type: LocationType::Home,
        };

        for i in 0..10 {
            model.record_event(UserEvent {
                timestamp: (i * 1000) as u64,
                active_app: Some(2),
                location: Some(location.clone()),
                power_consumption: 30.0,
                brightness: 0.4,
                cpu_usage: 0.2,
                screen_on: true,
                wifi_connected: true,
                temperature: 32.0,
            });
        }

        assert!(model.location_profiles.contains_key(&location));
        let profile = model.location_profiles.get(&location).unwrap();
        assert!(profile.wifi_probability > 0.5);  // WiFi가 연결됐으므로
    }

    #[test]
    fn test_learning_progress() {
        let mut model = UserBehaviorModel::new();
        assert_eq!(model.learning_progress(), 0.0);

        for i in 0..1000 {
            model.record_event(UserEvent {
                timestamp: (i * 100) as u64,
                active_app: Some((i % 20) as u32),
                location: None,
                power_consumption: 50.0,
                brightness: 0.5,
                cpu_usage: 0.3,
                screen_on: true,
                wifi_connected: false,
                temperature: 35.0,
            });
        }

        assert!(model.learning_progress() >= 10.0);
    }

    #[test]
    fn test_prediction_confidence_growth() {
        let mut model = UserBehaviorModel::new();
        let confidence_start = model.prediction_confidence();

        for i in 0..5000 {
            model.record_event(UserEvent {
                timestamp: (i * 100) as u64,
                active_app: Some((i % 30) as u32),
                location: None,
                power_consumption: 50.0,
                brightness: 0.5,
                cpu_usage: 0.3,
                screen_on: true,
                wifi_connected: false,
                temperature: 35.0,
            });
        }

        let confidence_end = model.prediction_confidence();
        assert!(confidence_end > confidence_start);
    }

    #[test]
    fn test_weekly_summary() {
        let mut model = UserBehaviorModel::new();
        let summary = model.weekly_summary();
        assert_eq!(summary.len(), 7);
    }

    #[test]
    fn test_screen_on_probability() {
        let mut model = UserBehaviorModel::new();

        // 09:00에 화면 항상 켬
        for i in 0..20 {
            model.record_event(UserEvent {
                timestamp: (9 * 3600 + 100 * i) as u64,
                active_app: Some(1),
                location: None,
                power_consumption: 80.0,
                brightness: 0.8,
                cpu_usage: 0.5,
                screen_on: true,
                wifi_connected: false,
                temperature: 35.0,
            });
        }

        let prob = model.predict_screen_on_probability(9);
        assert!(prob > 0.8);  // 거의 항상 켜짐
    }

    #[test]
    fn test_multiple_apps_same_time() {
        let mut model = UserBehaviorModel::new();

        // 14:00에 여러 앱을 번갈아 사용
        for i in 0..30 {
            model.record_event(UserEvent {
                timestamp: (14 * 3600 + 100 * i) as u64,
                active_app: Some((i % 3) as u32 + 1),  // App 1, 2, 3
                location: None,
                power_consumption: 70.0,
                brightness: 0.6,
                cpu_usage: 0.4,
                screen_on: true,
                wifi_connected: false,
                temperature: 36.0,
            });
        }

        let predictions = model.predict_next_app(14, None);
        assert!(predictions.len() >= 2);
    }

    #[test]
    fn test_emerging_vs_mature() {
        let mut model = UserBehaviorModel::new();

        // Emerging 단계 (500-2000)
        for i in 0..700 {
            model.record_event(UserEvent {
                timestamp: (i * 100) as u64,
                active_app: Some((i % 10) as u32),
                location: None,
                power_consumption: 50.0,
                brightness: 0.5,
                cpu_usage: 0.3,
                screen_on: true,
                wifi_connected: false,
                temperature: 35.0,
            });
        }

        assert!(matches!(model.learning_state, LearningState::Emerging));
        let confidence_emerging = model.prediction_confidence();

        // Mature 단계 (10000+)
        for i in 700..12000 {
            model.record_event(UserEvent {
                timestamp: (i * 100) as u64,
                active_app: Some((i % 20) as u32),
                location: None,
                power_consumption: 50.0,
                brightness: 0.5,
                cpu_usage: 0.3,
                screen_on: true,
                wifi_connected: false,
                temperature: 35.0,
            });
        }

        assert!(matches!(model.learning_state, LearningState::Mature));
        let confidence_mature = model.prediction_confidence();
        assert!(confidence_mature > confidence_emerging);
    }
}
