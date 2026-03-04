// Project Sovereign: Performance Profiler Module
// Goal: Identify bottlenecks and optimization opportunities
// Target: <1% profiling overhead

use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileType {
    CPU,
    Memory,
    Thermal,
    Battery,
    GPU,
    Network,
}

#[derive(Clone, Debug)]
pub struct CPUProfile {
    pub usage_percent: f64,
    pub freq_mhz: u32,
    pub core_count: usize,
    pub throttling: bool,
    pub samples: usize,
}

#[derive(Clone, Debug)]
pub struct MemoryProfile {
    pub used_mb: f64,
    pub available_mb: f64,
    pub pressure_percent: f64,
    pub page_faults: u64,
    pub oom_events: usize,
}

#[derive(Clone, Debug)]
pub struct ThermalProfile {
    pub max_temp: f64,
    pub avg_temp: f64,
    pub hottest_zone: String,
    pub throttle_events: usize,
    pub emergency_events: usize,
}

#[derive(Clone, Debug)]
pub struct BatteryProfile {
    pub drain_rate_mw: f64,
    pub estimated_hours: f64,
    pub temp: f64,
    pub health_percent: f64,
    pub cycle_count: u32,
}

#[derive(Clone, Debug)]
pub struct GPUProfile {
    pub utilization_percent: f64,
    pub freq_mhz: u32,
    pub fps: f64,
    pub frame_drops: u64,
    pub power_mw: f64,
}

pub struct PerformanceProfiler {
    // Profile history
    cpu_samples: VecDeque<CPUProfile>,
    memory_samples: VecDeque<MemoryProfile>,
    thermal_samples: VecDeque<ThermalProfile>,
    battery_samples: VecDeque<BatteryProfile>,
    gpu_samples: VecDeque<GPUProfile>,

    // Bottleneck detection
    cpu_bottlenecks: Vec<Bottleneck>,
    memory_bottlenecks: Vec<Bottleneck>,
    thermal_bottlenecks: Vec<Bottleneck>,

    // Optimization opportunities
    optimization_score: f64,
    opportunities: Vec<OptimizationOpportunity>,

    // Statistics
    profiling_overhead_percent: f64,
    total_profiles: usize,
}

#[derive(Clone, Debug)]
pub struct Bottleneck {
    pub severity: f64,  // 0.0-1.0
    pub description: String,
    pub recommendation: String,
}

#[derive(Clone, Debug)]
pub struct OptimizationOpportunity {
    pub potential_gain: f64,  // mW or ms improvement
    pub difficulty: f64,      // 0.0-1.0 (easy to hard)
    pub description: String,
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            cpu_samples: VecDeque::with_capacity(100),
            memory_samples: VecDeque::with_capacity(100),
            thermal_samples: VecDeque::with_capacity(100),
            battery_samples: VecDeque::with_capacity(100),
            gpu_samples: VecDeque::with_capacity(100),

            cpu_bottlenecks: Vec::new(),
            memory_bottlenecks: Vec::new(),
            thermal_bottlenecks: Vec::new(),

            optimization_score: 0.0,
            opportunities: Vec::new(),

