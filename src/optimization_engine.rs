// Project Sovereign: Optimization Engine Module
// Goal: Automatic performance/power/thermal optimization
// Target: <10ms per optimization cycle, <5% false positive rate

use crate::performance_profiler::{Bottleneck, OptimizationOpportunity, ProfilingReport};
use crate::system_adaptation::{
    AdaptiveScheduler, PowerOptimizer, ThermalController,
    BatteryMode, ThermalState, WorkloadClass,
};
use crate::predictive_preload::PreloadPriority;
use crate::cpu_frequency::CPUFrequency;
use crate::gpu_control::GPUFrequency;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptimizationStrategy {
    /// Minimize power consumption
    PowerSaving,
    /// Balance performance and power
    Balanced,
    /// Maximize performance
    Performance,
    /// Thermal crisis mitigation
    ThermalControl,
    /// Memory conservation
    MemoryOptimization,
}

#[derive(Clone, Debug)]
pub struct OptimizationAction {
    pub strategy: OptimizationStrategy,
    pub cpu_frequency: Option<CPUFrequency>,
    pub gpu_frequency: Option<GPUFrequency>,
    pub reduce_memory: bool,
    pub cleanup_cache: bool,
    pub reduce_background_apps: bool,
    pub adjust_preload_priority: bool,
    pub new_preload_priority: PreloadPriority,
    pub thermal_throttle_percent: f64,
    pub estimated_power_savings_mw: f64,
    pub estimated_latency_impact_ms: f64,
}

#[derive(Clone, Debug)]
pub struct OptimizationMetrics {
    pub total_optimizations: usize,
    pub successful_optimizations: usize,
    pub average_power_savings_mw: f64,
    pub average_latency_impact_ms: f64,
    pub false_positive_rate: f64,
    pub effectiveness_score: f64,  // 0.0-1.0
}

pub struct OptimizationEngine {
    // Optimization history
    actions_taken: Vec<OptimizationAction>,
    optimization_results: Vec<(OptimizationAction, bool)>,  // (action, success)

    // Performance metrics
    total_optimizations: usize,
    successful_optimizations: usize,
    false_positives: usize,

    // Power tracking
    total_power_savings: f64,
    total_latency_impact: f64,

    // Thresholds
    power_threshold_mw: f64,
    thermal_threshold_celsius: f64,
    memory_pressure_threshold: f64,
}

impl OptimizationEngine {
    pub fn new() -> Self {
        Self {
            actions_taken: Vec::new(),
            optimization_results: Vec::new(),

            total_optimizations: 0,
            successful_optimizations: 0,
            false_positives: 0,

            total_power_savings: 0.0,
            total_latency_impact: 0.0,

            power_threshold_mw: 800.0,      // >800mW = optimize
            thermal_threshold_celsius: 45.0, // >45°C = thermal optimization
            memory_pressure_threshold: 70.0, // >70% = memory optimization
        }
    }

    /// Analyze profiling report and generate optimization actions
    pub fn analyze_and_optimize(&mut self, report: &ProfilingReport, current_mode: &str) -> OptimizationAction {
        let mut actions = Vec::new();

        // Analyze bottlenecks
        if !report.cpu_bottlenecks.is_empty() {
            actions.push(self.optimize_cpu_bottleneck(&report.cpu_bottlenecks));
        }

        if !report.memory_bottlenecks.is_empty() {
            actions.push(self.optimize_memory_bottleneck(&report.memory_bottlenecks));
        }

        if !report.thermal_bottlenecks.is_empty() {
            actions.push(self.optimize_thermal_bottleneck(&report.thermal_bottlenecks));
        }

        // Analyze opportunities
        if !report.optimization_opportunities.is_empty() {
            actions.push(self.exploit_opportunities(&report.optimization_opportunities));
        }

        // Merge actions into single decision
        let final_action = self.merge_actions(actions, current_mode);
        self.actions_taken.push(final_action.clone());
        self.total_optimizations += 1;

        final_action
    }

