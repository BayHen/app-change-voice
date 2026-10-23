pub mod factory;
pub mod manifest;
pub mod registry;

pub use manifest::{EffectDescriptor, ParameterDescriptor, ParameterKind, PluginManifest};
pub use registry::{EffectRegistry, PluginStatus};
