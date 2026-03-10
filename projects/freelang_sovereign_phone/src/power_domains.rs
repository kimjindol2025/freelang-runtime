// Project Sovereign: Power Domains Module
// Goal: Individual power control for CPU, GPU, Modem, Display
// Target: <10ms switching between power states

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerState {
    Off,        // 0mW (powered down)
    Sleep,      // <5mW (retention power)
    Idle,       // 10-50mW (active but idle)
    Active,     // 100-500mW (normal operation)
    Turbo,      // 500mW+ (maximum performance)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModemMode {
    Disabled,   // 0mW
    Mode2G,     // 50mW (GSM/EDGE)
    Mode3G,     // 150mW (UMTS/HSPA)
    Mode4G,     // 300mW (LTE)
    Mode5G,     // 400mW (NR)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GPUState {
    PowerOff,    // 0mW
    Idle,        // 20mW
    Light,       // 100mW (2D graphics)
    Medium,      // 250mW (3D, 30fps)
    Performance, // 500mW (3D, 60fps+)
}

#[derive(Clone, Debug)]
pub struct PowerDomain {
    name: String,
    current_state: PowerState,
    current_power_mw: f64,
    is_online: bool,
    switch_latency_ms: u32,
    supported_states: Vec<PowerState>,
}

impl PowerDomain {
    pub fn new(name: &str, supported_states: Vec<PowerState>) -> Self {
        Self {
            name: name.to_string(),
            current_state: PowerState::Idle,
            current_power_mw: 0.0,
            is_online: true,
            switch_latency_ms: 0,
            supported_states,
        }
    }

    pub fn power_for_state(state: PowerState) -> f64 {
        match state {
            PowerState::Off => 0.0,
            PowerState::Sleep => 2.0,
            PowerState::Idle => 25.0,
            PowerState::Active => 200.0,
            PowerState::Turbo => 500.0,
        }
    }

    pub fn switch_to(&mut self, new_state: PowerState) -> bool {
        if !self.supported_states.contains(&new_state) {
            return false;
        }

        // Calculate transition time (5ms per state change)
        self.switch_latency_ms = 5;

        self.current_state = new_state;
        self.current_power_mw = Self::power_for_state(new_state);
        self.is_online = new_state != PowerState::Off;

        true
    }

    pub fn get_power_mw(&self) -> f64 {
        self.current_power_mw
    }

    pub fn is_powered(&self) -> bool {
        self.is_online && self.current_state != PowerState::Off
    }
}

pub struct PowerDomainManager {
    // Individual power domains
    cpu0: PowerDomain,      // Cluster 0
    cpu1: PowerDomain,      // Cluster 1
    gpu: PowerDomain,       // Graphics processor
    modem: PowerDomain,     // Cellular modem
    display: PowerDomain,   // Screen backlight
    dsp: PowerDomain,       // Digital signal processor

    // Modem mode tracking
    modem_mode: ModemMode,
    modem_power_mw: f64,

    // GPU state tracking
    gpu_state: GPUState,

    // Statistics
    total_domain_switches: usize,
    total_power_budget_mw: f64,
    peak_power_mw: f64,
    history: Vec<(u64, f64)>,  // timestamp, total power
}

impl PowerDomainManager {
    pub fn new() -> Self {
        Self {
            cpu0: PowerDomain::new("CPU0", vec![
                PowerState::Sleep,
                PowerState::Idle,
                PowerState::Active,
                PowerState::Turbo,
            ]),
            cpu1: PowerDomain::new("CPU1", vec![
                PowerState::Sleep,
                PowerState::Idle,
                PowerState::Active,
                PowerState::Turbo,
            ]),
            gpu: PowerDomain::new("GPU", vec![
                PowerState::Off,
                PowerState::Idle,
                PowerState::Active,
                PowerState::Turbo,
            ]),
            modem: PowerDomain::new("Modem", vec![
                PowerState::Off,
                PowerState::Sleep,
                PowerState::Idle,
                PowerState::Active,
            ]),
            display: PowerDomain::new("Display", vec![
                PowerState::Off,
                PowerState::Idle,
                PowerState::Active,
            ]),
            dsp: PowerDomain::new("DSP", vec![
                PowerState::Off,
                PowerState::Sleep,
                PowerState::Active,
            ]),

            modem_mode: ModemMode::Disabled,
            modem_power_mw: 0.0,

            gpu_state: GPUState::PowerOff,

            total_domain_switches: 0,
            total_power_budget_mw: 0.0,
            peak_power_mw: 0.0,
            history: Vec::new(),
        }
    }

