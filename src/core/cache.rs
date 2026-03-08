/*
 * Copyright 2025 Tarin Mahmood
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MnemonicsType {
    PROJECT,
    TAG,
    CustomQuery,
}

pub trait MnemonicsCache {
    fn insert(
        &mut self,
        mn_type: MnemonicsType,
        key: &str,
        value: &str,
        ovrrde: bool,
    ) -> Result<(), anyhow::Error>;
    fn remove(&mut self, mn_type: MnemonicsType, key: &str) -> Result<(), anyhow::Error>;
    fn get(&self, mn_type: MnemonicsType, key: &str) -> Option<String>;
    fn save(&self) -> Result<(), anyhow::Error>;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MnemonicsTable {
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    tags: HashMap<String, String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    projects: HashMap<String, String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    custom_queries: HashMap<String, String>,
}

impl MnemonicsTable {
    pub const fn get(&self, mn_type: MnemonicsType) -> &HashMap<String, String> {
        match mn_type {
            MnemonicsType::PROJECT => &self.projects,
            MnemonicsType::TAG => &self.tags,
            MnemonicsType::CustomQuery => &self.custom_queries,
        }
    }

    pub fn insert(&mut self, mn_type: MnemonicsType, key: &str, value: &str) {
        let _ = match mn_type {
            MnemonicsType::PROJECT => self.projects.insert(key.to_string(), value.to_string()),
            MnemonicsType::TAG => self.tags.insert(key.to_string(), value.to_string()),
            MnemonicsType::CustomQuery => self
                .custom_queries
                .insert(key.to_string(), value.to_string()),
        };
    }

    pub fn remove(&mut self, mn_type: MnemonicsType, key: &str) {
        let _ = match mn_type {
            MnemonicsType::PROJECT => self.projects.remove(key),
            MnemonicsType::TAG => self.tags.remove(key),
            MnemonicsType::CustomQuery => self.custom_queries.remove(key),
        };
    }
}

#[derive(Debug, Clone)]
pub struct FileMnemonicsCache {
    cfg_path: Arc<Mutex<PathBuf>>,
    map: MnemonicsTable,
}

impl FileMnemonicsCache {
    pub fn new(path: Arc<Mutex<PathBuf>>) -> Self {
        Self {
            cfg_path: path,
            map: MnemonicsTable::default(),
        }
    }

    pub fn load(&mut self) -> Result<(), anyhow::Error> {
        let cfg_path_lck = self.cfg_path.lock().expect("Cannot lock file");
        let file = File::open(cfg_path_lck.as_path());
        if let Ok(mut file_obj) = file {
            let mut buf = String::new();
            let _ = file_obj.read_to_string(&mut buf);
            if !buf.is_empty() {
                let x: MnemonicsTable = toml::from_str(&buf)
                    .map_err(|p| anyhow!("Could not parse configuration file: {}!", p))?;
                self.map = x;
            }
        }
        Ok(())
    }
}

impl MnemonicsCache for FileMnemonicsCache {
    fn insert(
        &mut self,
        mn_type: MnemonicsType,
        key: &str,
        value: &str,
        ovrrde: bool,
    ) -> Result<(), anyhow::Error> {
        // Ensure, that the shortcut is duplicate for the own type.
        let x = self
            .map
            .get(mn_type.clone())
            .iter()
            .filter(|p| p.0 != key)
            .find(|p| p.1.as_str().eq(value));
        if let Some(x) = x {
            if ovrrde {
                let key_dlt = x.0.clone();
                self.map.remove(mn_type.clone(), &key_dlt);
            } else {
                return Err(anyhow!("Duplicate key generated!"));
            }
        }

        if mn_type.eq(&MnemonicsType::PROJECT) {
            let x = self
                .map
                .get(MnemonicsType::TAG)
                .values()
                .find(|p| p.as_str().eq(value));
            if x.is_some() {
                return Err(anyhow!("Duplicate key generated!"));
            }
        }
        if mn_type.eq(&MnemonicsType::TAG) {
            let x = self
                .map
                .get(MnemonicsType::PROJECT)
                .values()
                .find(|p| p.as_str().eq(value));
            if x.is_some() {
                return Err(anyhow!("Duplicate key generated!"));
            }
        }

        self.map.insert(mn_type, key, value);
        self.save()?;
        Ok(())
    }

    fn remove(&mut self, mn_type: MnemonicsType, key: &str) -> Result<(), anyhow::Error> {
        self.map.remove(mn_type, key);
        self.save()?;
        Ok(())
    }

    fn get(&self, mn_type: MnemonicsType, key: &str) -> Option<String> {
        self.map.get(mn_type).get(key).cloned()
    }

    fn save(&self) -> Result<(), anyhow::Error> {
        let p = self.cfg_path.lock().expect("Can lock file");
        let toml = toml::to_string(&self.map).unwrap();
        let mut f = File::create(p.as_path())?;
        let _ = f.write_all(toml.as_bytes());
        Ok(())
    }
}

pub(crate) type MnemonicsCacheType = dyn MnemonicsCache + Send + Sync;

#[cfg(test)]
mod tests;