            profiling_overhead_percent: 0.0,
            total_profiles: 0,
        }
    }

    /// Record CPU profile
    pub fn record_cpu(&mut self, profile: CPUProfile) {
        self.cpu_samples.push_back(profile);
        if self.cpu_samples.len() > 100 {
            self.cpu_samples.pop_front();
        }

        self.analyze_cpu_bottlenecks();
        self.total_profiles += 1;
    }

    /// Record Memory profile
    pub fn record_memory(&mut self, profile: MemoryProfile) {
        self.memory_samples.push_back(profile);
        if self.memory_samples.len() > 100 {
            self.memory_samples.pop_front();
        }

        self.analyze_memory_bottlenecks();
        self.total_profiles += 1;
    }

    /// Record Thermal profile
    pub fn record_thermal(&mut self, profile: ThermalProfile) {
        self.thermal_samples.push_back(profile);
        if self.thermal_samples.len() > 100 {
            self.thermal_samples.pop_front();
        }

        self.analyze_thermal_bottlenecks();
        self.total_profiles += 1;
    }

    /// Record Battery profile
    pub fn record_battery(&mut self, profile: BatteryProfile) {
        self.battery_samples.push_back(profile);
        if self.battery_samples.len() > 100 {
            self.battery_samples.pop_front();
        }

        self.total_profiles += 1;
    }

    /// Record GPU profile
    pub fn record_gpu(&mut self, profile: GPUProfile) {
        self.gpu_samples.push_back(profile);
        if self.gpu_samples.len() > 100 {
            self.gpu_samples.pop_front();
        }

        self.total_profiles += 1;
    }

    fn analyze_cpu_bottlenecks(&mut self) {
        self.cpu_bottlenecks.clear();

        if self.cpu_samples.is_empty() {
            return;
        }

        // Analyze usage patterns
        let avg_usage: f64 = self.cpu_samples.iter().map(|s| s.usage_percent).sum::<f64>()
            / self.cpu_samples.len() as f64;

        if avg_usage > 80.0 {
            self.cpu_bottlenecks.push(Bottleneck {
                severity: 0.8,
                description: "High CPU usage (80%+)".to_string(),
                recommendation: "Profile app, optimize algorithms, reduce frame rate".to_string(),
            });
        }

        // Check throttling
        let throttle_count = self
            .cpu_samples
            .iter()
            .filter(|s| s.throttling)
            .count();

        if throttle_count as f64 / self.cpu_samples.len() as f64 > 0.3 {
            self.cpu_bottlenecks.push(Bottleneck {
                severity: 0.7,
                description: "CPU throttling detected (30%+ of samples)".to_string(),
                recommendation: "Reduce workload or improve thermal cooling".to_string(),
            });
        }
    }

    fn analyze_memory_bottlenecks(&mut self) {
        self.memory_bottlenecks.clear();

        if self.memory_samples.is_empty() {
            return;
        }

        let latest = self.memory_samples.back().unwrap();

        if latest.pressure_percent > 80.0 {
            self.memory_bottlenecks.push(Bottleneck {
                severity: 0.9,
                description: format!("Memory pressure {:.0}%", latest.pressure_percent),
                recommendation: "Release cached data, reduce background apps, enable lowpower mode".to_string(),
            });
        }

        // Check for memory leaks (monotonic increase)
        if self.memory_samples.len() > 20 {
            let older = self.memory_samples.get(0).unwrap();
            let newer = self.memory_samples.back().unwrap();

            let growth = newer.used_mb - older.used_mb;
            if growth > 500.0 {
                // 500MB growth over 100 samples
                self.memory_bottlenecks.push(Bottleneck {
                    severity: 0.6,
                    description: format!("Possible memory leak ({:.0}MB growth)", growth),
                    recommendation: "Check for unreleased resources, restart app".to_string(),
                });
            }
        }
    }

    fn analyze_thermal_bottlenecks(&mut self) {
        self.thermal_bottlenecks.clear();

        if self.thermal_samples.is_empty() {
            return;
        }

        let latest = self.thermal_samples.back().unwrap();

        if latest.max_temp > 55.0 {
            self.thermal_bottlenecks.push(Bottleneck {
                severity: 0.95,
                description: format!("Critical temperature {:.1}°C", latest.max_temp),
                recommendation: "Reduce load immediately, enable active cooling".to_string(),
            });
        } else if latest.max_temp > 50.0 {
            self.thermal_bottlenecks.push(Bottleneck {
                severity: 0.7,
                description: format!("High temperature {:.1}°C", latest.max_temp),
                recommendation: "Monitor thermal trends, reduce background tasks".to_string(),
            });
        }

        if latest.throttle_events > 5 {
            self.thermal_bottlenecks.push(Bottleneck {
                severity: 0.6,
                description: format!("{} throttle events", latest.throttle_events),
                recommendation: "Improve thermal management, check device ventilation".to_string(),
            });
        }
    }

    /// Identify optimization opportunities
    pub fn identify_opportunities(&mut self) {
        self.opportunities.clear();

        // CPU optimization
        if !self.cpu_samples.is_empty() {
            let avg_usage: f64 = self.cpu_samples.iter().map(|s| s.usage_percent).sum::<f64>()
                / self.cpu_samples.len() as f64;

            if avg_usage < 20.0 {
                self.opportunities.push(OptimizationOpportunity {
                    potential_gain: 50.0,  // mW
                    difficulty: 0.2,
                    description: "Reduce CPU frequency for low utilization".to_string(),
                });
            }
        }

        // Memory optimization
        if !self.memory_samples.is_empty() {
            let latest = self.memory_samples.back().unwrap();

            if latest.pressure_percent > 70.0 {
                self.opportunities.push(OptimizationOpportunity {
                    potential_gain: 100.0,
                    difficulty: 0.4,
                    description: "Aggressive memory cleanup, reduce caches".to_string(),
                });
            }
        }

        // Thermal optimization
        if !self.thermal_samples.is_empty() {
            let latest = self.thermal_samples.back().unwrap();

            if latest.throttle_events > 0 {
                self.opportunities.push(OptimizationOpportunity {
                    potential_gain: 200.0,  // mW from better thermal management
                    difficulty: 0.6,
                    description: "Implement predictive thermal throttling".to_string(),
                });
            }
        }

        // Calculate optimization score
        self.calculate_optimization_score();
    }

    fn calculate_optimization_score(&mut self) {
        // Score based on:
        // 1. Absence of bottlenecks
        // 2. Availability of opportunities
        // 3. Current efficiency

        let bottleneck_count = (self.cpu_bottlenecks.len() + self.memory_bottlenecks.len()
            + self.thermal_bottlenecks.len()) as f64;

        let opportunity_count = self.opportunities.len() as f64;

        // Score 0.0-1.0: 1.0 = perfectly optimized
        let bottleneck_score = (1.0 - (bottleneck_count / 10.0)).max(0.0);
        let opportunity_score = (1.0 - (opportunity_count / 10.0)).max(0.0);

        self.optimization_score = (bottleneck_score * 0.6 + opportunity_score * 0.4).min(1.0);
    }

    /// Get profiling report
    pub fn get_report(&self) -> ProfilingReport {
        ProfilingReport {
            total_profiles: self.total_profiles,
            cpu_bottlenecks: self.cpu_bottlenecks.clone(),
            memory_bottlenecks: self.memory_bottlenecks.clone(),
            thermal_bottlenecks: self.thermal_bottlenecks.clone(),
            optimization_opportunities: self.opportunities.clone(),
            optimization_score: self.optimization_score,
            profiling_overhead: self.profiling_overhead_percent,
        }
    }

    /// Get current CPU average
    pub fn get_cpu_average(&self) -> Option<f64> {
        if self.cpu_samples.is_empty() {
            None
        } else {
            let sum: f64 = self.cpu_samples.iter().map(|s| s.usage_percent).sum();
            Some(sum / self.cpu_samples.len() as f64)
        }
    }

    /// Get current memory pressure
    pub fn get_memory_pressure(&self) -> Option<f64> {
        self.memory_samples.back().map(|m| m.pressure_percent)
    }

    /// Get current temperature
    pub fn get_current_temperature(&self) -> Option<f64> {
        self.thermal_samples.back().map(|t| t.max_temp)
    }

    /// Reset all profiles
    pub fn reset(&mut self) {
        self.cpu_samples.clear();
        self.memory_samples.clear();
        self.thermal_samples.clear();
        self.battery_samples.clear();
        self.gpu_samples.clear();
        self.cpu_bottlenecks.clear();
        self.memory_bottlenecks.clear();
        self.thermal_bottlenecks.clear();
        self.opportunities.clear();
        self.total_profiles = 0;
    }
}

