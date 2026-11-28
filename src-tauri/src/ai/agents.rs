//! Agent names describe library slots, never bundled or impersonated character voices.
use super::{config::EngineConfig, manager::AIModelManager, validator::verify_model};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::{collections::BTreeMap, fs, path::PathBuf};
pub const AGENTS: [&str; 19] = [
    "Jett", "Sage", "Reyna", "Viper", "Killjoy", "Neon", "Clove", "Fade", "Raze", "Skye",
    "Deadlock", "Omen", "Cypher", "Yoru", "Chamber", "Phoenix", "Sova", "Iso", "Gekko",
];
pub const ORIGINAL_AGENTS: [(&str, &str); 2] = [
    ("wind-assassin", "Wind Assassin"),
    ("jade-guardian", "Jade Guardian"),
];

pub fn original_name(id: &str) -> Option<&'static str> {
    ORIGINAL_AGENTS
        .iter()
        .find(|agent| agent.0 == id)
        .map(|agent| agent.1)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binding {
    #[serde(default)]
    pub weights_sha256: Option<String>,
    pub model_id: String,
    pub sha256: String,
    pub identity_name: String,
    pub rights_confirmed: bool,
    pub quality_mode: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentVoice {
    pub id: String,
    pub name: String,
    pub state: String,
    pub binding: Option<Binding>,
    pub message: String,
}
pub struct AgentLibrary {
    path: PathBuf,
    bindings: BTreeMap<String, Binding>,
}
impl AgentLibrary {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let bindings = if path.exists() {
            let mut bytes = Vec::new();
            fs::File::open(&path)
                .map_err(|e| e.to_string())?
                .take(128 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 128 * 1024 {
                return Err("Agent library exceeds limit".into());
            }
            serde_json::from_slice(&bytes).map_err(|e| format!("Invalid agent library: {e}"))?
        } else {
            BTreeMap::new()
        };
        Ok(Self { path, bindings })
    }
    pub fn list(&self, models: &AIModelManager) -> Vec<AgentVoice> {
        ORIGINAL_AGENTS
            .iter()
            .map(|(id, name)| (id.to_string(), name.to_string()))
            .chain(
                AGENTS
                    .iter()
                    .map(|name| (name.to_ascii_lowercase(), format!("{name}-inspired"))),
            )
            .map(|(id, name)| {
                let binding = self.bindings.get(&id).cloned();
                let (state, message) = match binding {
                    None => (
                        "notInstalled",
                        "Import an authorized original voice pack.".into(),
                    ),
                    Some(_) => match self.resolve(&id, models) {
                        Ok(_) => (
                            "imported",
                            "Neural identity assigned. Inference validation is required.".into(),
                        ),
                        Err(error) => ("error", error),
                    },
                };
                AgentVoice {
                    id,
                    name,
                    state: state.into(),
                    message,
                    binding,
                }
            })
            .collect()
    }
    pub fn assign(
        &mut self,
        id: &str,
        mut binding: Binding,
        models: &AIModelManager,
    ) -> Result<(), String> {
        check_id(id)?;
        quality_config(&EngineConfig::default(), &binding.quality_mode)?;
        if !binding.rights_confirmed
            || binding.identity_name.trim().is_empty()
            || binding.identity_name.len() > 160
        {
            return Err(
                "Supply an independent voice identity name and confirm model/voice rights".into(),
            );
        }
        let (metadata, path) = models.resolve(&binding.model_id)?;
        if metadata.backend != "w-okada-rvc" || !metadata.consent_confirmed {
            return Err(
                "Agent Voices require an authorized RVC neural model, not a DSP preset".into(),
            );
        }
        verify_model(&metadata, &path)?;
        let info = super::onnx::inspect(&path)?;
        if info.compatibility != "candidate" {
            return Err(format!(
                "Incompatible neural model: {}",
                info.issues.join(" ")
            ));
        }
        binding.weights_sha256 = Some(info.weights_sha256);
        if metadata.sha256 != binding.sha256 {
            return Err("Model checksum changed; inspect and assign again".into());
        }
        if self.bindings.iter().any(|(key, b)| {
            key != id
                && (b.sha256 == binding.sha256
                    || b.weights_sha256
                        .as_ref()
                        .is_some_and(|hash| Some(hash) == binding.weights_sha256.as_ref())
                    || b.identity_name
                        .trim()
                        .eq_ignore_ascii_case(binding.identity_name.trim()))
        }) {
            return Err("This neural identity is already assigned to another Agent. Pitch or DSP changes cannot create an independent identity.".into());
        }
        let mut next = self.bindings.clone();
        next.insert(id.into(), binding);
        self.persist(&next)?;
        self.bindings = next;
        Ok(())
    }
    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        check_id(id)?;
        let mut next = self.bindings.clone();
        next.remove(id);
        self.persist(&next)?;
        self.bindings = next;
        Ok(())
    }
    pub fn resolve(
        &self,
        id: &str,
        models: &AIModelManager,
    ) -> Result<(super::models::AIModelMetadata, PathBuf, Binding), String> {
        check_id(id)?;
        let b = self
            .bindings
            .get(id)
            .ok_or("Agent voice is Not Installed; assign an authorized independent neural model")?;
        let (m, p) = models.resolve(&b.model_id)?;
        if !b.rights_confirmed
            || !m.consent_confirmed
            || m.sha256 != b.sha256
            || m.backend != "w-okada-rvc"
        {
            return Err("Agent identity or licensing changed; assign it again".into());
        }
        verify_model(&m, &p)?;
        if let Some(hash) = m.feature_index_sha256.as_deref() {
            super::original::verify_index(&p.with_extension("index"), hash)?;
        }
        Ok((m, p, b.clone()))
    }
    fn persist(&self, next: &BTreeMap<String, Binding>) -> Result<(), String> {
        if let Some(p) = self.path.parent() {
            fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let temp = self.path.with_extension("json.new");
        fs::write(
            &temp,
            serde_json::to_vec_pretty(next).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        super::runtime::replace_file(&temp, &self.path)
    }
}
pub(super) fn check_id(id: &str) -> Result<(), String> {
    if original_name(id).is_none() && !AGENTS.iter().any(|a| a.to_ascii_lowercase() == id) {
        return Err("Unknown Agent voice slot".into());
    }
    Ok(())
}
pub fn quality_config(current: &EngineConfig, mode: &str) -> Result<EngineConfig, String> {
    let mut c = current.clone();
    match mode {
        "lowLatency" => {
            c.chunk_frames = 2048;
            c.crossfade_frames = 512;
            c.extra_context_frames = 2048;
        }
        "balanced" => {
            c.chunk_frames = 4096;
            c.crossfade_frames = 1024;
            c.extra_context_frames = 8192;
        }
        "highQuality" => {
            c.chunk_frames = 8192;
            c.crossfade_frames = 2048;
            c.extra_context_frames = 16384;
        }
        _ => return Err("Choose lowLatency, balanced or highQuality".into()),
    }
    c.failure_policy = super::config::FailurePolicy::Mute;
    c.validate()?;
    Ok(c)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_and_original_slots_stay_uninstalled_without_weights() {
        let root =
            std::env::temp_dir().join(format!("vm-agents-{}", super::super::process::now_ms()));
        let models = AIModelManager::new(root.join("models")).unwrap();
        let agents = AgentLibrary::load(root.join("agents.json")).unwrap();
        let list = agents.list(&models);
        assert_eq!(list.len(), 21);
        assert_eq!(list[0].id, "wind-assassin");
        assert_eq!(list[1].id, "jade-guardian");
        assert!(list.iter().all(|v| v.state == "notInstalled"));
        assert!(agents.resolve("jett", &models).is_err());
        assert!(agents.resolve("../bad", &models).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn quality_modes_apply_real_engine_settings() {
        let c = EngineConfig::default();
        assert_eq!(quality_config(&c, "lowLatency").unwrap().chunk_frames, 2048);
        assert_eq!(
            quality_config(&c, "highQuality")
                .unwrap()
                .extra_context_frames,
            16384
        );
        assert!(quality_config(&c, "fake").is_err());
    }
}
