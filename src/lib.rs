#![warn(clippy::panic, clippy::expect_used, clippy::unwrap_used)]
/*!
Library which defines a struct implementing the io::Write trait which will allows file rotation, if applicable, when a file write is done.
Currently this library only supports rotation by creating new files when a rotation is required, rather than renaming existing files.
For example if "my_file.log" is given then when the first rotation occurs this will be renamed "my_file.log.1". This means the latest file has the highest
index, not the original filename. This is done to minimize surface area with the filesystem but is part of the future work.

# Examples
Rotate when a log file exceeds a certain filesize

```
let some_bytes: Vec<u8> = vec![0; 1_000_000];
let mut log_file =
    RotatingFile::new("logs/super_important_service.log", RotationOption::SizeMB(500))
    .expect("failed to create RotatingFile");
file.write(&some_bytes).expect("Failed to write bytes to file");
```

Rotate when a log file is too old (based on filesystem metadata timestamps)

```
let max_log_age = Duration::from_secs(3600);
let some_bytes: Vec<u8> = vec![0; 10_000_000];
let mut log_file =
    RotatingFile::new("logs/super_important_service.log", RotationOption::Duration(max_log_age))
    .expect("failed to create RotatingFile");
file.write(&some_bytes).expect("Failed to write bytes to file");
```


*/
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
/// Struct masquerades as a file handle and is written to by whatever you like
pub struct RotatingFile {
    filename: String,
    parent: String,
    rotation: RotationOption,
    current_file: File,
    index: u32,
}

impl RotatingFile {
    /// Create a new RotatingFile given a desired filename and rotation option. The filename represents the stem or root of the files
    /// to be generated.
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

    /// Given a filename stem and folder path, list all files which contain the filename stem.
    /// Note: this currently literally does a .contains() check rather than verifying more carefully, but this a TODO.
    fn list_log_files(filename: &str, folder_path: &str) -> Result<Vec<String>> {
        let files = fs::read_dir(&folder_path)?;
        let mut log_files = vec![];
        for f in files {
            let filename_str = safe_unwrap_osstr(&f?.file_name())?;
            if filename_str.contains(filename) {
                log_files.push(filename_str);
            }
        }
        Ok(log_files)
    }

    /// A read-only wrapper to the index, at the moment only for testing purposes.
    pub fn index(&self) -> u32 {
        self.index
    }
    /// Given a filename stem and folder path find the highest index so where know where to pick up after we left off in a previous incarnation
    fn detect_latest_file_index(filename: &str, folder_path: &str) -> Result<u32> {
        let log_files = Self::list_log_files(filename, folder_path)?;
        let mut max_index = 0;
        for filename_string in log_files {
            let file_index = match filename_string.split('.').last() {
                None => bail!("Found log file ending in '.', can't process index."),
                Some(s) => s,
            };
            if file_index.is_empty() {
                continue;
            } else {
                let i = file_index.parse::<u32>()?;
                max_index = cmp::max(i, max_index);
            }
        }
        Ok(max_index)
    }

    /// Perform file rotation
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

    /// Given the RotationOption chosen when the struct was created, check if a rotation is in order
    /// NOTE: this currently does no check to see if the file rotation option has changed for a given set of logs, but this will never result in dataloss
    /// just maybe some confusingly-sized logs
    fn rotation_required(&mut self) -> Result<bool, std::io::Error> {
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
        if self.rotation_required()? {
            self.rotate_current_file()?;
        }
        self.current_file.write(bytes)
    }
    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.current_file.flush()
    }
}

/// Enum for possible file rotation options.
#[derive(Debug)]
pub enum RotationOption {
    None,
    SizeMB(u64),
    // SizeLines(u64),
    Duration(Duration),
}
