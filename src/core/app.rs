/*
 * Copyright 2025 Tarin Mahmood
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

use directories::ProjectDirs;
use std::{
    env::{self, home_dir},
    fs::create_dir_all,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex, RwLock},
};
use tera::Context;
use tracing::info;

use super::{
    cache::{FileMnemonicsCache, MnemonicsCacheType},
    config::AppSettings,
};

/// Holds state information and configurations
/// required in the API and business logic operations.
///
/// # Environments
/// Many of the options are configured via environment variables.
/// Following are supported:
/// | Environment variable      | AppState field           |
/// |---------------------------|--------------------------|
/// | TWK_USE_FONT              | font                     |
/// | TWK_THEME                 | theme                    |
/// | DISPLAY_TIME_OF_THE_DAY   | display_time_of_the_day  |
/// | TASKDATA                  | task_storage_path        |
/// | TWK_CONFIG_FOLDER         | app_config_path          |
/// | TWK_SYNC                  | interval in seconds      |
///
#[derive(Clone)]
pub struct AppState {
    pub font: Option<String>,
    pub fallback_family: String,
    pub theme: Option<String>,
    pub display_time_of_the_day: i32,
    pub task_storage_path: PathBuf,
    pub task_hooks_path: Option<PathBuf>,
    pub app_config_path: PathBuf,
    pub app_cache_path: PathBuf,
    pub app_cache: Arc<RwLock<MnemonicsCacheType>>,
    pub app_config: Arc<AppSettings>,
    pub sync_interval: i64,
    // Here must be cache object for mnemonics
}

impl Default for AppState {
    fn default() -> Self {
        let font = env::var("TWK_USE_FONT").map(|p| Some(p)).unwrap_or(None);
        let theme = match env::var("TWK_THEME") {
            Ok(p) if p.is_empty() => None,
            Ok(p) => Some(p),
            Err(_) => None,
        };
        let display_time_of_the_day = env::var("DISPLAY_TIME_OF_THE_DAY")
            .unwrap_or("0".to_string())
            .parse::<i32>()
            .unwrap_or(0);

        let home_dir = home_dir().unwrap_or_default();
        let home_dir = home_dir.join(".task");
        let task_storage_path =
            env::var("TASKDATA").unwrap_or(home_dir.to_str().unwrap_or("").to_string());
        let sync_interval = if let Ok(sync_interval) = env::var("TWK_SYNC") {
            i64::from_str(&sync_interval).unwrap_or_default()
        } else {
            0
        };
        let task_storage_path =
            PathBuf::from_str(&task_storage_path).expect("Storage path cannot be found");
        let task_hooks_path = Some(home_dir.clone().join("hooks"));

        let standard_project_dirs = ProjectDirs::from("", "", "Taskwarrior-Web");

        // Overall determination of the configuration files.
        let mut app_config_path: Option<PathBuf> = match env::var("TWK_CONFIG_FOLDER") {
            Ok(p) => Some(p.into()),
            Err(_) => None,
        };
        if app_config_path.is_none()
            && standard_project_dirs.is_some()
            && let Some(ref proj_dirs) = standard_project_dirs
        {
            app_config_path = Some(proj_dirs.config_dir().to_path_buf());
        }

        let app_config_path = app_config_path.expect("Configuration path not found");
        create_dir_all(app_config_path.as_path()).expect("Config folder cannot be created.");
        let app_config_path = app_config_path.join("config.toml");
        let app_settings = match AppSettings::new(app_config_path.as_path()) {
            Ok(s) => Ok(s),
            Err(e) => match e {
                config::ConfigError::Foreign(_) => {
                    info!(
                        "Configuration file could not be found ({}). Fallback to default.",
                        e.to_string()
                    );
                    Ok(AppSettings::default())
                }
                _ => Err(e),
            },
        }
        .expect("Proper configuration file does not exist");

        // Overall determination of the cache folder.
        let app_cache_path = standard_project_dirs
            .map(|p| p.cache_dir().to_path_buf())
            .expect("Cache folder not usable.");

        // initialize cache.
        // ensure, the folder exists.
        create_dir_all(&app_cache_path).expect("Cache folder cannot be created.");
        let cache_path = app_cache_path.join("mnemonics.cache");
        info!(
            "Cache file to store mnemonics is placed at {:?}",
            &cache_path
        );
        let mut cache = FileMnemonicsCache::new(Arc::new(Mutex::new(cache_path.clone())));
        cache
            .load()
            .inspect_err(|e| {
                tracing::error!(
                    "Cannot parse the configuration file, error: {}",
                    e.to_string()
                );
            })
            .expect("Configuration file exists, but is not parsable!");

        // Now ensure, that fixed keys are directly assigned to the custom queries.
        // For this, we need also to ensure, that conflicting cache entries are removed!
        app_settings.register_shortcuts(&mut cache);

        Self {
            font,
            fallback_family: "monospace".to_string(),
            theme,
            display_time_of_the_day,
            task_storage_path,
            task_hooks_path,
            app_config_path,
            app_cache_path,
            app_cache: Arc::new(RwLock::new(cache)),
            app_config: Arc::new(app_settings),
            sync_interval,
        }
    }
}

impl From<&AppState> for Context {
    fn from(val: &AppState) -> Self {
        let mut ctx = Context::new();
        ctx.insert("USE_FONT", &val.font);
        ctx.insert("FALLBACK_FAMILY", &val.fallback_family);
        ctx.insert("DEFAULT_THEME", &val.theme);
        ctx.insert("display_time_of_the_day", &val.display_time_of_the_day);
        ctx
    }
}

pub fn get_default_context(state: &AppState) -> Context {
    state.into()
}
