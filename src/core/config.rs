use std::collections::HashMap;

use super::{cache::{MnemonicsCache, MnemonicsType}, errors::FieldError};

pub trait ValidateSetting {
    fn validate(&self) -> Vec<FieldError>;
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct CustomQuery {
    pub query: String,
    pub description: String,
    pub fixed_key: Option<String>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct AppSettings {
    #[serde(default)]
    pub custom_queries: HashMap<String, CustomQuery>,
}

impl AppSettings {
    pub fn new(config_path: &std::path::Path) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::from(config_path).required(false))
            .add_source(
                config::Environment::with_prefix("TWK")
                    .prefix_separator("_")
                    .separator("__"),
            )
            .build()?;
        match settings.try_deserialize::<Self>() {
            Ok(s) => {
                let validation = s.validate();
                match validation.len() {
                    0 => Ok(s),
                    _ => {
                        let error_message = format!(
                            "Configuration file couldn't be read. Following error came up: {:?}",
                            validation
                        );
                        Err(config::ConfigError::Message(error_message))
                    }
                }
            }
            Err(e) => Err(e),
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            custom_queries: Default::default(),
        }
    }
}

impl AppSettings {
    pub fn register_shortcuts(&self, cache: &mut dyn MnemonicsCache) {
        for f in &self.custom_queries {
            if let Some(fixed_key) = &f.1.fixed_key {
                cache.get(MnemonicsType::CustomQuery, f.0).is_some_and(|f| f != *fixed_key).then(|| {
                    cache.remove(MnemonicsType::CustomQuery, f.0)
                });
                let _ = cache.insert(MnemonicsType::CustomQuery, f.0, fixed_key, true);
            }
        }
    }
}

impl ValidateSetting for CustomQuery {
    fn validate(&self) -> Vec<FieldError> {
        let mut errors: Vec<FieldError> = Vec::new();
        if self.fixed_key.as_ref().is_some_and(|f| f.len() != 2) {
            errors.push(FieldError {
                field: String::from("fixed_key"),
                message: format!(
                    "Fixed key must be 2 unique characters. Currently assigned {:?} for {}!",
                    self.fixed_key.as_ref(),
                    &self.description
                ),
            });
        }

        errors
    }
}

impl ValidateSetting for HashMap<String, CustomQuery> {
    fn validate(&self) -> Vec<FieldError> {
        let mut shortcuts: Vec<String> = Vec::new();

        self.iter()
            .map(|p| {
                let mut validations = p.1.validate();
                if let Some(fixed_key) = p.1.fixed_key.as_ref() {
                    if shortcuts.contains(fixed_key) {
                        validations.push(FieldError {
                            field: String::from("fixed_key"),
                            message: format!(
                                "Duplicate shortcut {} asssigned to query {}",
                                fixed_key, p.0
                            ),
                        });
                    } else {
                        shortcuts.push(fixed_key.clone());
                    }
                }
                validations
            })
            .collect::<Vec<Vec<FieldError>>>()
            .iter()
            .flat_map(|p| p.to_owned())
            .collect::<Vec<FieldError>>()
    }
}

impl ValidateSetting for AppSettings {
    fn validate(&self) -> Vec<FieldError> {
        [&self.custom_queries]
            .iter()
            .map(|p| p.validate())
            .collect::<Vec<Vec<FieldError>>>()
            .iter()
            .flat_map(|p| p.to_owned())
            .collect::<Vec<FieldError>>()
    }
}

#[cfg(test)]
mod tests {
    use std::{io::{Seek, Write}, path::PathBuf, sync::{Arc, Mutex}};

    use crate::core::cache::FileMnemonicsCache;

    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_default() {
        let cq = AppSettings::default();
        assert_eq!(cq.custom_queries.len(), 0);
    }

    #[test]
    fn test_config_file() {
        let mut file1 = NamedTempFile::with_suffix(".toml").expect("Cannot create named temp files.");
        // let file1_pb = PathBuf::from(file1.path());
        let _ = file1.as_file().set_len(0);
        let _ = file1.seek(std::io::SeekFrom::Start(0));
        let data = String::from("[custom_queries]\n\n[custom_queries.one_query]\nquery = \"end:20250502T043247Z limit:5\"\ndescription = \"report of something\"\n\n[custom_queries.two_query]\nquery = \"limit:1\"\ndescription = \"report of another thing\"\nfixed_key = \"ni\" # this will override randomly generated key\n\n");
        let _ = file1.write_all(data.as_bytes());
        let _ = file1.flush();
        
        let appconf = AppSettings::new(file1.path());
        println!("{:?}", appconf);
        assert_eq!(appconf.is_ok(), true);
        let appconf = appconf.unwrap();
        assert_eq!(appconf.custom_queries.len(), 2);
        assert_eq!(appconf.custom_queries.contains_key("one_query"), true);
        assert_eq!(appconf.custom_queries.contains_key("two_query"), true);
    }