    fn optimize_cpu_bottleneck(&self, bottlenecks: &[Bottleneck]) -> OptimizationAction {
        let severity = bottlenecks.iter().map(|b| b.severity).sum::<f64>() / bottlenecks.len() as f64;

        OptimizationAction {
            strategy: OptimizationStrategy::PowerSaving,
            cpu_frequency: if severity > 0.7 {
                Some(CPUFrequency::Conservative)
            } else {
                Some(CPUFrequency::Moderate)
            },
            gpu_frequency: None,
            reduce_memory: false,
            cleanup_cache: false,
            reduce_background_apps: true,
            adjust_preload_priority: false,
            new_preload_priority: PreloadPriority::Low,
            thermal_throttle_percent: severity * 20.0,
            estimated_power_savings_mw: severity * 150.0,
            estimated_latency_impact_ms: severity * 5.0,
        }
    }

    fn optimize_memory_bottleneck(&self, bottlenecks: &[Bottleneck]) -> OptimizationAction {
        let severity = bottlenecks.iter().map(|b| b.severity).sum::<f64>() / bottlenecks.len() as f64;

        OptimizationAction {
            strategy: OptimizationStrategy::MemoryOptimization,
            cpu_frequency: None,
            gpu_frequency: None,
            reduce_memory: true,
            cleanup_cache: true,
            reduce_background_apps: severity > 0.5,
            adjust_preload_priority: true,
            new_preload_priority: if severity > 0.7 {
                PreloadPriority::Low
            } else {
                PreloadPriority::Medium
            },
            thermal_throttle_percent: 0.0,
            estimated_power_savings_mw: 50.0 + severity * 100.0,
            estimated_latency_impact_ms: 2.0,
        }
    }

    fn optimize_thermal_bottleneck(&self, bottlenecks: &[Bottleneck]) -> OptimizationAction {
        let severity = bottlenecks.iter().map(|b| b.severity).sum::<f64>() / bottlenecks.len() as f64;

        let throttle_level = if severity > 0.9 {
            50.0  // Emergency: 50% throttle
        } else if severity > 0.7 {
            30.0  // High temperature: 30% throttle
        } else {
            15.0  // Moderate: 15% throttle
        };

        OptimizationAction {
            strategy: OptimizationStrategy::ThermalControl,
            cpu_frequency: if severity > 0.8 {
                Some(CPUFrequency::Conservative)
            } else if severity > 0.6 {
                Some(CPUFrequency::Moderate)
            } else {
                Some(CPUFrequency::Balanced)
            },
            gpu_frequency: if severity > 0.7 {
                Some(GPUFrequency::Low)
            } else {
                Some(GPUFrequency::Medium)
            },
            reduce_memory: false,
            cleanup_cache: false,
            reduce_background_apps: severity > 0.7,
            adjust_preload_priority: false,
            new_preload_priority: PreloadPriority::Low,
            thermal_throttle_percent: throttle_level,
            estimated_power_savings_mw: severity * 200.0,
            estimated_latency_impact_ms: severity * 10.0,
        }
    }

    fn exploit_opportunities(&self, opportunities: &[OptimizationOpportunity]) -> OptimizationAction {
        let avg_potential = opportunities.iter().map(|o| o.potential_gain).sum::<f64>()
            / opportunities.len() as f64;

        let avg_difficulty = opportunities.iter().map(|o| o.difficulty).sum::<f64>()
            / opportunities.len() as f64;

        // Pursue high-gain, low-difficulty opportunities
        let should_pursue = avg_potential > 100.0 && avg_difficulty < 0.5;

        if should_pursue {
            OptimizationAction {
                strategy: OptimizationStrategy::PowerSaving,
                cpu_frequency: Some(CPUFrequency::Moderate),
                gpu_frequency: None,
                reduce_memory: true,
                cleanup_cache: true,
                reduce_background_apps: false,
                adjust_preload_priority: true,
                new_preload_priority: PreloadPriority::Critical,
                thermal_throttle_percent: 5.0,
                estimated_power_savings_mw: avg_potential * 0.7,
                estimated_latency_impact_ms: 0.0,
            }
        } else {
            // Conservative approach
            OptimizationAction {
                strategy: OptimizationStrategy::Balanced,
                cpu_frequency: None,
                gpu_frequency: None,
                reduce_memory: false,
                cleanup_cache: false,
                reduce_background_apps: false,
                adjust_preload_priority: false,
                new_preload_priority: PreloadPriority::Medium,
                thermal_throttle_percent: 0.0,
                estimated_power_savings_mw: 0.0,
                estimated_latency_impact_ms: 0.0,
            }
        }
    }

