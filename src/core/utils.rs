/*
 * Copyright 2025 Tarin Mahmood
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

use rand::distr::{Alphanumeric, SampleString};
use std::collections::HashSet;
use tracing::trace;

use super::{app::AppState, cache::MnemonicsType};

pub fn make_shortcut(shortcuts: &mut HashSet<String>) -> String {
    let mut len = 2;
    let mut tries = 0;
    loop {
        let shortcut = Alphanumeric
            .sample_string(&mut rand::rng(), len)
            .to_lowercase();
        if !shortcuts.contains(&shortcut) {
            shortcuts.insert(shortcut.clone());
            return shortcut;
        }
        tries += 1;
        if tries > 1000 {
            len += 1;
            // TODO: This can happen if we have many shortcuts in the long run, need to implement cache cleanup
            assert!(len <= 3, "too many shortcuts! this should not happen");
            tries = 0;
        }
    }
}

pub fn make_shortcut_cache(mn_type: MnemonicsType, key: &str, app_state: &AppState) -> String {
    let mut len = 2;
    let mut tries = 0;
    // Check if available in the cache.
    let shortcut_cache = app_state
        .app_cache
        .read()
        .unwrap()
        .get(mn_type.clone(), key);
    if let Some(shortcut_cache) = shortcut_cache {
        return shortcut_cache;
    }
    loop {
        let shortcut = Alphanumeric
            .sample_string(&mut rand::rng(), len)
            .to_lowercase();
        if app_state
            .app_cache
            .write()
            .unwrap()
            .insert(mn_type.clone(), key, &shortcut, false)
            .is_err()
        {
            tries += 1;
            if tries > 1000 {
                len += 1;
            }
            if len > 3 {
                panic!("too many shortcuts! this should not happen");
            }
            continue;
        }
        trace!(
            "Searching shortcut for type {:?} with key {} and found {}",
            &mn_type, key, &shortcut
        );
        return shortcut;
    }
}
