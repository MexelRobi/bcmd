use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Storage {
    pub shortcuts: BTreeMap<String, String>,
}

impl Storage {
    const FILE_PATH: &'static str = "shortcuts.toml";

    pub fn load() -> Self {
        if !Path::new(Self::FILE_PATH).exists() {
            return Self::default();
        }
        let content = fs::read_to_string(Self::FILE_PATH).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let content = toml::to_string_pretty(self).unwrap();
        fs::write(Self::FILE_PATH, content)
    }
}
