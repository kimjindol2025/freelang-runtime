// Project Sovereign: CPU Frequency Scaling Module (DVFS)
// Goal: Dynamic Voltage & Frequency Scaling for energy efficiency
// Target: Frequency range 300MHz - 3.4GHz with voltage co-scaling

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CPUFrequency {
    // Performance tiers: MHz
    Conservative,  // 300MHz - minimal power
    PowerSave,     // 800MHz - light tasks
    Balanced,      // 1.5GHz - normal operation
    Performance,   // 2.4GHz - demanding apps
    Max,          // 3.4GHz - gaming/recording
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VoltageLevel {
    // Voltage (mV) - scaled with frequency
    UltraLow,   // 600mV (300MHz)
    Low,        // 700mV (800MHz)
    Normal,     // 800mV (1.5GHz)
    High,       // 900mV (2.4GHz)
    UltraHigh,  // 1000mV (3.4GHz)
}

impl CPUFrequency {
    pub fn to_mhz(&self) -> u32 {
        match self {
            CPUFrequency::Conservative => 300,
            CPUFrequency::PowerSave => 800,
            CPUFrequency::Balanced => 1500,
            CPUFrequency::Performance => 2400,
            CPUFrequency::Max => 3400,
        }
    }

    pub fn to_voltage(&self) -> VoltageLevel {
        match self {
            CPUFrequency::Conservative => VoltageLevel::UltraLow,
            CPUFrequency::PowerSave => VoltageLevel::Low,
            CPUFrequency::Balanced => VoltageLevel::Normal,
            CPUFrequency::Performance => VoltageLevel::High,
            CPUFrequency::Max => VoltageLevel::UltraHigh,
        }
    }

    pub fn power_consumption_mw(&self) -> f64 {
        // P = C × V² × f (capacitance × voltage² × frequency)
        // Rough estimation: power scales with f × V²
        match self {
            CPUFrequency::Conservative => 20.0,    // 300MHz @ 0.6V
            CPUFrequency::PowerSave => 80.0,       // 800MHz @ 0.7V
            CPUFrequency::Balanced => 200.0,       // 1.5GHz @ 0.8V
            CPUFrequency::Performance => 450.0,    // 2.4GHz @ 0.9V
            CPUFrequency::Max => 800.0,            // 3.4GHz @ 1.0V
        }
    }

    pub fn idle_power_mw(&self) -> f64 {
        self.power_consumption_mw() * 0.1  // 10% of active power
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CPUCore {
    BigCore0,
    BigCore1,
    MidCore,
    LittleCore0,
    LittleCore1,
    LittleCore2,
}

#[derive(Clone, Debug)]
pub struct CPUFrequencyScaler {
    // Current state
    current_frequency: CPUFrequency,
    current_voltage: VoltageLevel,
    target_frequency: CPUFrequency,

    // Per-core tracking
    big_cores_active: usize,
    mid_cores_active: usize,
    little_cores_active: usize,

    // Scaling constraints
    max_temperature: f64,
    max_frequency_at_temp: CPUFrequency,
    thermal_throttling_active: bool,

    // Statistics
    total_scaling_operations: usize,
    total_time_conservative: u64,
    total_time_max: u64,
    frequency_changes: Vec<(u64, CPUFrequency)>,  // timestamp, freq
    power_budget_mw: f64,
}

impl CPUFrequencyScaler {
    pub fn new(power_budget_mw: f64) -> Self {
        Self {
            current_frequency: CPUFrequency::Balanced,
            current_voltage: VoltageLevel::Normal,
            target_frequency: CPUFrequency::Balanced,
            big_cores_active: 2,
            mid_cores_active: 1,
            little_cores_active: 1,
            max_temperature: 35.0,
            max_frequency_at_temp: CPUFrequency::Max,
            thermal_throttling_active: false,
            total_scaling_operations: 0,
            total_time_conservative: 0,
            total_time_max: 0,
            frequency_changes: Vec::new(),
            power_budget_mw,
        }
    }

    /// Scale frequency based on workload and thermal constraints
    pub fn scale_frequency(
        &mut self,
        cpu_usage: f64,           // 0.0-1.0
        temperature: f64,         // °C
        timestamp: u64,           // ms
    ) -> CPUFrequency {
        // Step 1: Determine target frequency based on CPU usage
        let usage_target = self.usage_to_frequency(cpu_usage);

        // Step 2: Apply thermal constraints
        let thermal_limit = self.apply_thermal_limit(temperature);

        // Step 3: Determine final frequency (conservative of both)
        let target = self.select_conservative_frequency(usage_target, thermal_limit);

        // Step 4: Apply frequency change with transition time
        if target != self.current_frequency {
            self.execute_frequency_change(target, timestamp);
        }

        self.current_frequency
    }

    fn usage_to_frequency(&self, usage: f64) -> CPUFrequency {
        match usage {
            u if u < 0.1 => CPUFrequency::Conservative,
            u if u < 0.25 => CPUFrequency::PowerSave,
            u if u < 0.5 => CPUFrequency::Balanced,
            u if u < 0.75 => CPUFrequency::Performance,
            _ => CPUFrequency::Max,
        }
    }

    fn apply_thermal_limit(&mut self, temperature: f64) -> CPUFrequency {
        self.max_temperature = temperature;

        // Thermal throttling policy
        if temperature > 55.0 {
            self.thermal_throttling_active = true;
            CPUFrequency::PowerSave  // Aggressive throttle
        } else if temperature > 50.0 {
            self.thermal_throttling_active = true;
            CPUFrequency::Balanced   // Moderate throttle
        } else if temperature > 45.0 {
            self.thermal_throttling_active = true;
            CPUFrequency::Performance  // Light throttle
        } else {
            self.thermal_throttling_active = false;
            CPUFrequency::Max  // No limit
        }
    }

    fn select_conservative_frequency(
        &self,
        usage_target: CPUFrequency,
        thermal_limit: CPUFrequency,
    ) -> CPUFrequency {
        // Use lower of the two frequencies
        if (usage_target as u32) < (thermal_limit as u32) {
            usage_target
        } else {
            thermal_limit
        }
    }

    fn execute_frequency_change(&mut self, new_frequency: CPUFrequency, timestamp: u64) {
        self.current_frequency = new_frequency;
        self.current_voltage = new_frequency.to_voltage();
        self.target_frequency = new_frequency;
        self.total_scaling_operations += 1;
        self.frequency_changes.push((timestamp, new_frequency));

        // Track time spent at extremes
        if new_frequency == CPUFrequency::Conservative {
            self.total_time_conservative += 1;
        } else if new_frequency == CPUFrequency::Max {
            self.total_time_max += 1;
        }

        // Limit history to last 1000 changes
        if self.frequency_changes.len() > 1000 {
            self.frequency_changes.remove(0);
        }
    }

    /// Get current power consumption
    pub fn get_current_power(&self) -> f64 {
        let active_power = self.current_frequency.power_consumption_mw();
        let idle_power = self.current_frequency.idle_power_mw();

        // Assume: 30% active, 70% idle when at this frequency
        active_power * 0.3 + idle_power * 0.7
    }

    /// Check if frequency change would violate power budget
    pub fn can_scale_to(&self, frequency: CPUFrequency) -> bool {
        frequency.power_consumption_mw() <= self.power_budget_mw
    }

    /// Get frequency scaling efficiency (0.0-1.0)
    pub fn get_scaling_efficiency(&self) -> f64 {
        // Efficiency = actual power / budget
        self.get_current_power() / self.power_budget_mw
    }

    /// Update power budget dynamically
    pub fn update_power_budget(&mut self, new_budget_mw: f64) {
        self.power_budget_mw = new_budget_mw;
    }

    /// Get scaling statistics
    pub fn get_scaling_stats(&self) -> DVFSStats {
        let avg_frequency = if self.frequency_changes.is_empty() {
            self.current_frequency.to_mhz() as f64
        } else {
            let sum: u32 = self.frequency_changes
                .iter()
                .map(|(_, f)| f.to_mhz())
                .sum();
            sum as f64 / self.frequency_changes.len() as f64
        };

        DVFSStats {
            current_frequency_mhz: self.current_frequency.to_mhz(),
            avg_frequency_mhz: avg_frequency as u32,
            current_voltage_mv: self.current_voltage.to_mv(),
            current_power_mw: self.get_current_power(),
            total_scaling_ops: self.total_scaling_operations,
            thermal_throttling: self.thermal_throttling_active,
            power_budget_mw: self.power_budget_mw,
            efficiency: self.get_scaling_efficiency(),
        }
    }

    /// Get frequency history
    pub fn get_frequency_history(&self) -> Vec<(u64, u32)> {
        self.frequency_changes
            .iter()
            .map(|(ts, freq)| (*ts, freq.to_mhz()))
            .collect()
    }

    /// Reset scaling statistics
    pub fn reset_stats(&mut self) {
        self.total_scaling_operations = 0;
        self.total_time_conservative = 0;
        self.total_time_max = 0;
        self.frequency_changes.clear();
    }
}

impl VoltageLevel {
    pub fn to_mv(&self) -> u32 {
        match self {
            VoltageLevel::UltraLow => 600,
            VoltageLevel::Low => 700,
            VoltageLevel::Normal => 800,
            VoltageLevel::High => 900,
            VoltageLevel::UltraHigh => 1000,
        }
    }

    pub fn leakage_power_mw(&self) -> f64 {
        // Leakage power scales with V²
        match self {
            VoltageLevel::UltraLow => 5.0,
            VoltageLevel::Low => 8.0,
            VoltageLevel::Normal => 12.0,
            VoltageLevel::High => 18.0,
            VoltageLevel::UltraHigh => 25.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DVFSStats {
    pub current_frequency_mhz: u32,
    pub avg_frequency_mhz: u32,
    pub current_voltage_mv: u32,
    pub current_power_mw: f64,
    pub total_scaling_ops: usize,
    pub thermal_throttling: bool,
    pub power_budget_mw: f64,
    pub efficiency: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_creation() {
        assert_eq!(CPUFrequency::Conservative.to_mhz(), 300);
        assert_eq!(CPUFrequency::Max.to_mhz(), 3400);
    }

    #[test]
    fn test_voltage_scaling() {
        assert_eq!(
            CPUFrequency::Conservative.to_voltage(),
            VoltageLevel::UltraLow
        );
        assert_eq!(CPUFrequency::Max.to_voltage(), VoltageLevel::UltraHigh);
    }

    #[test]
    fn test_power_consumption() {
        assert!(CPUFrequency::Conservative.power_consumption_mw() < 50.0);
        assert!(CPUFrequency::Max.power_consumption_mw() > 700.0);
    }

    #[test]
    fn test_dvfs_creation() {
        let scaler = CPUFrequencyScaler::new(500.0);
        assert_eq!(scaler.current_frequency, CPUFrequency::Balanced);
        assert_eq!(scaler.power_budget_mw, 500.0);
    }

    #[test]
    fn test_usage_to_frequency() {
        let scaler = CPUFrequencyScaler::new(500.0);

        assert_eq!(scaler.usage_to_frequency(0.05), CPUFrequency::Conservative);
        assert_eq!(scaler.usage_to_frequency(0.2), CPUFrequency::PowerSave);
        assert_eq!(scaler.usage_to_frequency(0.4), CPUFrequency::Balanced);
        assert_eq!(scaler.usage_to_frequency(0.7), CPUFrequency::Performance);
        assert_eq!(scaler.usage_to_frequency(0.9), CPUFrequency::Max);
    }

    #[test]
    fn test_thermal_limiting() {
        let mut scaler = CPUFrequencyScaler::new(500.0);

        // Cool - no throttling
        let freq = scaler.apply_thermal_limit(35.0);
        assert_eq!(freq, CPUFrequency::Max);

        // Warm - light throttle
        let freq = scaler.apply_thermal_limit(47.0);
        assert_eq!(freq, CPUFrequency::Performance);

        // Hot - moderate throttle
        let freq = scaler.apply_thermal_limit(52.0);
        assert_eq!(freq, CPUFrequency::Balanced);

        // Critical - aggressive throttle
        let freq = scaler.apply_thermal_limit(57.0);
        assert_eq!(freq, CPUFrequency::PowerSave);
    }

    #[test]
    fn test_frequency_scaling_operation() {
        let mut scaler = CPUFrequencyScaler::new(500.0);

        // Low usage at cool temp
        let freq = scaler.scale_frequency(0.1, 35.0, 1000);
        assert_eq!(freq, CPUFrequency::Conservative);
        assert_eq!(scaler.total_scaling_operations, 1);

        // High usage at normal temp
        let freq = scaler.scale_frequency(0.8, 40.0, 2000);
        assert_eq!(freq, CPUFrequency::Max);
        assert_eq!(scaler.total_scaling_operations, 2);
    }

    #[test]
    fn test_thermal_throttling_overrides_usage() {
        let mut scaler = CPUFrequencyScaler::new(500.0);

        // High usage but high temperature
        let freq = scaler.scale_frequency(0.9, 57.0, 1000);
        // Should be throttled to PowerSave despite high usage
        assert_eq!(freq, CPUFrequency::PowerSave);
        assert!(scaler.thermal_throttling_active);
    }

    #[test]
    fn test_power_budget_constraint() {
        let scaler = CPUFrequencyScaler::new(100.0);  // Very low budget

        // Can scale to conservative (20mW)
        assert!(scaler.can_scale_to(CPUFrequency::Conservative));

        // Cannot scale to max (800mW)
        assert!(!scaler.can_scale_to(CPUFrequency::Max));
    }

    #[test]
    fn test_scaling_efficiency() {
        let scaler = CPUFrequencyScaler::new(200.0);
        let efficiency = scaler.get_scaling_efficiency();

        assert!(efficiency > 0.0);
        assert!(efficiency <= 1.0);
    }

    #[test]
    fn test_frequency_history() {
        let mut scaler = CPUFrequencyScaler::new(500.0);

        scaler.scale_frequency(0.1, 35.0, 1000);
        scaler.scale_frequency(0.8, 40.0, 2000);
        scaler.scale_frequency(0.5, 38.0, 3000);

        let history = scaler.get_frequency_history();
        assert!(history.len() >= 2);  // At least 2 changes
    }

    #[test]
    fn test_statistics() {
        let mut scaler = CPUFrequencyScaler::new(500.0);

        for i in 0..5 {
            scaler.scale_frequency(0.2 * i as f64, 35.0 + (i as f64 * 2.0), 1000 + (i as u64 * 100));
        }

        let stats = scaler.get_scaling_stats();
        assert!(stats.current_power_mw > 0.0);
        assert!(stats.total_scaling_ops > 0);
    }

    #[test]
    fn test_dynamic_power_budget_update() {
        let mut scaler = CPUFrequencyScaler::new(100.0);

        assert!(!scaler.can_scale_to(CPUFrequency::Max));

        scaler.update_power_budget(1000.0);
        assert!(scaler.can_scale_to(CPUFrequency::Max));
    }

    #[test]
    fn test_voltage_leakage_power() {
        assert!(VoltageLevel::UltraLow.leakage_power_mw() < 10.0);
        assert!(VoltageLevel::UltraHigh.leakage_power_mw() > 20.0);
    }

    #[test]
    fn test_idle_power_consumption() {
        let idle_power = CPUFrequency::Max.idle_power_mw();
        assert!(idle_power > 0.0);
        assert!(idle_power < CPUFrequency::Max.power_consumption_mw());
    }

    #[test]
    fn test_reset_statistics() {
        let mut scaler = CPUFrequencyScaler::new(500.0);

        scaler.scale_frequency(0.5, 35.0, 1000);
        assert!(scaler.total_scaling_operations > 0);

        scaler.reset_stats();
        assert_eq!(scaler.total_scaling_operations, 0);
        assert_eq!(scaler.frequency_changes.len(), 0);
    }

    #[test]
    fn test_frequency_rounding() {
        let scaler = CPUFrequencyScaler::new(500.0);

        // Test all frequency values are sensible
        for freq in [
            CPUFrequency::Conservative,
            CPUFrequency::PowerSave,
            CPUFrequency::Balanced,
            CPUFrequency::Performance,
            CPUFrequency::Max,
        ] {
            assert!(freq.to_mhz() > 0);
            assert!(freq.to_mhz() <= 3400);
        }
    }

    #[test]
    fn test_voltage_mv_values() {
        assert_eq!(VoltageLevel::UltraLow.to_mv(), 600);
        assert_eq!(VoltageLevel::Low.to_mv(), 700);
        assert_eq!(VoltageLevel::Normal.to_mv(), 800);
        assert_eq!(VoltageLevel::High.to_mv(), 900);
        assert_eq!(VoltageLevel::UltraHigh.to_mv(), 1000);
    }
}
