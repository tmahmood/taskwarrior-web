use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, PartialEq)]
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
    pub fn get(&self, mn_type: MnemonicsType) -> &HashMap<String, String> {
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
                let x: MnemonicsTable = toml::from_str(&buf).map_err(|p| {
                    anyhow!("Could not parse configuration file: {}!", p.to_string())
                })?;
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
            .filter(|p| p.0 != &key)
            .find(|p| p.1.as_str().eq(value));
        if let Some(x) = x.clone() {
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
        self.map.remove(mn_type, &key);
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
mod tests {
    use std::{
        io::{Read, Seek},
        str::FromStr,
    };

    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_mnemonics_cache() {
        let mut file1 = NamedTempFile::new().expect("Cannot create named temp files.");
        let x = PathBuf::from(file1.path());
        let file_mtx = Arc::new(Mutex::new(x));

        let mut mock = FileMnemonicsCache::new(file_mtx);
        assert_eq!(mock.get(MnemonicsType::PROJECT, "personal"), None);
        assert_eq!(
            mock.insert(MnemonicsType::TAG, "personal", "xz", false)
                .is_ok(),
            true
        );
        assert_eq!(
            mock.get(MnemonicsType::TAG, "personal"),
            Some(String::from("xz"))
        );
        // how to validate content?
        file1.reopen().expect("Cannot reopen");
        let mut buf = String::new();
        let read_result = file1.read_to_string(&mut buf);
        assert_eq!(read_result.is_ok(), true);
        let read_result = read_result.expect("Could not read from file");
        assert!(read_result > 0);
        assert_eq!(
            buf,
            String::from("[tags]\npersonal = \"xz\"\n")
        );
        assert_eq!(
            mock.insert(MnemonicsType::PROJECT, "taskwarrior", "xz", false)
                .is_ok(),
            false
        );
        assert_eq!(mock.remove(MnemonicsType::TAG, "personal").is_ok(), true);
        assert_eq!(mock.get(MnemonicsType::TAG, "personal"), None);
        assert_eq!(
            mock.insert(MnemonicsType::PROJECT, "taskwarrior", "xz", false)
                .is_ok(),
            true
        );
        assert_eq!(
            mock.insert(MnemonicsType::TAG, "personal", "xz", false)
                .is_ok(),
            false
        );
        assert_eq!(
            mock.remove(MnemonicsType::PROJECT, "taskwarrior").is_ok(),
            true
        );
        file1.reopen().expect("Cannot reopen");
        let _ = file1.as_file().set_len(0);
        let _ = file1.seek(std::io::SeekFrom::Start(0));
        let data = String::from("[tags]\npersonal = \"xz\"\n\n[projects]\n");
        let _ = file1.write_all(data.as_bytes());
        let _ = file1.flush();
        assert_eq!(mock.load().is_ok(), true);
        assert_eq!(
            mock.get(MnemonicsType::TAG, "personal"),
            Some(String::from("xz"))
        );
        file1.reopen().expect("Cannot reopen");
        let _ = file1.as_file().set_len(0);
        let _ = file1.seek(std::io::SeekFrom::Start(0));
        let data = String::from("**********");
        let _ = file1.write_all(data.as_bytes());
        let _ = file1.flush();
        assert_eq!(mock.load().is_ok(), false);
        // Empty file cannot be parsed, but should not through an error!
        let _ = file1.as_file().set_len(0);
        let _ = file1.seek(std::io::SeekFrom::Start(0));
        let _ = file1.flush();
        assert_eq!(mock.load().is_ok(), true);
        // If the configuration file does not exist yet (close will delete),
        // it is fine as well.
        let _ = file1.close();
        assert_eq!(mock.load().is_ok(), true);
    }

    #[test]
    fn test_custom_queries() {
        let mut file1 = NamedTempFile::new().expect("Cannot create named temp files.");
        let x = PathBuf::from(file1.path());
        let file_mtx = Arc::new(Mutex::new(x));
        let mut mock = FileMnemonicsCache::new(file_mtx);

        // Check for retrieving custom query shortcuts.
        assert_eq!(mock.get(MnemonicsType::CustomQuery, "one_query"), None);

        // Insert a one_query shortcut and verify, that the query shortcut
        // is saved.
        assert_eq!(
            mock.insert(MnemonicsType::CustomQuery, "one_query", "ad", false)
                .is_ok(),
            true
        );
        assert_eq!(
            mock.get(MnemonicsType::CustomQuery, "one_query"),
            Some(String::from("ad"))
        );

        // Save to file and ensure, its proper written.
        let mut buf = String::new();
        let read_result = file1.read_to_string(&mut buf);
        assert_eq!(read_result.is_ok(), true);
        let read_result = read_result.expect("Could not read from file");
        assert!(read_result > 0);
        assert_eq!(
            buf,
            String::from("[custom_queries]\none_query = \"ad\"\n")
        );

        // Delete again.
        assert_eq!(mock.remove(MnemonicsType::CustomQuery, "one_query").is_ok(), true);
        assert_eq!(mock.get(MnemonicsType::CustomQuery, "one_query"), None);
        
        // Test overwriting of queries.
        file1.reopen().expect("Cannot reopen");
        let _ = file1.as_file().set_len(0);
        let _ = file1.seek(std::io::SeekFrom::Start(0));
        let data = String::from("[custom_queries]\none_query = \"ad\"\n");
        let _ = file1.write_all(data.as_bytes());
        let _ = file1.flush();
        assert_eq!(mock.load().is_ok(), true);
        // Add a second query and ensure, that the one_query gets removed.
        assert_eq!(
            mock.insert(MnemonicsType::CustomQuery, "second_query", "ad", true)
                .is_ok(),
            true
        );
        assert_eq!(mock.get(MnemonicsType::CustomQuery, "one_query"), None);
        assert_eq!(mock.get(MnemonicsType::CustomQuery, "second_query"), Some(String::from("ad")));
        // Ensure, an error is produced in case its not overwritten.
        assert_eq!(
            mock.insert(MnemonicsType::CustomQuery, "one_query", "ad", false)
                .is_err(),
            true
        );
    }

    #[test]
    fn test_mnemonics_cache_file_fail() {
        let x = PathBuf::from_str("/4bda0a6b-da0d-46be-98e6-e06d43385fba/asdfa.cache").unwrap();
        let file_mtx = Arc::new(Mutex::new(x));

        let mut mock = FileMnemonicsCache::new(file_mtx);
        assert_eq!(
            mock.insert(MnemonicsType::TAG, "personal", "xz", false)
                .is_ok(),
            false
        );
        assert_eq!(
            mock.remove(MnemonicsType::PROJECT, "taskwarrior").is_ok(),
            false
        );
    }
}
