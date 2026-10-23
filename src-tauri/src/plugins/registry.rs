use super::{
    factory::{EffectFactory, TrimFactory},
    manifest::*,
};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginStatus {
    pub manifest: PluginManifest,
    pub enabled: bool,
}

pub struct EffectRegistry {
    plugins: BTreeMap<String, PluginStatus>,
    effects: HashMap<String, String>,
    factories: HashMap<String, Box<dyn EffectFactory>>,
}

impl EffectRegistry {
    pub fn built_in() -> Result<Self, String> {
        let mut registry = Self {
            plugins: BTreeMap::new(),
            effects: HashMap::new(),
            factories: HashMap::new(),
        };
        registry.register_factory(Box::new(TrimFactory));
        registry.register(PluginManifest {
            schema_version: PLUGIN_MANIFEST_SCHEMA,
            id: "voicemorph-core".into(),
            name: "VoiceMorph Core Effects".into(),
            version: "0.1.0".into(),
            api_version: PLUGIN_API_VERSION.into(),
            publisher: "VoiceMorph Studio".into(),
            trusted: true,
            supported_platforms: vec!["windows-x86_64".into(), "macos-universal".into()],
            required_capabilities: vec!["audio-process".into()],
            dependencies: BTreeMap::new(),
            effects: vec![EffectDescriptor {
                id: "builtin.trim".into(),
                name: "Trim".into(),
                realtime_safe: true,
                parameters: vec![ParameterDescriptor {
                    id: "gain".into(),
                    label: "Gain".into(),
                    kind: ParameterKind::Float,
                    minimum: Some(0.0),
                    maximum: Some(2.0),
                    default: serde_json::json!(1.0),
                }],
            }],
        })?;
        Ok(registry)
    }
    pub fn register_factory(&mut self, factory: Box<dyn EffectFactory>) {
        self.factories.insert(factory.effect_id().into(), factory);
    }
    pub fn register(&mut self, manifest: PluginManifest) -> Result<(), String> {
        validate_manifest(&manifest)?;
        if self.plugins.contains_key(&manifest.id) {
            return Err(format!("Duplicate plugin ID: {}", manifest.id));
        }
        for effect in &manifest.effects {
            if self.effects.contains_key(&effect.id) {
                return Err(format!("Duplicate effect ID: {}", effect.id));
            }
            if !self.factories.contains_key(&effect.id) {
                return Err(format!(
                    "No trusted factory registered for effect: {}",
                    effect.id
                ));
            }
        }
        for effect in &manifest.effects {
            self.effects.insert(effect.id.clone(), manifest.id.clone());
        }
        self.plugins.insert(
            manifest.id.clone(),
            PluginStatus {
                manifest,
                enabled: true,
            },
        );
        Ok(())
    }
    pub fn list_plugins(&self) -> Vec<PluginStatus> {
        self.plugins.values().cloned().collect()
    }
    pub fn list_effects(&self) -> Vec<EffectDescriptor> {
        self.plugins
            .values()
            .filter(|p| p.enabled)
            .flat_map(|p| p.manifest.effects.clone())
            .collect()
    }
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<PluginStatus, String> {
        let plugin = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| format!("Plugin not found: {id}"))?;
        plugin.enabled = enabled;
        Ok(plugin.clone())
    }
    pub fn create(
        &self,
        id: &str,
        sample_rate: u32,
        parameters: &BTreeMap<String, f32>,
    ) -> Result<Box<dyn crate::audio::processor::AudioProcessor>, String> {
        let plugin_id = self
            .effects
            .get(id)
            .ok_or_else(|| format!("Unsupported effect: {id}"))?;
        if !self.plugins.get(plugin_id).is_some_and(|p| p.enabled) {
            return Err(format!("Plugin is disabled: {plugin_id}"));
        }
        self.factories
            .get(id)
            .ok_or_else(|| format!("Factory unavailable: {id}"))?
            .create(sample_rate, parameters)
    }
}

pub fn validate_manifest(manifest: &PluginManifest) -> Result<(), String> {
    if manifest.schema_version != PLUGIN_MANIFEST_SCHEMA {
        return Err("Unsupported plugin manifest schema".into());
    }
    if manifest.api_version != PLUGIN_API_VERSION {
        return Err("Unsupported plugin API version".into());
    }
    if !manifest.trusted {
        return Err("Untrusted native plugins are not executable".into());
    }
    if !manifest
        .supported_platforms
        .iter()
        .any(|p| p == "windows-x86_64")
    {
        return Err("Plugin does not support Windows x64".into());
    }
    if manifest.id.trim().is_empty()
        || manifest.name.trim().is_empty()
        || manifest.version.trim().is_empty()
    {
        return Err("Plugin identity fields are required".into());
    }
    let mut ids = HashSet::new();
    for effect in &manifest.effects {
        if !effect.realtime_safe {
            return Err(format!("Effect is not realtime-safe: {}", effect.id));
        }
        if !ids.insert(&effect.id) {
            return Err(format!("Duplicate effect ID: {}", effect.id));
        }
        for p in &effect.parameters {
            if let (Some(min), Some(max)) = (p.minimum, p.maximum)
                && (!min.is_finite() || !max.is_finite() || min > max)
            {
                return Err(format!("Invalid parameter range: {}", p.id));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn built_in_factory_processes_audio() {
        let r = EffectRegistry::built_in().unwrap();
        let mut p = r
            .create(
                "builtin.trim",
                48000,
                &BTreeMap::from([("gain".into(), 0.5)]),
            )
            .unwrap();
        assert_eq!(p.process_sample(0.8), 0.4);
    }
    #[test]
    fn duplicate_plugin_is_rejected() {
        let mut r = EffectRegistry::built_in().unwrap();
        let m = r.list_plugins()[0].manifest.clone();
        assert!(r.register(m).unwrap_err().contains("Duplicate"));
    }
    #[test]
    fn untrusted_manifest_is_rejected() {
        let mut m = EffectRegistry::built_in().unwrap().list_plugins()[0]
            .manifest
            .clone();
        m.id = "other".into();
        m.trusted = false;
        assert!(validate_manifest(&m).is_err());
    }
}
