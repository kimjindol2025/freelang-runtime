// Project Sovereign: Thermal Management Module
// Goal: Monitor 6 thermal zones and apply predictive throttling
// Target: Maintain temperature within safe operating range (<50°C normal, <55°C limit)

use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermalZone {
    SoC,          // System-on-Chip (CPU/GPU main)
    CPUCluster0,  // Performance cores
    CPUCluster1,  // Efficiency cores
    GPU,          // Graphics processor
    Battery,      // Battery pack
    Modem,        // Cellular modem
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ThermalState {
    Cool,        // <35°C (normal operation)
    Warm,        // 35-40°C (monitor)
    Hot,         // 40-50°C (prepare throttle)
    Critical,    // 50-55°C (aggressive throttle)
    Emergency,   // >55°C (emergency shutdown)
}

#[derive(Clone, Debug)]
pub struct ThermalZoneMonitor {
    zone: ThermalZone,
    current_temp: f64,
    max_safe_temp: f64,
    thermal_state: ThermalState,
    history: VecDeque<(u64, f64)>,  // timestamp, temperature
    trend: f64,  // °C per second
    throttle_level: f64,  // 0.0-1.0
}

impl ThermalZoneMonitor {
    pub fn new(zone: ThermalZone) -> Self {
        let max_safe_temp = match zone {
            ThermalZone::SoC => 50.0,
            ThermalZone::CPUCluster0 => 55.0,
            ThermalZone::CPUCluster1 => 50.0,
            ThermalZone::GPU => 52.0,
            ThermalZone::Battery => 45.0,
            ThermalZone::Modem => 48.0,
        };

        Self {
            zone,
            current_temp: 25.0,
            max_safe_temp,
            thermal_state: ThermalState::Cool,
            history: VecDeque::with_capacity(60),
            trend: 0.0,
            throttle_level: 0.0,
        }
    }

    pub fn update_temperature(&mut self, temp: f64, timestamp: u64) {
        // Record previous for trend calculation
        let prev_temp = self.current_temp;
        self.current_temp = temp;

        // Update history
        self.history.push_back((timestamp, temp));
        if self.history.len() > 60 {
            self.history.pop_front();
        }

        // Calculate trend
        if let Some((prev_ts, _)) = self.history.get(0) {
            let time_delta = timestamp.saturating_sub(*prev_ts) as f64 / 1000.0;
            if time_delta > 0.0 {
                self.trend = (temp - prev_temp) / time_delta;
            }
        }

        // Update thermal state
        self.update_thermal_state();

        // Calculate throttle level
        self.calculate_throttle_level();
    }

    fn update_thermal_state(&mut self) {
        self.thermal_state = match self.current_temp {
            t if t < 35.0 => ThermalState::Cool,
            t if t < 40.0 => ThermalState::Warm,
            t if t < 50.0 => ThermalState::Hot,
            t if t < 55.0 => ThermalState::Critical,
            _ => ThermalState::Emergency,
        };
    }

    fn calculate_throttle_level(&mut self) {
        // Throttle level increases as temperature approaches max
        let safety_margin = self.max_safe_temp - self.current_temp;
        let range = 15.0;  // Temperature range over which to throttle (55-40 = 15)

        self.throttle_level = if safety_margin <= 0.0 {
            1.0  // Full throttle
        } else if safety_margin >= range {
            0.0  // No throttle
        } else {
            // Linear scaling: 0 at range, 1 at 0
            1.0 - (safety_margin / range)
        };

        self.throttle_level = self.throttle_level.max(0.0).min(1.0);
    }

    pub fn get_throttle_level(&self) -> f64 {
        self.throttle_level
    }

    pub fn get_trend(&self) -> f64 {
        self.trend
    }

    pub fn is_heating_up(&self) -> bool {
        self.trend > 0.5  // Rising >0.5°C/sec
    }

    pub fn is_cooling_down(&self) -> bool {
        self.trend < -0.5  // Falling <-0.5°C/sec
    }

    pub fn needs_immediate_action(&self) -> bool {
        self.thermal_state == ThermalState::Emergency
            || (self.thermal_state == ThermalState::Critical && self.is_heating_up())
    }
}

pub struct ThermalManager {
    zones: Vec<ThermalZoneMonitor>,

    // Passive cooling (reduce frequency/power)
    passive_cooling_active: bool,
    passive_cooldown_time_ms: u32,

    // Active cooling (fan/liquid)
    active_cooling_active: bool,
    active_cooling_duty_cycle: f64,  // 0.0-1.0

    // History and statistics
    max_zone_temp: f64,
    thermal_throttle_events: usize,
    emergency_shutdowns: usize,

    // Predictive control
    prediction_window_ms: u32,
}

impl ThermalManager {
    pub fn new() -> Self {
        let mut zones = Vec::new();

        // Initialize all 6 thermal zones
        for zone in [
            ThermalZone::SoC,
            ThermalZone::CPUCluster0,
            ThermalZone::CPUCluster1,
            ThermalZone::GPU,
            ThermalZone::Battery,
            ThermalZone::Modem,
        ] {
            zones.push(ThermalZoneMonitor::new(zone));
        }

        Self {
            zones,
            passive_cooling_active: false,
            passive_cooldown_time_ms: 0,
            active_cooling_active: false,
            active_cooling_duty_cycle: 0.0,
            max_zone_temp: 0.0,
            thermal_throttle_events: 0,
            emergency_shutdowns: 0,
            prediction_window_ms: 500,  // Look ahead 500ms
        }
    }

    pub fn update_zone_temperature(
        &mut self,
        zone: ThermalZone,
        temp: f64,
        timestamp: u64,
    ) {
        // Find and update the zone
        for monitor in &mut self.zones {
            if monitor.zone == zone {
                monitor.update_temperature(temp, timestamp);

                // Track peak temperature
                if temp > self.max_zone_temp {
                    self.max_zone_temp = temp;
                }

                // Check if action needed
                if monitor.needs_immediate_action() {
                    self.thermal_throttle_events += 1;
                }

                break;
            }
        }

        // Evaluate overall thermal state
        self.evaluate_cooling_needs();
    }

    fn evaluate_cooling_needs(&mut self) {
        // Find hottest zone
        let max_throttle = self.zones
            .iter()
            .map(|z| z.get_throttle_level())
            .fold(0.0, f64::max);

        // Passive cooling first
        if max_throttle > 0.2 {
            self.passive_cooling_active = true;
            self.passive_cooldown_time_ms = (max_throttle * 5000.0) as u32;
        } else {
            self.passive_cooling_active = false;
        }

        // Active cooling if passive not enough
        if max_throttle > 0.7 {
            self.active_cooling_active = true;
            self.active_cooling_duty_cycle = max_throttle;
        } else {
            self.active_cooling_active = false;
            self.active_cooling_duty_cycle = 0.0;
        }
    }

    pub fn predict_thermal_state(&self, horizon_ms: u32) -> ThermalPrediction {
        let mut predictions = Vec::new();

        for zone in &self.zones {
            // Simple linear projection: current_temp + trend × (horizon_ms / 1000)
            let horizon_sec = horizon_ms as f64 / 1000.0;
            let predicted_temp = zone.current_temp + zone.trend * horizon_sec;

            // Clamp to reasonable range
            let predicted_temp = predicted_temp.max(20.0).min(70.0);

            let predicted_state = match predicted_temp {
                t if t < 35.0 => ThermalState::Cool,
                t if t < 40.0 => ThermalState::Warm,
                t if t < 50.0 => ThermalState::Hot,
                t if t < 55.0 => ThermalState::Critical,
                _ => ThermalState::Emergency,
            };

            predictions.push((zone.zone, predicted_temp, predicted_state));
        }

        ThermalPrediction {
            horizon_ms,
            predictions,
            will_exceed_limit: predictions.iter().any(|(_, _, state)| {
                *state == ThermalState::Emergency
            }),
        }
    }

    pub fn get_global_throttle_level(&self) -> f64 {
        // Maximum throttle across all zones
        self.zones
            .iter()
            .map(|z| z.get_throttle_level())
            .fold(0.0, f64::max)
    }

    pub fn get_hottest_zone(&self) -> Option<(ThermalZone, f64, ThermalState)> {
        self.zones
            .iter()
            .max_by(|a, b| a.current_temp.partial_cmp(&b.current_temp).unwrap())
            .map(|z| (z.zone, z.current_temp, z.thermal_state))
    }

    pub fn get_zone_status(&self, zone: ThermalZone) -> Option<ZoneStatus> {
        self.zones
            .iter()
            .find(|z| z.zone == zone)
            .map(|z| ZoneStatus {
                zone: z.zone,
                current_temp: z.current_temp,
                thermal_state: z.thermal_state,
                throttle_level: z.throttle_level,
                trend: z.trend,
                is_heating: z.is_heating_up(),
                is_cooling: z.is_cooling_down(),
            })
    }

    pub fn get_thermal_summary(&self) -> ThermalSummary {
        let avg_temp = self.zones.iter().map(|z| z.current_temp).sum::<f64>()
            / self.zones.len() as f64;

        let hottest = self.get_hottest_zone();
        let (hottest_zone, hottest_temp) = hottest.map(|(z, t, _)| (z, t)).unwrap_or((ThermalZone::SoC, 0.0));

        ThermalSummary {
            average_temp: avg_temp,
            max_temp: self.max_zone_temp,
            hottest_zone,
            hottest_zone_temp: hottest_temp,
            global_throttle: self.get_global_throttle_level(),
            passive_cooling_active: self.passive_cooling_active,
            active_cooling_active: self.active_cooling_active,
            active_cooling_duty: self.active_cooling_duty_cycle,
            throttle_events: self.thermal_throttle_events,
            emergency_events: self.emergency_shutdowns,
        }
    }

    pub fn reset_stats(&mut self) {
        self.max_zone_temp = 0.0;
        self.thermal_throttle_events = 0;
        self.emergency_shutdowns = 0;
    }
}

#[derive(Clone, Debug)]
pub struct ThermalPrediction {
    pub horizon_ms: u32,
    pub predictions: Vec<(ThermalZone, f64, ThermalState)>,
    pub will_exceed_limit: bool,
}

#[derive(Clone, Debug)]
pub struct ZoneStatus {
    pub zone: ThermalZone,
    pub current_temp: f64,
    pub thermal_state: ThermalState,
    pub throttle_level: f64,
    pub trend: f64,
    pub is_heating: bool,
    pub is_cooling: bool,
}

#[derive(Clone, Debug)]
pub struct ThermalSummary {
    pub average_temp: f64,
    pub max_temp: f64,
    pub hottest_zone: ThermalZone,
    pub hottest_zone_temp: f64,
    pub global_throttle: f64,
    pub passive_cooling_active: bool,
    pub active_cooling_active: bool,
    pub active_cooling_duty: f64,
    pub throttle_events: usize,
    pub emergency_events: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_zone_creation() {
        let zone = ThermalZoneMonitor::new(ThermalZone::SoC);
        assert_eq!(zone.zone, ThermalZone::SoC);
        assert_eq!(zone.current_temp, 25.0);
        assert_eq!(zone.thermal_state, ThermalState::Cool);
    }

    #[test]
    fn test_temperature_update() {
        let mut zone = ThermalZoneMonitor::new(ThermalZone::SoC);
        zone.update_temperature(40.0, 1000);
        assert_eq!(zone.current_temp, 40.0);
    }

    #[test]
    fn test_thermal_state_transitions() {
        let mut zone = ThermalZoneMonitor::new(ThermalZone::SoC);

        zone.update_temperature(30.0, 1000);
        assert_eq!(zone.thermal_state, ThermalState::Cool);

        zone.update_temperature(37.0, 2000);
        assert_eq!(zone.thermal_state, ThermalState::Warm);

        zone.update_temperature(45.0, 3000);
        assert_eq!(zone.thermal_state, ThermalState::Hot);

        zone.update_temperature(53.0, 4000);
        assert_eq!(zone.thermal_state, ThermalState::Critical);
    }

    #[test]
    fn test_throttle_level_calculation() {
        let mut zone = ThermalZoneMonitor::new(ThermalZone::SoC);

        zone.update_temperature(35.0, 1000);
        assert_eq!(zone.get_throttle_level(), 0.0);  // No throttle

        zone.update_temperature(45.0, 2000);
        assert!(zone.get_throttle_level() > 0.0);    // Some throttle

        zone.update_temperature(50.0, 3000);
        assert_eq!(zone.get_throttle_level(), 1.0);  // Full throttle
    }

    #[test]
    fn test_thermal_trend() {
        let mut zone = ThermalZoneMonitor::new(ThermalZone::SoC);

        zone.update_temperature(30.0, 1000);
        zone.update_temperature(33.0, 2000);

        // Should detect heating (3°C per second)
        assert!(zone.is_heating_up());
    }

    #[test]
    fn test_cooling_detection() {
        let mut zone = ThermalZoneMonitor::new(ThermalZone::SoC);

        zone.update_temperature(45.0, 1000);
        zone.update_temperature(42.0, 2000);

        // Should detect cooling
        assert!(zone.is_cooling_down());
    }

    #[test]
    fn test_manager_creation() {
        let manager = ThermalManager::new();
        assert_eq!(manager.zones.len(), 6);
    }

    #[test]
    fn test_zone_temperature_update() {
        let mut manager = ThermalManager::new();

        manager.update_zone_temperature(ThermalZone::SoC, 45.0, 1000);

        let status = manager.get_zone_status(ThermalZone::SoC);
        assert!(status.is_some());
        assert_eq!(status.unwrap().current_temp, 45.0);
    }

    #[test]
    fn test_passive_cooling_activation() {
        let mut manager = ThermalManager::new();

        manager.update_zone_temperature(ThermalZone::SoC, 48.0, 1000);

        // High temperature should activate passive cooling
        assert!(manager.passive_cooling_active);
    }

    #[test]
    fn test_active_cooling_activation() {
        let mut manager = ThermalManager::new();

        // Very high temperature
        manager.update_zone_temperature(ThermalZone::SoC, 52.0, 1000);

        assert!(manager.active_cooling_active);
        assert!(manager.active_cooling_duty_cycle > 0.5);
    }

    #[test]
    fn test_thermal_prediction() {
        let mut manager = ThermalManager::new();

        manager.update_zone_temperature(ThermalZone::SoC, 35.0, 1000);
        manager.update_zone_temperature(ThermalZone::SoC, 38.0, 2000);

        let prediction = manager.predict_thermal_state(1000);
        assert!(!prediction.predictions.is_empty());
    }

    #[test]
    fn test_hottest_zone() {
        let mut manager = ThermalManager::new();

        manager.update_zone_temperature(ThermalZone::SoC, 40.0, 1000);
        manager.update_zone_temperature(ThermalZone::GPU, 45.0, 1000);
        manager.update_zone_temperature(ThermalZone::Battery, 35.0, 1000);

        let hottest = manager.get_hottest_zone();
        assert!(hottest.is_some());
        assert_eq!(hottest.unwrap().0, ThermalZone::GPU);
    }

    #[test]
    fn test_thermal_summary() {
        let mut manager = ThermalManager::new();

        for _ in 0..6 {
            manager.update_zone_temperature(ThermalZone::SoC, 40.0, 1000);
        }

        let summary = manager.get_thermal_summary();
        assert!(summary.average_temp > 0.0);
        assert!(summary.max_temp > 0.0);
    }

    #[test]
    fn test_global_throttle_level() {
        let mut manager = ThermalManager::new();

        manager.update_zone_temperature(ThermalZone::SoC, 30.0, 1000);
        assert_eq!(manager.get_global_throttle_level(), 0.0);

        manager.update_zone_temperature(ThermalZone::SoC, 50.0, 2000);
        assert!(manager.get_global_throttle_level() > 0.0);
    }

    #[test]
    fn test_max_safe_temp_per_zone() {
        let soc_zone = ThermalZoneMonitor::new(ThermalZone::SoC);
        let gpu_zone = ThermalZoneMonitor::new(ThermalZone::GPU);

        // Different zones have different max safe temps
        assert_eq!(soc_zone.max_safe_temp, 50.0);
        assert_eq!(gpu_zone.max_safe_temp, 52.0);
    }

    #[test]
    fn test_reset_statistics() {
        let mut manager = ThermalManager::new();

        manager.update_zone_temperature(ThermalZone::SoC, 52.0, 1000);
        assert!(manager.thermal_throttle_events > 0);

        manager.reset_stats();
        assert_eq!(manager.thermal_throttle_events, 0);
    }
}