    fn merge_actions(&self, actions: Vec<OptimizationAction>, current_mode: &str) -> OptimizationAction {
        if actions.is_empty() {
            return OptimizationAction {
                strategy: OptimizationStrategy::Balanced,
                cpu_frequency: None,
                gpu_frequency: None,
                reduce_memory: false,
                cleanup_cache: false,
                reduce_background_apps: false,
                adjust_preload_priority: false,
                new_preload_priority: PreloadPriority::Medium,
                thermal_throttle_percent: 0.0,
                estimated_power_savings_mw: 0.0,
                estimated_latency_impact_ms: 0.0,
            };
        }

        // Select highest-priority strategy
        let strategy = if actions.iter().any(|a| a.strategy == OptimizationStrategy::ThermalControl) {
            OptimizationStrategy::ThermalControl
        } else if actions.iter().any(|a| a.strategy == OptimizationStrategy::MemoryOptimization) {
            OptimizationStrategy::MemoryOptimization
        } else if current_mode == "performance" {
            OptimizationStrategy::Performance
        } else {
            OptimizationStrategy::PowerSaving
        };

        // Merge frequency recommendations
        let cpu_freq = actions.iter().find_map(|a| a.cpu_frequency);
        let gpu_freq = actions.iter().find_map(|a| a.gpu_frequency);

        let reduce_memory = actions.iter().any(|a| a.reduce_memory);
        let cleanup_cache = actions.iter().any(|a| a.cleanup_cache);
        let reduce_bg_apps = actions.iter().any(|a| a.reduce_background_apps);
        let adjust_preload = actions.iter().any(|a| a.adjust_preload_priority);

        // Maximum preload priority (Critical > High > Medium > Low)
        let preload_priority = *actions
            .iter()
            .map(|a| &a.new_preload_priority)
            .max()
            .unwrap_or(&PreloadPriority::Medium);

        // Sum thermal throttling (take maximum)
        let thermal_throttle = actions.iter().map(|a| a.thermal_throttle_percent).fold(0.0, f64::max);

        // Sum power savings
        let power_savings: f64 = actions.iter().map(|a| a.estimated_power_savings_mw).sum();

        // Sum latency impact
        let latency_impact: f64 = actions.iter().map(|a| a.estimated_latency_impact_ms).sum();

        OptimizationAction {
            strategy,
            cpu_frequency: cpu_freq,
            gpu_frequency: gpu_freq,
            reduce_memory,
            cleanup_cache,
            reduce_background_apps: reduce_bg_apps,
            adjust_preload_priority: adjust_preload,
            new_preload_priority: preload_priority,
            thermal_throttle_percent: thermal_throttle,
            estimated_power_savings_mw: power_savings,
            estimated_latency_impact_ms: latency_impact,
        }
    }

    /// Track optimization result for feedback
    pub fn record_optimization_result(&mut self, action: OptimizationAction, was_successful: bool) {
        self.optimization_results.push((action.clone(), was_successful));

        if was_successful {
            self.successful_optimizations += 1;
            self.total_power_savings += action.estimated_power_savings_mw;
        } else {
            self.false_positives += 1;
        }

        self.total_latency_impact += action.estimated_latency_impact_ms;
    }

