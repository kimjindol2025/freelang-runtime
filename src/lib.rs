// Project Sovereign: Self-Learning Intelligent Phone OS
// L4 Intelligence Layer - Root Module

pub mod user_behavior_model;
pub mod system_adaptation;

pub use user_behavior_model::{
    UserBehaviorModel, UserEvent, Location, LocationType, AppCategory,
    TimeSlotProfile, LocationProfile, AppProfile,
};

pub use system_adaptation::{
    AdaptiveScheduler, PowerOptimizer, ThermalController,
    CoreAssignment, BatteryMode, ThermalState, WorkloadClass, NetworkState, ScreenState,
};
