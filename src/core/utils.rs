use std::collections::HashSet;
use rand::distr::{Alphanumeric, SampleString};

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