#[derive(Clone, Debug)]
pub struct ProfilingReport {
    pub total_profiles: usize,
    pub cpu_bottlenecks: Vec<Bottleneck>,
    pub memory_bottlenecks: Vec<Bottleneck>,
    pub thermal_bottlenecks: Vec<Bottleneck>,
    pub optimization_opportunities: Vec<OptimizationOpportunity>,
    pub optimization_score: f64,
    pub profiling_overhead: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let profiler = PerformanceProfiler::new();
        assert_eq!(profiler.total_profiles, 0);
        assert_eq!(profiler.optimization_score, 0.0);
    }

    #[test]
    fn test_cpu_recording() {
        let mut profiler = PerformanceProfiler::new();

        let profile = CPUProfile {
            usage_percent: 45.0,
            freq_mhz: 1500,
            core_count: 8,
            throttling: false,
            samples: 100,
        };

        profiler.record_cpu(profile);
        assert_eq!(profiler.total_profiles, 1);
        assert_eq!(profiler.cpu_samples.len(), 1);
    }

    #[test]
    fn test_cpu_bottleneck_detection() {
        let mut profiler = PerformanceProfiler::new();

        for _ in 0..20 {
            profiler.record_cpu(CPUProfile {
                usage_percent: 85.0,  // High usage
                freq_mhz: 3400,
                core_count: 8,
                throttling: true,
                samples: 100,
            });
        }

        assert!(!profiler.cpu_bottlenecks.is_empty());
    }

    #[test]
    fn test_memory_bottleneck_detection() {
        let mut profiler = PerformanceProfiler::new();

        let profile = MemoryProfile {
            used_mb: 3800.0,
            available_mb: 256.0,
            pressure_percent: 93.0,
            page_faults: 10000,
            oom_events: 2,
        };

        profiler.record_memory(profile);
        assert!(!profiler.memory_bottlenecks.is_empty());
    }

    #[test]
    fn test_optimization_score() {
        let mut profiler = PerformanceProfiler::new();

        profiler.record_cpu(CPUProfile {
            usage_percent: 30.0,
            freq_mhz: 1500,
            core_count: 8,
            throttling: false,
            samples: 100,
        });

        profiler.identify_opportunities();
        assert!(profiler.optimization_score > 0.0);
    }

    #[test]
    fn test_get_report() {
        let mut profiler = PerformanceProfiler::new();

        profiler.record_cpu(CPUProfile {
            usage_percent: 45.0,
            freq_mhz: 1500,
            core_count: 8,
            throttling: false,
            samples: 100,
        });

        let report = profiler.get_report();
        assert_eq!(report.total_profiles, 1);
    }

    #[test]
    fn test_get_cpu_average() {
        let mut profiler = PerformanceProfiler::new();

        profiler.record_cpu(CPUProfile {
            usage_percent: 40.0,
            freq_mhz: 1500,
            core_count: 8,
            throttling: false,
            samples: 100,
        });

        profiler.record_cpu(CPUProfile {
            usage_percent: 60.0,
            freq_mhz: 2400,
            core_count: 8,
            throttling: false,
            samples: 100,
        });

        let avg = profiler.get_cpu_average();
        assert!(avg.is_some());
        assert_eq!(avg.unwrap(), 50.0);
    }

    #[test]
    fn test_reset() {
        let mut profiler = PerformanceProfiler::new();

        profiler.record_cpu(CPUProfile {
            usage_percent: 45.0,
            freq_mhz: 1500,
            core_count: 8,
            throttling: false,
            samples: 100,
        });

        assert!(profiler.total_profiles > 0);

        profiler.reset();
        assert_eq!(profiler.total_profiles, 0);
        assert_eq!(profiler.cpu_samples.len(), 0);
    }
}
