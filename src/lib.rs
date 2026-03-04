// Project Sovereign: Self-Learning Intelligent Phone OS
// L4 Intelligence Layer + L3 Hardware Integration

pub mod user_behavior_model;
pub mod system_adaptation;
pub mod predictive_preload;
pub mod anomaly_detection;
pub mod cpu_frequency;
pub mod power_domains;
pub mod thermal_management;
pub mod gpu_control;

pub use user_behavior_model::{
    UserBehaviorModel, UserEvent, Location, LocationType, AppCategory,
    TimeSlotProfile, LocationProfile, AppProfile,
};

pub use system_adaptation::{
    AdaptiveScheduler, PowerOptimizer, ThermalController,
    CoreAssignment, BatteryMode, ThermalState, WorkloadClass, NetworkState, ScreenState,
};

pub use predictive_preload::{
    PredictivePreload, AppPreloadProfile, PreloadState, PreloadPriority,
    WiFiPrediction, NetworkOptimization, PreloadMetrics,
};

pub use anomaly_detection::{
    AnomalyDetector, AnomalyEvent, AnomalyType, SeverityLevel,
    SystemMetrics, DetectionStats,
};

pub use cpu_frequency::{
    CPUFrequencyScaler, CPUFrequency, VoltageLevel, CPUCore, DVFSStats,
};

pub use power_domains::{
    PowerDomainManager, PowerDomain, PowerState, ModemMode, GPUState,
    PowerBreakdown, PowerDomainStats,
};

pub use thermal_management::{
    ThermalManager, ThermalZoneMonitor, ThermalZone, ThermalState as ThermalZoneState,
    ThermalPrediction, ZoneStatus, ThermalSummary,
};

pub use gpu_control::{
    GPUController, GPUFrequency as GPUFreq, RenderingMode, FrameBuffer, ColorFormat,
    GPUMetrics,
};