    /// Get current optimization metrics
    pub fn get_metrics(&self) -> OptimizationMetrics {
        let avg_power = if self.successful_optimizations > 0 {
            self.total_power_savings / self.successful_optimizations as f64
        } else {
            0.0
        };

        let avg_latency = if self.total_optimizations > 0 {
            self.total_latency_impact / self.total_optimizations as f64
        } else {
            0.0
        };

        let false_positive_rate = if self.total_optimizations > 0 {
            (self.false_positives as f64 / self.total_optimizations as f64) * 100.0
        } else {
            0.0
        };

        // Effectiveness: successful_rate * (1 - false_positive_penalty)
        let success_rate = if self.total_optimizations > 0 {
            self.successful_optimizations as f64 / self.total_optimizations as f64
        } else {
            0.0
        };

        let effectiveness = (success_rate * 0.7 + (1.0 - false_positive_rate / 100.0) * 0.3).max(0.0).min(1.0);

        OptimizationMetrics {
            total_optimizations: self.total_optimizations,
            successful_optimizations: self.successful_optimizations,
            average_power_savings_mw: avg_power,
            average_latency_impact_ms: avg_latency,
            false_positive_rate,
            effectiveness_score: effectiveness,
        }
    }

    /// Reset optimization engine state
    pub fn reset(&mut self) {
        self.actions_taken.clear();
        self.optimization_results.clear();
        self.total_optimizations = 0;
        self.successful_optimizations = 0;
        self.false_positives = 0;
        self.total_power_savings = 0.0;
        self.total_latency_impact = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = OptimizationEngine::new();
        assert_eq!(engine.total_optimizations, 0);
    }

    #[test]
    fn test_cpu_bottleneck_optimization() {
        let engine = OptimizationEngine::new();

        let bottleneck = Bottleneck {
            severity: 0.8,
            description: "High CPU usage".to_string(),
            recommendation: "Reduce workload".to_string(),
        };

        let action = engine.optimize_cpu_bottleneck(&[bottleneck]);
        assert_eq!(action.strategy, OptimizationStrategy::PowerSaving);
        assert!(action.reduce_background_apps);
        assert!(action.estimated_power_savings_mw > 100.0);
    }

    #[test]
    fn test_memory_bottleneck_optimization() {
        let engine = OptimizationEngine::new();

        let bottleneck = Bottleneck {
            severity: 0.8,
            description: "High memory pressure".to_string(),
            recommendation: "Clean cache".to_string(),
        };

        let action = engine.optimize_memory_bottleneck(&[bottleneck]);
        assert_eq!(action.strategy, OptimizationStrategy::MemoryOptimization);
        assert!(action.reduce_memory);
        assert!(action.cleanup_cache);
    }

    #[test]
    fn test_thermal_bottleneck_optimization() {
        let engine = OptimizationEngine::new();

        let bottleneck = Bottleneck {
            severity: 0.95,
            description: "Critical temperature".to_string(),
            recommendation: "Emergency throttle".to_string(),
        };

        let action = engine.optimize_thermal_bottleneck(&[bottleneck]);
        assert_eq!(action.strategy, OptimizationStrategy::ThermalControl);
        assert!(action.thermal_throttle_percent > 40.0);
    }

    #[test]
    fn test_opportunity_exploitation() {
        let engine = OptimizationEngine::new();

        let opportunity = OptimizationOpportunity {
            potential_gain: 200.0,
            difficulty: 0.3,
            description: "Easy optimization".to_string(),
        };

        let action = engine.exploit_opportunities(&[opportunity]);
        assert_eq!(action.strategy, OptimizationStrategy::PowerSaving);
        assert!(action.cleanup_cache);
    }

    #[test]
    fn test_merge_actions_empty() {
        let engine = OptimizationEngine::new();
        let action = engine.merge_actions(vec![], "idle");

        assert_eq!(action.strategy, OptimizationStrategy::Balanced);
        assert_eq!(action.estimated_power_savings_mw, 0.0);
    }

    #[test]
    fn test_merge_actions_priority() {
        let engine = OptimizationEngine::new();

        let thermal_action = OptimizationAction {
            strategy: OptimizationStrategy::ThermalControl,
            cpu_frequency: None,
            gpu_frequency: None,
            reduce_memory: false,
            cleanup_cache: false,
            reduce_background_apps: false,
            adjust_preload_priority: false,
            new_preload_priority: PreloadPriority::Medium,
            thermal_throttle_percent: 20.0,
            estimated_power_savings_mw: 100.0,
            estimated_latency_impact_ms: 0.0,
        };

        let power_action = OptimizationAction {
            strategy: OptimizationStrategy::PowerSaving,
            cpu_frequency: None,
            gpu_frequency: None,
            reduce_memory: false,
            cleanup_cache: false,
            reduce_background_apps: false,
            adjust_preload_priority: false,
            new_preload_priority: PreloadPriority::Medium,
            thermal_throttle_percent: 0.0,
            estimated_power_savings_mw: 50.0,
            estimated_latency_impact_ms: 0.0,
        };

        let merged = engine.merge_actions(vec![thermal_action, power_action], "idle");
        assert_eq!(merged.strategy, OptimizationStrategy::ThermalControl);
        assert_eq!(merged.thermal_throttle_percent, 20.0);
        assert_eq!(merged.estimated_power_savings_mw, 150.0);
    }

