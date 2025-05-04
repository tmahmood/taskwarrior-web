use std::{env::{self, home_dir}, fs::create_dir_all, path::PathBuf, str::FromStr, sync::{Arc, Mutex, RwLock}};
use directories::ProjectDirs;
use tera::Context;
use tracing::info;

use super::cache::{FileMnemonicsCache, MnemonicsCacheType};

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

        let home_dir = home_dir().unwrap_or(PathBuf::default());
        let home_dir = home_dir.join(".task");
        let task_storage_path =
            env::var("TASKDATA").unwrap_or(home_dir.to_str().unwrap_or("").to_string());
        let task_storage_path = PathBuf::from_str(&task_storage_path)
            .expect("Storage path cannot be found");
        let task_hooks_path = Some(home_dir.clone().join("hooks"));

        let standard_project_dirs = ProjectDirs::from("", "",  "Taskwarrior-Web");
        
        let mut app_config_path: Option<PathBuf> = match env::var("TWK_CONFIG_FOLDER") {
            Ok(p) => {
                let app_config_path: Result<PathBuf, _> = p.try_into();
                match app_config_path {
                    Ok(x) => Some(x),
                    Err(_) => None
                }
            },
            Err(_) => None,
        };
        if app_config_path.is_none() && standard_project_dirs.is_some() {
            if let Some(ref proj_dirs) = standard_project_dirs {
                app_config_path = Some(proj_dirs.config_dir().to_path_buf());
            }
        }

        let app_config_path = app_config_path.expect("Configuration file found");
        let app_cache_path =  match standard_project_dirs {
            Some(p) => Some(p.cache_dir().to_path_buf()),
            None => None,
        }.expect("Cache folder not usable.");

        // initialize cache.
        // ensure, the folder exists.
        create_dir_all(app_cache_path.as_path()).expect("Cache folder cannot be created.");
        let cache_path = app_cache_path.join("mnemonics.cache");
        info!("Cache file to store mnemonics is placed at {:?}", &cache_path);
        let mut cache = FileMnemonicsCache::new(Arc::new(Mutex::new(cache_path)));
        cache.load().map_err(|e| {
            tracing::error!("Cannot parse the configuration file, error: {}", e.to_string());
            e
        }).expect("Configuration file exists, but is not parsable!");

        Self {
            font: font,
            fallback_family: "monospace".to_string(),
            theme: theme,
            display_time_of_the_day: display_time_of_the_day,
            task_storage_path: task_storage_path,
            task_hooks_path: task_hooks_path,
            app_config_path: app_config_path,
            app_cache_path: app_cache_path,
            app_cache: Arc::new(RwLock::new(cache)),
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
