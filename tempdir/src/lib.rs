/// Code for a TempDir struct to enable creating temporary, randomly named, directories for testing.
/// Future work: make this an in-mem filesystem instead, maybe?
use std::{
    collections::HashSet,
    fs::{create_dir_all, read_dir, remove_dir_all},
    iter,
};

use rand::{distributions::Alphanumeric, thread_rng, Rng};

/// Temporary directory with a random name. When the struct is dropped, the directory and its contents are deleted
pub struct TempDir {
    pub path: String,
}
impl TempDir {
    pub fn new() -> Self {
        let mut rng = thread_rng();
        let chars: String = iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .map(char::from)
            .take(7)
            .collect();
        let path = chars;
        create_dir_all(&path).unwrap();
        Self { path }
    }

    fn clear(&self) {
        remove_dir_all(&self.path).unwrap_or(());
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        self.clear();
    }
}

// Some helpers
pub fn get_dir_files_hashset(dir: &str) -> HashSet<String> {
    let mut files = HashSet::new();
    for file in read_dir(dir).unwrap() {
        let filename = file.unwrap().file_name().to_str().unwrap().to_string();
        files.insert(filename);
    }
    files
}

pub fn assert_correct_files(dir: &str, log_filenames: Vec<&str>) {
    // TODO: change to ref of vec, prob doesn't need ownership
    // TODO: fix this complete shitshow
    let log_files: HashSet<String> = get_dir_files_hashset(dir);
    let log_files_str: HashSet<&str> = log_files.iter().map(AsRef::as_ref).collect();
    let expected: HashSet<&str> = log_filenames.into_iter().collect();

    assert_eq!(log_files_str, expected);
}
