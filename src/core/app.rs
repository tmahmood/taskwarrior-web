use std::{env::{self, home_dir}, path::PathBuf, str::FromStr};
use tera::Context;

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
/// 
#[derive(Clone, Debug)]
pub struct AppState {
    pub font: Option<String>,
    pub fallback_family: String,
    pub theme: Option<String>,
    pub display_time_of_the_day: i32,
    pub task_storage_path: PathBuf,
    pub task_hooks_path: Option<PathBuf>,
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

        Self {
            font: font,
            fallback_family: "monospace".to_string(),
            theme: theme,
            display_time_of_the_day: display_time_of_the_day,
            task_storage_path: task_storage_path,
            task_hooks_path: task_hooks_path,
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
