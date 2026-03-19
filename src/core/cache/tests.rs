/*
 * Copyright 2026 Tarin Mahmood
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

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
    assert_eq!(mock.get(&MnemonicsType::PROJECT, "personal"), None);
    assert!(
        mock.insert(&MnemonicsType::TAG, "personal", "xz", false)
            .is_ok()
    );
    assert_eq!(
        mock.get(&MnemonicsType::TAG, "personal"),
        Some(String::from("xz"))
    );
    // how to validate content?
    file1.reopen().expect("Cannot reopen");
    let mut buf = String::new();
    let read_result = file1.read_to_string(&mut buf);
    assert!(read_result.is_ok());
    let read_result = read_result.expect("Could not read from file");
    assert!(read_result > 0);
    assert_eq!(buf, String::from("[tags]\npersonal = \"xz\"\n"));
    assert!(
        mock.insert(&MnemonicsType::PROJECT, "taskwarrior", "xz", false)
            .is_err(),
    );
    assert!(mock.remove(&MnemonicsType::TAG, "personal").is_ok());
    assert_eq!(mock.get(&MnemonicsType::TAG, "personal"), None);
    assert!(
        mock.insert(&MnemonicsType::PROJECT, "taskwarrior", "xz", false)
            .is_ok(),
    );
    assert!(
        mock.insert(&MnemonicsType::TAG, "personal", "xz", false)
            .is_err(),
    );
    assert!(mock.remove(&MnemonicsType::PROJECT, "taskwarrior").is_ok());
    file1.reopen().expect("Cannot reopen");
    let _ = file1.as_file().set_len(0);
    let _ = file1.seek(std::io::SeekFrom::Start(0));
    let data = String::from("[tags]\npersonal = \"xz\"\n\n[projects]\n");
    let _ = file1.write_all(data.as_bytes());
    let _ = file1.flush();
    assert!(mock.load().is_ok());
    assert_eq!(
        mock.get(&MnemonicsType::TAG, "personal"),
        Some(String::from("xz"))
    );
    file1.reopen().expect("Cannot reopen");
    let _ = file1.as_file().set_len(0);
    let _ = file1.seek(std::io::SeekFrom::Start(0));
    let data = String::from("**********");
    let _ = file1.write_all(data.as_bytes());
    let _ = file1.flush();
    assert!(mock.load().is_err());
    // Empty file cannot be parsed, but should not through an error!
    let _ = file1.as_file().set_len(0);
    let _ = file1.seek(std::io::SeekFrom::Start(0));
    let _ = file1.flush();
    assert!(mock.load().is_ok());
    // If the configuration file does not exist yet (close will delete),
    // it is fine as well.
    let _ = file1.close();
    assert!(mock.load().is_ok());
}

#[test]
fn test_custom_queries() {
    let mut file1 = NamedTempFile::new().expect("Cannot create named temp files.");
    let x = PathBuf::from(file1.path());
    let file_mtx = Arc::new(Mutex::new(x));
    let mut mock = FileMnemonicsCache::new(file_mtx);

    // Check for retrieving custom query shortcuts.
    assert_eq!(mock.get(&MnemonicsType::CustomQuery, "one_query"), None);

    // Insert a one_query shortcut and verify, that the query shortcut
    // is saved.
    assert!(
        mock.insert(&MnemonicsType::CustomQuery, "one_query", "ad", false)
            .is_ok()
    );
    assert_eq!(
        mock.get(&MnemonicsType::CustomQuery, "one_query"),
        Some(String::from("ad"))
    );

    // Save to file and ensure, its proper written.
    let mut buf = String::new();
    let read_result = file1.read_to_string(&mut buf);
    assert!(read_result.is_ok());
    let read_result = read_result.expect("Could not read from file");
    assert!(read_result > 0);
    assert_eq!(buf, String::from("[custom_queries]\none_query = \"ad\"\n"));

    // Delete again.
    assert!(
        mock.remove(&MnemonicsType::CustomQuery, "one_query")
            .is_ok()
    );
    assert_eq!(mock.get(&MnemonicsType::CustomQuery, "one_query"), None);

    // Test overwriting of queries.
    file1.reopen().expect("Cannot reopen");
    let _ = file1.as_file().set_len(0);
    let _ = file1.seek(std::io::SeekFrom::Start(0));
    let data = String::from("[custom_queries]\none_query = \"ad\"\n");
    let _ = file1.write_all(data.as_bytes());
    let _ = file1.flush();
    assert!(mock.load().is_ok());
    // Add a second query and ensure, that the one_query gets removed.
    assert!(
        mock.insert(&MnemonicsType::CustomQuery, "second_query", "ad", true)
            .is_ok()
    );
    assert_eq!(mock.get(&MnemonicsType::CustomQuery, "one_query"), None);
    assert_eq!(
        mock.get(&MnemonicsType::CustomQuery, "second_query"),
        Some(String::from("ad"))
    );
    // Ensure, an error is produced in case its not overwritten.
    assert!(
        mock.insert(&MnemonicsType::CustomQuery, "one_query", "ad", false)
            .is_err()
    );
}

#[test]
fn test_mnemonics_cache_file_fail() {
    let x = PathBuf::from_str("/4bda0a6b-da0d-46be-98e6-e06d43385fba/asdfa.cache").unwrap();
    let file_mtx = Arc::new(Mutex::new(x));

    let mut mock = FileMnemonicsCache::new(file_mtx);
    assert!(
        mock.insert(&MnemonicsType::TAG, "personal", "xz", false)
            .is_err()
    );
    assert!(mock.remove(&MnemonicsType::PROJECT, "taskwarrior").is_err());
}
