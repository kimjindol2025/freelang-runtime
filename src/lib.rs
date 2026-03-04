// Project Sovereign: Self-Learning Intelligent Phone OS
// L4 Intelligence Layer + L3 Hardware Integration + L2 System Integration + L1 Optimization + L0 ML Intelligence

pub mod user_behavior_model;
pub mod system_adaptation;
pub mod predictive_preload;
pub mod anomaly_detection;
pub mod cpu_frequency;
pub mod power_domains;
pub mod thermal_management;
pub mod gpu_control;
pub mod system_integration;
pub mod performance_profiler;
pub mod optimization_engine;
pub mod api_interface;
pub mod ml_model;
pub mod neural_predictor;
pub mod online_learning;
pub mod model_evaluation;
pub mod device_metrics_collector;
pub mod on_device_learning_server;
pub mod telemetry_uploader;
pub mod lstm_sequence_model;
pub mod attention_mechanism;
pub mod multi_task_learner;
pub mod model_ensemble;
pub mod quantization_engine;
pub mod gpu_accelerator;
pub mod cache_optimizer;
pub mod performance_profiler;
pub mod deployment_manager;
pub mod health_monitor;
pub mod analytics_engine;
pub mod alerting_system;
pub mod rollback_manager;

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

pub use system_integration::{
    SovereignSystem, SystemMode, ControlDecision,
};

pub use performance_profiler::{
    PerformanceProfiler, CPUProfile, MemoryProfile, ThermalProfile, BatteryProfile, GPUProfile,
    Bottleneck, OptimizationOpportunity, ProfilingReport, ProfileType,
};

pub use optimization_engine::{
    OptimizationEngine, OptimizationStrategy, OptimizationAction, OptimizationMetrics,
};

pub use api_interface::{
    SovereignAPI, SystemController, SystemStatus, PerformanceMetrics, TelemetryData,
    APIError, APIResult,
};

pub use ml_model::{
    MLModel, ModelConfig, ModelQuantization, Prediction, PredictionResult, ModelMetrics,
    PredictionConfidence,
};

pub use neural_predictor::{
    NeuralPredictor, FeatureVector, AppPrediction, NeuralPrediction, PredictionInput,
};

pub use online_learning::{
    OnlineLearning, LearningPhase, LearningMetrics, TrainingBatch, DriftDetectionResult,
};

pub use model_evaluation::{
    ModelEvaluator, ValidationResult, ClassMetrics, ConfusionMatrix, PredictionEval,
};

pub use device_metrics_collector::{
    SensorReading, PrivacyFilteredMetrics, BufferMetrics,
    SensorDataAggregator, PrivacyFilter, MetricsBuffer, DeviceMetricsCollector,
};

pub use on_device_learning_server::{
    TrainingBatch, DriftResult, SafetyCheckResult,
    LocalTrainingEngine, DriftMonitoringV2, SafetyValidator, OnDeviceLearningServer,
};

pub use telemetry_uploader::{
    AggregatedStats, DifferentialPrivacyResult, TelemetryBatch, PersistedTelemetry,
    EdgeAggregation, DifferentialPrivacy, RobustUpload, TelemetryUploader,
};

pub use lstm_sequence_model::{
    LSTMCellState, LSTMOutput, SequenceBuffer,
    LSTMCell, LSTMLayer, LSTMSequenceModel,
};

pub use attention_mechanism::{
    AttentionHeadOutput, MultiHeadAttentionOutput,
    AttentionHead, MultiHeadAttention, AttentionMechanism,
};

pub use multi_task_learner::{
    TaskPrediction, MultiTaskOutput, TaskLosses,
    TaskHead, SharedBackbone, MultiTaskLearner,
};

pub use model_ensemble::{
    EnsemblePrediction, FixedWeightEnsemble, LearnedGateEnsemble,
    ConfidenceWeightedEnsemble, ModelEnsemble, EnsembleStrategy,
};

pub use quantization_engine::{
    QuantizationScheme, Int8Quantizer, Int4Quantizer, CalibrationEngine,
    QuantizationEngine,
};

pub use gpu_accelerator::{
    GPUDeviceType, GPUMemory, GPUKernel, GPUDevice, GPUAccelerator,
};

pub use cache_optimizer::{
    CacheHierarchy, LoopTiler, DataReorderer, Prefetcher,
    CacheOptimizer, CacheStats,
};

pub use performance_profiler::{
    LatencyTracker, PowerMonitor, CacheMonitor, AccuracyTracker,
    PerformanceProfiler, ProfilingReport,
};

pub use deployment_manager::{
    DeploymentVersion, DeploymentStage, HealthGate, DeploymentMetrics,
    DeploymentManager,
};

pub use health_monitor::{
    SystemMetrics, InferenceMetrics, HealthScore, HealthMonitor,
};

pub use analytics_engine::{
    LatencyDistribution, AnomalyDetector, ThroughputAnalysis,
    AnalyticsEngine,
};

pub use alerting_system::{
    AlertSeverity, Alert, AlertingSystem,
};

pub use rollback_manager::{
    RollbackVersion, RollbackReason, RollbackManager,
};
