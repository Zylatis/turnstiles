use rand::{distributions::Alphanumeric, thread_rng, Rng};
/// Code for a TempDir struct to enable creating temporary, randomly named, directories for testing.
/// Future work: make this an in-mem filesystem instead, maybe?
use std::{
    fs::{create_dir_all, remove_dir_all},
    iter,
};
const N_DIR_NAME_CHARS: usize = 7;

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
            .take(N_DIR_NAME_CHARS)
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