    /// Switch modem mode (automatic for battery/network optimization)
    pub fn set_modem_mode(&mut self, mode: ModemMode) -> bool {
        let new_state = match mode {
            ModemMode::Disabled => PowerState::Off,
            ModemMode::Mode2G => PowerState::Active,
            ModemMode::Mode3G => PowerState::Active,
            ModemMode::Mode4G => PowerState::Active,
            ModemMode::Mode5G => PowerState::Turbo,
        };

        if self.modem.switch_to(new_state) {
            self.modem_mode = mode;
            self.modem_power_mw = self.modem_mode.power_consumption();
            self.total_domain_switches += 1;
            true
        } else {
            false
        }
    }

    /// Set GPU state for rendering
    pub fn set_gpu_state(&mut self, state: GPUState) -> bool {
        let power_state = match state {
            GPUState::PowerOff => PowerState::Off,
            GPUState::Idle => PowerState::Idle,
            GPUState::Light => PowerState::Active,
            GPUState::Medium => PowerState::Active,
            GPUState::Performance => PowerState::Turbo,
        };

        if self.gpu.switch_to(power_state) {
            self.gpu_state = state;
            self.total_domain_switches += 1;
            true
        } else {
            false
        }
    }

    /// Set CPU cluster power state
    pub fn set_cpu_cluster(
        &mut self,
        cluster: usize,  // 0 or 1
        state: PowerState,
    ) -> bool {
        let domain = if cluster == 0 {
            &mut self.cpu0
        } else if cluster == 1 {
            &mut self.cpu1
        } else {
            return false;
        };

        if domain.switch_to(state) {
            self.total_domain_switches += 1;
            true
        } else {
            false
        }
    }

    /// Set display power with brightness control
    pub fn set_display_power(&mut self, brightness: f64) -> bool {
        // 0.0 = off, >0.0 = on with brightness level
        let state = if brightness <= 0.0 {
            PowerState::Off
        } else if brightness < 0.2 {
            PowerState::Idle
        } else {
            PowerState::Active
        };

        if self.display.switch_to(state) {
            self.total_domain_switches += 1;
            true
        } else {
            false
        }
    }

    /// Set DSP for audio processing
    pub fn set_dsp_active(&mut self, active: bool) -> bool {
        let state = if active {
            PowerState::Active
        } else {
            PowerState::Sleep
        };

        if self.dsp.switch_to(state) {
            self.total_domain_switches += 1;
            true
        } else {
            false
        }
    }

    /// Get total power consumption from all domains
    pub fn get_total_power(&mut self, timestamp: u64) -> f64 {
        let total = self.cpu0.get_power_mw()
            + self.cpu1.get_power_mw()
            + self.gpu.get_power_mw()
            + self.modem.get_power_mw()
            + self.display.get_power_mw()
            + self.dsp.get_power_mw();

        self.total_power_budget_mw = total;
        if total > self.peak_power_mw {
            self.peak_power_mw = total;
        }

        self.history.push((timestamp, total));
        if self.history.len() > 500 {
            self.history.remove(0);
        }

        total
    }

    /// Get per-domain breakdown
    pub fn get_power_breakdown(&self) -> PowerBreakdown {
        PowerBreakdown {
            cpu0_mw: self.cpu0.get_power_mw(),
            cpu1_mw: self.cpu1.get_power_mw(),
            gpu_mw: self.gpu.get_power_mw(),
            modem_mw: self.modem_power_mw,
            display_mw: self.display.get_power_mw(),
            dsp_mw: self.dsp.get_power_mw(),
            total_mw: self.total_power_budget_mw,
        }
    }

    /// Check if system is in low-power state
    pub fn is_low_power_state(&self) -> bool {
        // Low power if total < 100mW
        self.total_power_budget_mw < 100.0
    }

    /// Estimate time to wake from sleep (ms)
    pub fn estimate_wake_latency(&self) -> u32 {
        let mut total_latency = 0u32;

        // Max 5ms per domain that needs to wake
        if self.cpu0.current_state == PowerState::Sleep {
            total_latency += 5;
        }
        if self.cpu1.current_state == PowerState::Sleep {
            total_latency += 5;
        }
        if self.gpu.current_state == PowerState::Off {
            total_latency += 10;  // GPU takes longer
        }
        if self.modem.current_state == PowerState::Off {
            total_latency += 15;  // Modem initialization
        }

        total_latency
    }