    #[test]
    fn test_config_file_syntax() {
        let mut file1 = NamedTempFile::with_suffix(".toml").expect("Cannot create named temp files.");
        // let file1_pb = PathBuf::from(file1.path());
        let _ = file1.as_file().set_len(0);
        let _ = file1.seek(std::io::SeekFrom::Start(0));
        let data = String::from("[custom_queries]\nquery = \"end:20250502T043247Z limit:5\"\ndescription = \"report of something\"\n");
        let _ = file1.write_all(data.as_bytes());
        let _ = file1.flush();
        
        let appconf = AppSettings::new(file1.path());
        assert_eq!(appconf.is_err(), true);
    }

    #[test]
    fn test_config_file_validation() {
        let mut file1 = NamedTempFile::with_suffix(".toml").expect("Cannot create named temp files.");
        // let file1_pb = PathBuf::from(file1.path());
        let _ = file1.as_file().set_len(0);
        let _ = file1.seek(std::io::SeekFrom::Start(0));
        let data = String::from("[custom_queries]\n\n[custom_queries.one_query]\nquery = \"end:20250502T043247Z limit:5\"\ndescription = \"report of something\"\nfixed_key = \"ni\"\n\n[custom_queries.two_query]\nquery = \"limit:1\"\ndescription = \"report of another thing\"\nfixed_key = \"ni\" # this will override randomly generated key\n\n");
        let _ = file1.write_all(data.as_bytes());
        let _ = file1.flush();
        
        let appconf = AppSettings::new(file1.path());
        assert_eq!(appconf.is_err(), true);
    }

    #[test]
    fn test_config_validation() {
        let mut appconf = AppSettings::default();

        appconf.custom_queries.insert(
            String::from("two_query"), 
            CustomQuery { query: String::from("limit:1"), description: String::from("report of another thing"), fixed_key: Some(String::from("ni")) }
        );
        assert_eq!(appconf.custom_queries.len(), 1);

        appconf.custom_queries.insert(
            String::from("third_query"), 
            CustomQuery { query: String::from("limit:10"), description: String::from("Simple query"), fixed_key: Some(String::from("ni")) }
        );
        let valid = appconf.validate();
        assert_eq!(valid.len(), 1);

        // Lets add further query with only a fixed_key with less than one char.
        appconf.custom_queries.insert(
            String::from("fourth_query"), 
            CustomQuery { query: String::from("project:TWK"), description: String::from("Simple query #4"), fixed_key: Some(String::from("n")) }
        );
        let valid = appconf.validate();
        assert_eq!(valid.len(), 2);
    }

    #[test]
    fn test_config_register_shortcut() {
        let mut appconf = AppSettings::default();

        let file1 = NamedTempFile::new().expect("Cannot create named temp files.");
        let file_mtx = Arc::new(Mutex::new(PathBuf::from(file1.path())));

        let mut mock = FileMnemonicsCache::new(file_mtx);

        let result = mock.insert(MnemonicsType::CustomQuery, "two_query", "ad", false);
        assert_eq!(result.is_ok(), true);
        assert_eq!(mock.get(MnemonicsType::CustomQuery, "two_query"), Some(String::from("ad")));

        appconf.custom_queries.insert(
            String::from("two_query"), 
            CustomQuery { query: String::from("limit:1"), description: String::from("report of another thing"), fixed_key: Some(String::from("ni")) }
        );
        appconf.custom_queries.insert(
            String::from("third_query"), 
            CustomQuery { query: String::from("limit:10"), description: String::from("Simple query"), fixed_key: None }
        );

        appconf.register_shortcuts(&mut mock);

        assert_eq!(mock.get(MnemonicsType::CustomQuery, "two_query"), Some(String::from("ni")));
        assert_eq!(mock.get(MnemonicsType::CustomQuery, "third_query"), None);
    }
}