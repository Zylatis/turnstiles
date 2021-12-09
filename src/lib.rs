use anyhow::{bail, Result};
use std::{cmp, fs};
use std::{
    fs::{File, Metadata, OpenOptions},
    io,
    time::Duration,
};
mod utils;
use utils::{filename_to_details, safe_unwrap_osstr};
#[derive(Debug)]
pub struct RotatingFile {
    filename: String,
    parent: String,
    rotation: RotationOption,
    current_file: File,
    index: u32,
}

impl RotatingFile {
    pub fn new(path_str: &str, rotation: RotationOption) -> Result<Self> {
        let (path_filename, parent) = filename_to_details(path_str)?;
        let current_index = Self::detect_latest_file_index(&path_filename, &parent)?;
        let filename = if current_index != 0 {
            format!("{}.{}", path_filename, current_index)
        } else {
            path_filename
        };

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(path_str)?;
        Ok(Self {
            rotation,
            current_file: file,
            index: current_index,
            filename,
            parent,
        })
    }

    fn list_log_files(filename: &String, path: &String) -> Result<Vec<String>> {
        let files = fs::read_dir(&path)?;
        let mut log_files = vec![];
        for f in files {
            let filename_str = safe_unwrap_osstr(&f?.file_name())?;
            if filename_str.contains(filename) {
                log_files.push(filename_str);
            }
        }
        Ok(log_files)
    }

    pub fn index(&self) -> u32 {
        self.index
    }
    fn detect_latest_file_index(filename: &String, path: &String) -> Result<u32> {
        let log_files = Self::list_log_files(filename, path).unwrap();
        let mut max_index = 0;
        for filename_string in log_files {
            let file_index = match filename_string.split(".").last() {
                None => bail!("Found log file ending in '.', can't process index."),
                Some(s) => s,
            };
            if file_index == "" {
                continue;
            } else {
                let i = file_index.parse::<u32>()?;
                max_index = cmp::max(i, max_index);
            }
        }
        Ok(max_index)
    }

    fn rotate_current_file(&mut self) -> Result<(), std::io::Error> {
        self.index += 1;
        let new_file = &format!("{}/{}.{}", self.parent, self.filename, self.index);
        self.current_file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(new_file)?;
        Ok(())
    }

    fn rotate(&mut self) -> Result<bool, std::io::Error> {
        let rotate = match self.rotation {
            RotationOption::None => false,
            RotationOption::SizeMB(size) => self.file_metadata()?.len() * 1_000_000 > size,
            // RotationOption::SizeLines(len) => false,
            RotationOption::Duration(duration) => {
                match self.file_metadata()?.created()?.elapsed() {
                    Ok(elapsed) => elapsed > duration,
                    Err(e) => {
                        println!("Warning: failed to determine time since log file created, got error {}. Rotating anyway as a precaution.", e);
                        true
                    }
                }
            }
        };
        Ok(rotate)
    }
    fn file_metadata(&self) -> Result<Metadata, std::io::Error> {
        self.current_file.sync_all()?;
        self.current_file.metadata()
    }
}
impl io::Write for RotatingFile {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, std::io::Error> {
        if self.rotate()? {
            self.rotate_current_file()?;
        }
        self.current_file.write(bytes)
    }
    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.current_file.flush()
    }
}
#[derive(Debug)]
pub enum RotationOption {
    None,
    SizeMB(u64),
    // SizeLines(u64),
    Duration(Duration),
}

#[cfg(test)]
mod tests {
    use rand::{distributions::Alphanumeric, thread_rng, Rng};
    use std::{
        fs::{create_dir_all, remove_dir_all},
        io::Write,
        iter,
        thread::sleep,
        time::Duration,
    };
    struct TempDir {
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
            let path = chars.to_string();
            create_dir_all(&path).unwrap();
            Self { path: path }
        }

        fn clear(&self) {
            remove_dir_all(&self.path).unwrap_or(());
        }
    }

    use crate::{RotatingFile, RotationOption};

    #[cfg(test)]
    impl Drop for TempDir {
        fn drop(&mut self) {
            // This seems highly dangerous, were it to ever be moved out of test it would delete everyones logs
            // Better to specify a temp directory and have it on that drop
            self.clear();
        }
    }

    #[test]
    fn test_file_size() {
        let dir = TempDir::new();
        let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
        let data: Vec<u8> = vec![0; 1_000_000];
        let mut file = RotatingFile::new(path, RotationOption::SizeMB(1)).unwrap();
        assert!(file.index() == 0);
        file.write(&data).unwrap(); //write 1mb to file
        file.write(&data).unwrap(); //write 1mb to file
        assert!(file.index() == 1);
        file.write(&data).unwrap(); //write 1mb to file
        assert!(file.index() == 2); // should have 3 files now
    }

    #[test]
    fn test_file_duration() {
        let dir = TempDir::new();
        let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");

        let data: Vec<u8> = vec!["a"; 100_000].join("").as_bytes().to_vec();
        let mut file =
            RotatingFile::new(path, RotationOption::Duration(Duration::from_millis(100))).unwrap();
        file.write(&data).unwrap();
        file.write(&data).unwrap();
        sleep(Duration::from_millis(200));
        file.write(&data).unwrap();
        assert!(file.index() == 1);
    }

    #[test]
    #[should_panic]
    fn test_file_duration_delay_fail() {
        let dir = TempDir::new();
        let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");

        let data: Vec<u8> = vec!["a"; 100_000].join("").as_bytes().to_vec();
        let mut file =
            RotatingFile::new(path, RotationOption::Duration(Duration::from_millis(100))).unwrap();
        sleep(Duration::from_millis(200)); // the constructor makes the file and so the timer starts from then, this should cause it to fail
        file.write(&data).unwrap();
        file.write(&data).unwrap();
        sleep(Duration::from_millis(200));
        file.write(&data).unwrap();
        assert!(file.index() == 1);
    }
}
