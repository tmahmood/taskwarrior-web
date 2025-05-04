use std::collections::HashSet;
use rand::distr::{Alphanumeric, SampleString};
use tracing::debug;

use super::{app::AppState, cache::MnemonicsType};

pub fn make_shortcut(shortcuts: &mut HashSet<String>) -> String {
    let alpha = Alphanumeric::default();
    let mut len = 2;
    let mut tries = 0;
    loop {
        let shortcut = alpha.sample_string(&mut rand::rng(), len).to_lowercase();
        if !shortcuts.contains(&shortcut) {
            shortcuts.insert(shortcut.clone());
            return shortcut;
        }
        tries += 1;
        if tries > 1000 {
            len += 1;
            if len > 3 {
                panic!("too many shortcuts! this should not happen");
            }
            tries = 0;
        }
    }
}

pub fn make_shortcut_cache(mn_type: MnemonicsType, key: &str, app_state: &AppState) -> String {
    let alpha = Alphanumeric::default();
    let mut len = 2;
    let mut tries = 0;
    loop {
        let shortcut = alpha.sample_string(&mut rand::rng(), len).to_lowercase();
        let shortcut_insert = app_state.app_cache.write().unwrap().insert(mn_type.clone(), key, &shortcut);
        if shortcut_insert.is_ok() {
            return shortcut;
        } else {
            debug!("Failed generating and saving shortcut {} - error: {:?}", shortcut, shortcut_insert.err());
        }
        tries += 1;
        if tries > 1000 {
            len += 1;
            if len > 3 {
                panic!("too many shortcuts! this should not happen");
            }
            tries = 0;
        }
    }
}