    /// Get statistics
    pub fn get_stats(&self) -> PowerDomainStats {
        let avg_power = if self.history.is_empty() {
            0.0
        } else {
            let sum: f64 = self.history.iter().map(|(_, p)| p).sum();
            sum / self.history.len() as f64
        };

        PowerDomainStats {
            total_domain_switches: self.total_domain_switches,
            peak_power_mw: self.peak_power_mw,
            current_power_mw: self.total_power_budget_mw,
            avg_power_mw: avg_power,
            modem_mode: self.modem_mode,
            gpu_state: self.gpu_state,
            is_low_power: self.is_low_power_state(),
            wake_latency_ms: self.estimate_wake_latency(),
        }
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.total_domain_switches = 0;
        self.peak_power_mw = 0.0;
        self.history.clear();
    }
}

impl ModemMode {
    pub fn power_consumption(&self) -> f64 {
        match self {
            ModemMode::Disabled => 0.0,
            ModemMode::Mode2G => 50.0,
            ModemMode::Mode3G => 150.0,
            ModemMode::Mode4G => 300.0,
            ModemMode::Mode5G => 400.0,
        }
    }
}

impl GPUState {
    pub fn power_consumption(&self) -> f64 {
        match self {
            GPUState::PowerOff => 0.0,
            GPUState::Idle => 20.0,
            GPUState::Light => 100.0,
            GPUState::Medium => 250.0,
            GPUState::Performance => 500.0,
        }
    }

