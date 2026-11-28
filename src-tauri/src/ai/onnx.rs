//! Structural ONNX reader; operator execution is checked by the pinned runtime.
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub compatibility: String,
    pub issues: Vec<String>,
    pub metadata: BTreeMap<String, String>,
    pub inputs: Vec<Tensor>,
    pub outputs: Vec<Tensor>,
    pub sha256: String,
    pub weights_sha256: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tensor {
    pub name: String,
    pub element_type: u64,
    pub shape: Vec<String>,
}
#[derive(Clone, Copy)]
struct F {
    n: u64,
    w: u64,
    v: u64,
    a: u64,
    b: u64,
}
struct Reader {
    file: File,
    budget: usize,
    weight_hashes: BTreeMap<String, Vec<String>>,
}
fn err(e: std::io::Error) -> String {
    format!("Cannot inspect ONNX: {e}")
}
impl Reader {
    fn var(&mut self, end: u64) -> Result<u64, String> {
        let mut v = 0;
        for s in (0..70).step_by(7) {
            if self.file.stream_position().map_err(err)? >= end {
                return Err("Truncated ONNX protobuf".into());
            }
            let mut b = [0];
            self.file.read_exact(&mut b).map_err(err)?;
            if s == 63 && b[0] > 1 {
                return Err("Invalid ONNX varint".into());
            }
            v |= ((b[0] & 127) as u64) << s;
            if b[0] < 128 {
                return Ok(v);
            }
        }
        Err("Invalid ONNX varint".into())
    }
    fn fields(&mut self, a: u64, b: u64) -> Result<Vec<F>, String> {
        self.file.seek(SeekFrom::Start(a)).map_err(err)?;
        let mut fs = Vec::new();
        while self.file.stream_position().map_err(err)? < b {
            if self.budget == 0 {
                return Err("ONNX complexity limit exceeded".into());
            }
            self.budget -= 1;
            let k = self.var(b)?;
            let n = k >> 3;
            let w = k & 7;
            if n == 0 {
                return Err("Invalid ONNX field".into());
            }
            let v = match w {
                0 | 2 => self.var(b)?,
                1 => 8,
                5 => 4,
                _ => return Err("Invalid ONNX wire type".into()),
            };
            let a = self.file.stream_position().map_err(err)?;
            let next = if w == 0 {
                a
            } else {
                a.checked_add(v).ok_or("Invalid ONNX length")?
            };
            if next > b {
                return Err("Truncated ONNX message".into());
            }
            fs.push(F {
                n,
                w,
                v,
                a,
                b: next,
            });
            self.file.seek(SeekFrom::Start(next)).map_err(err)?;
        }
        Ok(fs)
    }
    fn sub(&mut self, f: F) -> Result<Vec<F>, String> {
        if f.w != 2 {
            return Err("Invalid ONNX message".into());
        }
        self.fields(f.a, f.b)
    }
    fn text(&mut self, fs: &[F], n: u64) -> Result<Option<String>, String> {
        let Some(f) = fs.iter().find(|f| f.n == n) else {
            return Ok(None);
        };
        if f.w != 2 || f.v > 65536 {
            return Err("ONNX string exceeds limit".into());
        }
        self.file.seek(SeekFrom::Start(f.a)).map_err(err)?;
        let mut b = vec![0; f.v as usize];
        self.file.read_exact(&mut b).map_err(err)?;
        String::from_utf8(b)
            .map(Some)
            .map_err(|_| "Invalid UTF-8 ONNX metadata".into())
    }
    fn tensor(&mut self, f: F) -> Result<Tensor, String> {
        let fs = self.sub(f)?;
        let name = self.text(&fs, 1)?.ok_or("Missing tensor name")?;
        let typ = self.sub(*fs.iter().find(|f| f.n == 2).ok_or("Missing tensor type")?)?;
        let t = self.sub(
            *typ.iter()
                .find(|f| f.n == 1)
                .ok_or("Only dense tensors supported")?,
        )?;
        let element_type = t
            .iter()
            .find(|f| f.n == 1 && f.w == 0)
            .ok_or("Missing tensor element type")?
            .v;
        let mut shape = Vec::new();
        if let Some(f) = t.iter().find(|f| f.n == 2) {
            for d in self.sub(*f)?.into_iter().filter(|f| f.n == 1) {
                let ds = self.sub(d)?;
                shape.push(if let Some(n) = ds.iter().find(|f| f.n == 1 && f.w == 0) {
                    n.v.to_string()
                } else {
                    self.text(&ds, 2)?.unwrap_or_else(|| "?".into())
                });
            }
        }
        Ok(Tensor {
            name,
            element_type,
            shape,
        })
    }
    fn weights(&mut self, f: F) -> Result<(), String> {
        let fs = self.sub(f)?;
        if fs
            .iter()
            .any(|f| f.n == 13 || (f.n == 14 && f.w == 0 && f.v == 1))
        {
            return Err("External ONNX weights are unsupported; export one self-contained model with embedded data".into());
        }
        let name = self.text(&fs, 8)?.unwrap_or_default();
        let mut digest = Sha256::new();
        let mut data = false;
        for field in fs.iter().filter(|f| [4, 5, 6, 7, 9, 10, 11].contains(&f.n)) {
            data = true;
            digest.update(field.n.to_le_bytes());
            digest.update(field.w.to_le_bytes());
            if field.w == 0 {
                digest.update(field.v.to_le_bytes());
                continue;
            }
            digest.update((field.b - field.a).to_le_bytes());
            self.file.seek(SeekFrom::Start(field.a)).map_err(err)?;
            let mut remaining = field.b - field.a;
            let mut buffer = [0u8; 65536];
            while remaining > 0 {
                let count = remaining.min(buffer.len() as u64) as usize;
                self.file.read_exact(&mut buffer[..count]).map_err(err)?;
                digest.update(&buffer[..count]);
                remaining -= count as u64;
            }
        }
        if data {
            self.weight_hashes
                .entry(name)
                .or_default()
                .push(format!("{:x}", digest.finalize()));
        }
        Ok(())
    }
    fn graph(&mut self, f: F, depth: usize) -> Result<(Vec<Tensor>, Vec<Tensor>), String> {
        if depth > 16 {
            return Err("ONNX nesting limit exceeded".into());
        }
        let mut ins = Vec::new();
        let mut outs = Vec::new();
        for f in self.sub(f)? {
            match f.n {
                11 => ins.push(self.tensor(f)?),
                12 => outs.push(self.tensor(f)?),
                5 => self.weights(f)?,
                15 => return Err("Sparse weights are unsupported".into()),
                1 => {
                    let node = self.sub(f)?;
                    if self.text(&node, 4)?.is_none_or(|s| s.is_empty()) {
                        return Err("Invalid ONNX operation".into());
                    }
                    for a in node.into_iter().filter(|f| f.n == 5) {
                        for attr in self.sub(a)? {
                            match attr.n {
                                5 | 10 => self.weights(attr)?,
                                6 | 11 => {
                                    self.graph(attr, depth + 1)?;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok((ins, outs))
    }
}
pub fn inspect(path: &Path) -> Result<Inspection, String> {
    let sha256 = super::validator::sha256_file(path)?;
    let file = File::open(path).map_err(err)?;
    let size = file.metadata().map_err(err)?.len();
    let mut r = Reader {
        file,
        budget: 2_000_000,
        weight_hashes: BTreeMap::new(),
    };
    let fs = r.fields(0, size)?;
    if !fs.iter().any(|f| f.n == 1 && f.w == 0 && f.v > 0)
        || !fs.iter().any(|f| f.n == 8 && f.w == 2)
    {
        return Err("Invalid ONNX: IR version or opset missing".into());
    }
    let graph = fs
        .iter()
        .find(|f| f.n == 7)
        .ok_or("Invalid ONNX: graph missing")?;
    let (inputs, outputs) = r.graph(*graph, 0)?;
    if inputs.is_empty() || outputs.is_empty() {
        return Err("Invalid ONNX: graph tensors missing".into());
    }
    let mut metadata = BTreeMap::new();
    for f in fs.iter().filter(|f| f.n == 14) {
        let entry = r.sub(*f)?;
        let key = r.text(&entry, 1)?.ok_or("Missing metadata key")?;
        let value = r.text(&entry, 2)?.unwrap_or_default();
        if metadata.insert(key, value).is_some() {
            return Err("Duplicate ONNX metadata key".into());
        }
    }
    let m: serde_json::Value = match metadata.get("metadata") {
        Some(v) => serde_json::from_str(v).map_err(|_| "Invalid embedded RVC JSON")?,
        None => serde_json::Value::Null,
    };
    let mut issues = Vec::new();
    let version = m["version"].as_str();
    let dimensions = m["embChannels"].as_u64();
    let rate = m["samplingRate"].as_u64();
    let f0 = m["f0"]
        .as_bool()
        .or_else(|| m["f0"].as_u64().filter(|v| *v <= 1).map(|v| v == 1));
    if !matches!(version, Some("1" | "2" | "2.1" | "2.2")) {
        issues.push(
            "Missing or unsupported RVC version; export metadata for the pinned w-okada engine"
                .into(),
        );
    }
    if !matches!(dimensions, Some(256 | 768))
        || !matches!(rate, Some(32000 | 40000 | 48000))
        || f0.is_none()
    {
        issues.push("Missing or incompatible embedded embChannels, samplingRate or f0".into());
    }
    if !matches!(m["embedder"].as_str(), Some("hubert_base" | "contentvec")) {
        issues.push("Unsupported embedder: expected hubert_base or contentvec".into());
    }
    let expected = if f0 == Some(false) {
        vec!["feats", "p_len", "sid"]
    } else {
        vec!["feats", "p_len", "pitch", "pitchf", "sid"]
    };
    if expected.len() != inputs.len()
        || expected
            .iter()
            .any(|n| !inputs.iter().any(|t| t.name == *n))
    {
        issues.push("Incompatible RVC input signature".into());
    }
    if !inputs.first().is_some_and(|t| {
        t.name == "feats"
            && t.shape.len() == 3
            && [1, 10].contains(&t.element_type)
            && t.shape[2].parse::<u64>().ok() == dimensions
    }) {
        issues.push(
            "RVC feats must be the first input, rank-3 float, with matching embedding dimensions"
                .into(),
        );
    }
    for t in &inputs {
        if match t.name.as_str() {
            "p_len" | "sid" => t.element_type != 7 || t.shape.len() != 1,
            "pitch" => t.element_type != 7 || t.shape.len() != 2,
            "pitchf" => t.element_type != 1 || t.shape.len() != 2,
            _ => false,
        } {
            issues.push(format!("Unsupported tensor type or shape: {}", t.name));
        }
    }
    let rank = if matches!(version, Some("2.1" | "2.2")) {
        1
    } else {
        3
    };
    if !outputs
        .iter()
        .any(|t| t.name == "audio" && t.shape.len() == rank && [1, 10].contains(&t.element_type))
    {
        issues.push(format!("Expected rank-{rank} float audio output"));
    }
    let mut weight_digest = Sha256::new();
    if r.weight_hashes.is_empty() {
        issues.push("No embedded neural weights detected".into());
    }
    for (name, mut hashes) in r.weight_hashes {
        hashes.sort();
        weight_digest.update((name.len() as u64).to_le_bytes());
        weight_digest.update(name.as_bytes());
        for hash in hashes {
            weight_digest.update(hash.as_bytes());
        }
    }
    let weights_sha256 = format!("{:x}", weight_digest.finalize());
    Ok(Inspection {
        compatibility: if issues.is_empty() {
            "candidate"
        } else {
            "incompatible"
        }
        .into(),
        issues,
        metadata,
        inputs,
        outputs,
        sha256,
        weights_sha256,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_arbitrary_and_truncated_bytes() {
        let p =
            std::env::temp_dir().join(format!("vm-onnx-{}.onnx", super::super::process::now_ms()));
        for bytes in [&b"fake model"[..], &[0x3a, 0xff][..], &[][..]] {
            std::fs::write(&p, bytes).unwrap();
            assert!(inspect(&p).is_err());
        }
        std::fs::remove_file(p).unwrap();
    }
}