    #[test]
    fn test_optimization_result_tracking() {
        let mut engine = OptimizationEngine::new();

        let action = OptimizationAction {
            strategy: OptimizationStrategy::PowerSaving,
            cpu_frequency: Some(CPUFrequency::Conservative),
            gpu_frequency: None,
            reduce_memory: false,
            cleanup_cache: false,
            reduce_background_apps: false,
            adjust_preload_priority: false,
            new_preload_priority: PreloadPriority::Medium,
            thermal_throttle_percent: 0.0,
            estimated_power_savings_mw: 150.0,
            estimated_latency_impact_ms: 2.0,
        };

        engine.total_optimizations = 1;
        engine.record_optimization_result(action.clone(), true);

        assert_eq!(engine.successful_optimizations, 1);
        assert_eq!(engine.total_power_savings, 150.0);
    }

    #[test]
    fn test_optimization_metrics() {
        let mut engine = OptimizationEngine::new();

        for _ in 0..10 {
            let action = OptimizationAction {
                strategy: OptimizationStrategy::PowerSaving,
                cpu_frequency: None,
                gpu_frequency: None,
                reduce_memory: false,
                cleanup_cache: false,
                reduce_background_apps: false,
                adjust_preload_priority: false,
                new_preload_priority: PreloadPriority::Medium,
                thermal_throttle_percent: 0.0,
                estimated_power_savings_mw: 100.0,
                estimated_latency_impact_ms: 1.0,
            };

            engine.total_optimizations += 1;
            engine.record_optimization_result(action, true);
        }

        let metrics = engine.get_metrics();
        assert_eq!(metrics.total_optimizations, 10);
        assert_eq!(metrics.successful_optimizations, 10);
        assert!(metrics.effectiveness_score > 0.9);
    }

    #[test]
    fn test_false_positive_detection() {
        let mut engine = OptimizationEngine::new();

        let action = OptimizationAction {
            strategy: OptimizationStrategy::PowerSaving,
            cpu_frequency: None,
            gpu_frequency: None,
            reduce_memory: false,
            cleanup_cache: false,
            reduce_background_apps: false,
            adjust_preload_priority: false,
            new_preload_priority: PreloadPriority::Medium,
            thermal_throttle_percent: 0.0,
            estimated_power_savings_mw: 100.0,
            estimated_latency_impact_ms: 0.0,
        };

        for _ in 0..5 {
            engine.total_optimizations += 1;
            engine.record_optimization_result(action.clone(), true);
        }

        for _ in 0..5 {
            engine.total_optimizations += 1;
            engine.record_optimization_result(action.clone(), false);
        }

        let metrics = engine.get_metrics();
        assert_eq!(metrics.total_optimizations, 10);
        assert_eq!(metrics.false_positive_rate, 50.0);
    }

    #[test]
    fn test_reset_engine() {
        let mut engine = OptimizationEngine::new();
        engine.total_optimizations = 10;
        engine.successful_optimizations = 8;

        engine.reset();

        assert_eq!(engine.total_optimizations, 0);
        assert_eq!(engine.successful_optimizations, 0);
        assert_eq!(engine.false_positives, 0);
    }