    pub fn fps(&self) -> u32 {
        match self {
            GPUState::PowerOff => 0,
            GPUState::Idle => 0,
            GPUState::Light => 30,
            GPUState::Medium => 30,
            GPUState::Performance => 60,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PowerBreakdown {
    pub cpu0_mw: f64,
    pub cpu1_mw: f64,
    pub gpu_mw: f64,
    pub modem_mw: f64,
    pub display_mw: f64,
    pub dsp_mw: f64,
    pub total_mw: f64,
}

#[derive(Clone, Debug)]
pub struct PowerDomainStats {
    pub total_domain_switches: usize,
    pub peak_power_mw: f64,
    pub current_power_mw: f64,
    pub avg_power_mw: f64,
    pub modem_mode: ModemMode,
    pub gpu_state: GPUState,
    pub is_low_power: bool,
    pub wake_latency_ms: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_domain_creation() {
        let domain = PowerDomain::new("CPU", vec![PowerState::Idle, PowerState::Active]);
        assert_eq!(domain.name, "CPU");
        assert_eq!(domain.current_state, PowerState::Idle);
    }

    #[test]
    fn test_power_state_transitions() {
        let mut domain = PowerDomain::new("CPU", vec![PowerState::Idle, PowerState::Active]);

        assert!(domain.switch_to(PowerState::Active));
        assert_eq!(domain.current_state, PowerState::Active);
        assert!(domain.is_powered());
    }

    #[test]
    fn test_unsupported_state_transition() {
        let mut domain = PowerDomain::new("CPU", vec![PowerState::Idle]);

        // Cannot switch to unsupported state
        assert!(!domain.switch_to(PowerState::Active));
        assert_eq!(domain.current_state, PowerState::Idle);
    }

    #[test]
    fn test_power_for_state() {
        assert_eq!(PowerDomain::power_for_state(PowerState::Off), 0.0);
        assert!(PowerDomain::power_for_state(PowerState::Active) > 0.0);
        assert!(
            PowerDomain::power_for_state(PowerState::Turbo)
                > PowerDomain::power_for_state(PowerState::Active)
        );
    }

    #[test]
    fn test_manager_creation() {
        let manager = PowerDomainManager::new();
        assert_eq!(manager.total_domain_switches, 0);
        assert_eq!(manager.modem_mode, ModemMode::Disabled);
    }

    #[test]
    fn test_set_modem_mode() {
        let mut manager = PowerDomainManager::new();

        assert!(manager.set_modem_mode(ModemMode::Mode4G));
        assert_eq!(manager.modem_mode, ModemMode::Mode4G);
        assert!(manager.total_domain_switches > 0);
    }

    #[test]
    fn test_set_gpu_state() {
        let mut manager = PowerDomainManager::new();

        assert!(manager.set_gpu_state(GPUState::Medium));
        assert_eq!(manager.gpu_state, GPUState::Medium);
    }

    #[test]
    fn test_set_cpu_cluster() {
        let mut manager = PowerDomainManager::new();

        assert!(manager.set_cpu_cluster(0, PowerState::Active));
        assert_eq!(manager.cpu0.current_state, PowerState::Active);

        assert!(manager.set_cpu_cluster(1, PowerState::Sleep));
        assert_eq!(manager.cpu1.current_state, PowerState::Sleep);
    }

    #[test]
    fn test_set_display_power() {
        let mut manager = PowerDomainManager::new();

        // Off
        assert!(manager.set_display_power(0.0));
        assert_eq!(manager.display.current_state, PowerState::Off);

        // Bright
        assert!(manager.set_display_power(0.8));
        assert_eq!(manager.display.current_state, PowerState::Active);
    }

    #[test]
    fn test_get_total_power() {
        let mut manager = PowerDomainManager::new();

        manager.set_modem_mode(ModemMode::Mode4G);
        manager.set_gpu_state(GPUState::Medium);

        let total = manager.get_total_power(1000);
        assert!(total > 0.0);
        assert!(total >= 300.0);  // At least modem + gpu
    }

    #[test]
    fn test_power_breakdown() {
        let mut manager = PowerDomainManager::new();

        manager.set_modem_mode(ModemMode::Mode4G);
        manager.set_gpu_state(GPUState::Medium);
        manager.get_total_power(1000);

        let breakdown = manager.get_power_breakdown();
        assert!(breakdown.total_mw > 0.0);
        assert!(breakdown.modem_mw >= 300.0);
        assert!(breakdown.gpu_mw >= 250.0);
    }

    #[test]
    fn test_low_power_state_detection() {
        let mut manager = PowerDomainManager::new();

        // All off = low power
        manager.get_total_power(1000);
        assert!(manager.is_low_power_state());

        // With modem on = not low power
        manager.set_modem_mode(ModemMode::Mode4G);
        manager.get_total_power(2000);
        assert!(!manager.is_low_power_state());
    }

    #[test]
    fn test_wake_latency_estimation() {
        let mut manager = PowerDomainManager::new();

        manager.set_cpu_cluster(0, PowerState::Sleep);
        manager.set_modem_mode(ModemMode::Disabled);

        let latency = manager.estimate_wake_latency();
        assert!(latency > 0);
        assert!(latency < 50);  // Should be relatively fast
    }

    #[test]
    fn test_statistics() {
        let mut manager = PowerDomainManager::new();

        manager.set_modem_mode(ModemMode::Mode4G);
        manager.get_total_power(1000);
        manager.get_total_power(2000);

        let stats = manager.get_stats();
        assert!(stats.peak_power_mw > 0.0);
        assert!(stats.avg_power_mw > 0.0);
    }

    #[test]
    fn test_modem_mode_power_levels() {
        assert_eq!(ModemMode::Disabled.power_consumption(), 0.0);
        assert!(ModemMode::Mode2G.power_consumption() < ModemMode::Mode3G.power_consumption());
        assert!(ModemMode::Mode3G.power_consumption() < ModemMode::Mode4G.power_consumption());
        assert!(ModemMode::Mode4G.power_consumption() < ModemMode::Mode5G.power_consumption());
    }

    #[test]
    fn test_gpu_fps_correlation() {
        assert_eq!(GPUState::PowerOff.fps(), 0);
        assert_eq!(GPUState::Light.fps(), 30);
        assert_eq!(GPUState::Performance.fps(), 60);
    }

    #[test]
    fn test_reset_statistics() {
        let mut manager = PowerDomainManager::new();

        manager.set_modem_mode(ModemMode::Mode4G);
        manager.get_total_power(1000);

        assert!(manager.total_domain_switches > 0);
        manager.reset_stats();
        assert_eq!(manager.total_domain_switches, 0);
    }

    #[test]
    fn test_multiple_domain_coordination() {
        let mut manager = PowerDomainManager::new();

        // Simulate gaming session
        assert!(manager.set_gpu_state(GPUState::Performance));
        assert!(manager.set_modem_mode(ModemMode::Mode4G));
        assert!(manager.set_dsp_active(true));

        manager.get_total_power(1000);
        let power = manager.get_total_power(2000);

        assert!(power > 700.0);  // Should be high power
    }

    #[test]
    fn test_idle_mode() {
        let mut manager = PowerDomainManager::new();

        // Simulate idle state
        assert!(manager.set_gpu_state(GPUState::PowerOff));
        assert!(manager.set_modem_mode(ModemMode::Disabled));
        assert!(manager.set_display_power(0.0));
        assert!(manager.set_cpu_cluster(0, PowerState::Sleep));
        assert!(manager.set_cpu_cluster(1, PowerState::Sleep));

        manager.get_total_power(1000);
        assert!(manager.is_low_power_state());
    }
}
