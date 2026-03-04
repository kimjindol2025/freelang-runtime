// Project Sovereign: Self-Learning Intelligent Phone OS
// L4 Intelligence Layer - Root Module

pub mod user_behavior_model;
pub mod system_adaptation;
pub mod predictive_preload;
pub mod anomaly_detection;

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
