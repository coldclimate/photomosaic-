use serde_json::{json, Value};
use std::path::Path;

pub struct TileCache {
    cache: Value,
    cache_path: std::path::PathBuf,
}

impl TileCache {
    pub fn new(tile_dir: &Path) -> Self {
        let cache_path = tile_dir.join(".photomosaic_cache.json");

        let cache = if cache_path.exists() {
            std::fs::read_to_string(&cache_path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_else(|| json!({"version": 1, "tiles": {}}))
        } else {
            json!({"version": 1, "tiles": {}})
        };

        TileCache { cache, cache_path }
    }

    pub fn get_features(&self, filename: &str, file_id: &str, colorspace: &str) -> Option<Vec<f32>> {
        self.cache["tiles"][filename]
            .get("id")
            .and_then(|h| {
                if h.as_str() == Some(file_id) {
                    self.cache["tiles"][filename]["features"][colorspace]
                        .as_array()
                        .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
                } else {
                    None
                }
            })
    }

    pub fn set_features(&mut self, filename: &str, file_id: &str, colorspace: &str, features: &[f32]) {
        if !self.cache["tiles"][filename].is_object() {
            self.cache["tiles"][filename] = json!({"id": file_id, "features": {}});
        } else if self.cache["tiles"][filename]["id"].as_str() != Some(file_id) {
            self.cache["tiles"][filename]["id"] = json!(file_id);
            self.cache["tiles"][filename]["features"] = json!({});
        }

        let features_json: Vec<Value> = features.iter().map(|f| json!(f)).collect();
        self.cache["tiles"][filename]["features"][colorspace] = json!(features_json);
    }

    pub fn save(&self) -> std::io::Result<()> {
        let json_str = serde_json::to_string_pretty(&self.cache)?;
        std::fs::write(&self.cache_path, json_str)
    }
}

pub fn compute_file_id(path: &Path) -> std::io::Result<String> {
    // Use file size + modification time as a fast way to detect changes
    let metadata = std::fs::metadata(path)?;
    let size = metadata.len();
    let mtime = metadata.modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(format!("{:x}_{:x}", size, mtime))
}
