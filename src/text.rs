use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

const TEXT_DIRECTORY: &str = "assets/text";

pub struct Texts {
    entries: HashMap<String, String>,
}

impl Texts {
    pub fn load(language: &str) -> Result<Self> {
        let path = Path::new(TEXT_DIRECTORY).join(format!("{language}.ron"));
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let entries =
            ron::from_str(&text).with_context(|| format!("cannot parse {}", path.display()))?;
        Ok(Self { entries })
    }

    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.entries.get(key).map_or(key, String::as_str)
    }
}