    #[test]
    fn test_strategy_selection_performance_mode() {
        let engine = OptimizationEngine::new();

        let thermal_action = OptimizationAction {
            strategy: OptimizationStrategy::ThermalControl,
            cpu_frequency: None,
            gpu_frequency: None,
            reduce_memory: false,
            cleanup_cache: false,
            reduce_background_apps: false,
            adjust_preload_priority: false,
            new_preload_priority: PreloadPriority::Medium,
            thermal_throttle_percent: 0.0,
            estimated_power_savings_mw: 0.0,
            estimated_latency_impact_ms: 0.0,
        };

        let power_action = OptimizationAction {
            strategy: OptimizationStrategy::PowerSaving,
            cpu_frequency: None,
            gpu_frequency: None,
            reduce_memory: false,
            cleanup_cache: false,
            reduce_background_apps: false,
            adjust_preload_priority: false,
            new_preload_priority: PreloadPriority::Medium,
            thermal_throttle_percent: 0.0,
            estimated_power_savings_mw: 0.0,
            estimated_latency_impact_ms: 0.0,
        };

        let merged = engine.merge_actions(vec![power_action], "performance");
        assert_eq!(merged.strategy, OptimizationStrategy::Performance);
    }

    #[test]
    fn test_preload_priority_merging() {
        let engine = OptimizationEngine::new();

        let action1 = OptimizationAction {
            strategy: OptimizationStrategy::PowerSaving,
            cpu_frequency: None,
            gpu_frequency: None,
            reduce_memory: false,
            cleanup_cache: false,
            reduce_background_apps: false,
            adjust_preload_priority: false,
            new_preload_priority: PreloadPriority::Low,
            thermal_throttle_percent: 0.0,
            estimated_power_savings_mw: 0.0,
            estimated_latency_impact_ms: 0.0,
        };

        let action2 = OptimizationAction {
            strategy: OptimizationStrategy::PowerSaving,
            cpu_frequency: None,
            gpu_frequency: None,
            reduce_memory: false,
            cleanup_cache: false,
            reduce_background_apps: false,
            adjust_preload_priority: false,
            new_preload_priority: PreloadPriority::Critical,
            thermal_throttle_percent: 0.0,
            estimated_power_savings_mw: 0.0,
            estimated_latency_impact_ms: 0.0,
        };

        let merged = engine.merge_actions(vec![action1, action2], "idle");
        assert_eq!(merged.new_preload_priority, PreloadPriority::Critical);
    }

    #[test]
    fn test_thermal_emergency_handling() {
        let engine = OptimizationEngine::new();

        let bottleneck = Bottleneck {
            severity: 0.99,
            description: "Emergency temperature".to_string(),
            recommendation: "Maximum throttle".to_string(),
        };

        let action = engine.optimize_thermal_bottleneck(&[bottleneck]);
        assert!(action.thermal_throttle_percent > 45.0);
        assert!(action.reduce_background_apps);
    }

    #[test]
    fn test_cpu_frequency_selection() {
        let engine = OptimizationEngine::new();

        // High severity
        let high_sev = Bottleneck {
            severity: 0.8,
            description: "High CPU".to_string(),
            recommendation: "Reduce".to_string(),
        };

        let action = engine.optimize_cpu_bottleneck(&[high_sev]);
        assert_eq!(action.cpu_frequency, Some(CPUFrequency::Conservative));

        // Low severity
        let low_sev = Bottleneck {
            severity: 0.4,
            description: "Medium CPU".to_string(),
            recommendation: "Monitor".to_string(),
        };

        let action = engine.optimize_cpu_bottleneck(&[low_sev]);
        assert_eq!(action.cpu_frequency, Some(CPUFrequency::Moderate));
    }

    #[test]
    fn test_memory_reduction_scaling() {
        let engine = OptimizationEngine::new();

        let low_sev = Bottleneck {
            severity: 0.3,
            description: "Mild pressure".to_string(),
            recommendation: "Monitor".to_string(),
        };

        let action = engine.optimize_memory_bottleneck(&[low_sev]);
        assert_eq!(action.new_preload_priority, PreloadPriority::Medium);

        let high_sev = Bottleneck {
            severity: 0.9,
            description: "Critical pressure".to_string(),
            recommendation: "Reduce".to_string(),
        };

        let action = engine.optimize_memory_bottleneck(&[high_sev]);
        assert_eq!(action.new_preload_priority, PreloadPriority::Low);
    }
}
