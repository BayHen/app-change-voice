use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PLUGIN_MANIFEST_SCHEMA: u32 = 1;
pub const PLUGIN_API_VERSION: &str = "1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ParameterKind {
    Float,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParameterDescriptor {
    pub id: String,
    pub label: String,
    pub kind: ParameterKind,
    pub minimum: Option<f32>,
    pub maximum: Option<f32>,
    pub default: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectDescriptor {
    pub id: String,
    pub name: String,
    pub realtime_safe: bool,
    #[serde(default)]
    pub parameters: Vec<ParameterDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub publisher: String,
    pub trusted: bool,
    pub supported_platforms: Vec<String>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    pub effects: Vec<EffectDescriptor>,
}
