use std::path::PathBuf;
use std::{cmp, fs};
use std::{
    fs::{DirEntry, File, Metadata, OpenOptions},
    io,
    time::Duration,
};
// mod rng;
#[derive(Debug)]
pub struct RotatingFile {
    filename: String,
    parent: PathBuf,
    rotation: RotationOption,
    current_file: File,
    index: u32,
}

impl RotatingFile {
    pub fn new(path_str: &str, rotation: RotationOption) -> Result<Self, std::io::Error> {
        let path = PathBuf::from(path_str);
        let mut path_str = path_str.to_owned();
        let current_index = Self::detect_latest_file_index(&path);
        if current_index != 0 {
            path_str += &format!(".{}", current_index);
        }

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(path_str)?;
        Ok(Self {
            rotation,
            current_file: file,
            index: current_index,
            filename: path.file_name().unwrap().to_str().unwrap().to_string(),
            parent: path.parent().unwrap().to_path_buf(),
        })
    }

    fn list_log_files(path: &PathBuf) -> Vec<DirEntry> {
        let dir = match path.parent() {
            None => "/",
            Some(s) => match s.to_str().unwrap() {
                "" => ".",
                s => s,
            },
        };

        let files = fs::read_dir(&dir).unwrap().map(|x| x.unwrap());
        let mut log_files = vec![];
        let prefix = path.file_name().unwrap().to_str().unwrap();
        for f in files {
            if f.file_name().to_str().unwrap().contains(prefix) {
                log_files.push(f);
            }
        }
        log_files
    }

    pub fn index(&self) -> u32 {
        self.index
    }
    fn detect_latest_file_index(path: &PathBuf) -> u32 {
        let log_files = Self::list_log_files(path);
        let mut max_index = 0;
        for f in log_files {
            // JFC...
            let fname = f.file_name();
            let i_str = fname
                .to_str()
                .unwrap()
                .to_string()
                .replace(path.file_name().unwrap().to_str().unwrap(), "");
            let i_str = i_str.split(".").clone().last().clone().unwrap();

            if i_str == "" {
                continue;
            } else {
                let i = i_str.parse::<u32>().unwrap();
                max_index = cmp::max(i, max_index);
            }
        }
        max_index
    }

    fn rotate_current_file(&mut self) {
        self.index += 1;
        let new_file = &format!(
            "{}/{}.{}",
            self.parent.to_str().unwrap(),
            self.filename,
            self.index
        );
        self.current_file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(new_file)
            .unwrap();
    }

    fn rotate(&mut self) -> bool {
        match self.rotation {
            RotationOption::None => false,
            RotationOption::SizeMB(size) => self.file_metadata().unwrap().len() * 1_000_000 > size,
            // RotationOption::SizeLines(len) => false,
            RotationOption::Duration(duration) => {
                self.file_metadata()
                    .unwrap()
                    .created()
                    .unwrap()
                    .elapsed()
                    .unwrap()
                    > duration
            }
        }
    }
    fn file_metadata(&self) -> Result<Metadata, std::io::Error> {
        self.current_file.sync_all()?;
        self.current_file.metadata()
    }
}
impl io::Write for RotatingFile {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, std::io::Error> {
        if self.rotate() {
            self.rotate_current_file();
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
