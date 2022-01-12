#![warn(clippy::panic, clippy::expect_used, clippy::unwrap_used)]
/*!
Library which defines a struct implementing the io::Write trait which will allows file rotation, if applicable, when a file write is done.
Currently this library only supports rotation by creating new files when a rotation is required, rather than renaming existing files.
For example if `my_file.log` is given then when the first rotation occurs a file with the name `my_file.log.1` will be created and written to.
This means the latest file has the highest index, not the original filename. This is done to minimize surface area with the filesystem, but it
is part of future work to potentially include the case where `my_file.log` is always the latest.

# Examples
Rotate when a log file exceeds a certain filesize

```
use std::{io::Write, thread::sleep, time::Duration};
use turnstiles::{RotatingFile, RotationOption};

use tempdir::TempDir; // Subcrate provided for testing
let dir = TempDir::new();

let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
let data: Vec<u8> = vec![0; 500_000];
let mut file = RotatingFile::new(path, RotationOption::SizeMB(1), false).unwrap();

// Write 500k to file creating test.log
file.write(&data).unwrap();
assert!(file.index() == 0);

// Write another 500kb so test.log is 1mb
file.write_all(&data).unwrap();
assert!(file.index() == 0);

// The check for rotation is done _before_ writing, so we don't rotate, and then write 500kb
// so this file is 1.5mb now, still the same file
file.write_all(&data).unwrap();
assert!(file.index() == 0);


// Now we check if we need to rotate before writing, and it's 1.5mb > the rotation option so
// we make a new file and  write to that
file.write_all(&data).unwrap();
assert!(file.index() == 1);

// Now have test.log and test.log.1
```

Rotate when a log file is too old (based on filesystem metadata timestamps)

```
use std::{io::Write, thread::sleep, time::Duration};
use turnstiles::{RotatingFile, RotationOption};

use tempdir::TempDir; // Subcrate provided for testing
 let dir = TempDir::new();
let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");

let max_log_age = Duration::from_millis(100);
let data: Vec<u8> = vec![0; 1_000_000];
let mut file =
    RotatingFile::new(path, RotationOption::Duration(max_log_age), false).unwrap();

assert!(file.index() == 0);
file.write_all(&data).unwrap();
assert!(file.index() == 0);
file.write_all(&data).unwrap();
assert!(file.index() == 0);
sleep(Duration::from_millis(200));

// Rotation only happens when we call .write() so index remains unchanged after this duration
// even though it exceeds that given in the RotationOption
assert!(file.index() == 0);
// Bit touch and go but assuming two writes of 1mb bytes doesn't take 100ms!
file.write_all(&data).unwrap();
assert!(file.index() == 1);
file.write_all(&data).unwrap();
assert!(file.index() == 1);

// Will now have test.log and test.log.1
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

// TODO: template this maybe? Or just make it u128 and fugheddaboutit?
type FileIndexInt = u32;
const ACTIVE_PREFIX: &'static str = "ACTIVE_";
#[derive(Debug)]
/// Struct masquerades as a file handle and is written to by whatever you like
pub struct RotatingFile {
    filename_root: String,

    rotation: RotationOption,
    current_file: File,
    index: FileIndexInt,
    require_newline: bool, // Should be type to avoid runtime cost?
}

impl RotatingFile {
    /// Create a new RotatingFile given a desired filename and rotation option. The filename represents the stem or root of the files
    /// to be generated.
    pub fn new(path_str: &str, rotation: RotationOption, require_newline: bool) -> Result<Self> {
        // TODO: throw error if path_str (rootname) ends in digit as this will break the numbering stuff
        let (path_filename, parent) = filename_to_details(path_str)?;
        let current_index = Self::detect_latest_file_index(&path_filename, &parent)? + 1;
        let current_file_path = format!("{}/{}{}", parent, ACTIVE_PREFIX, path_filename);
        dbg!(&current_file_path);
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(current_file_path)?;
        Ok(Self {
            rotation,
            current_file: file,
            index: current_index,
            filename_root: path_str.to_string(),
            require_newline,
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
    pub fn index(&self) -> FileIndexInt {
        self.index
    }
    /// Given a filename stem and folder path find the highest index so where know where to pick up after we left off in a previous incarnation
    fn detect_latest_file_index(filename: &str, folder_path: &str) -> Result<FileIndexInt> {
        let log_files = Self::list_log_files(filename, folder_path)?;
        let mut max_index = 0;
        for filename_string in log_files {
            if filename_string == filename {
                continue;
            } else {
                let file_index = match filename_string.split('.').last() {
                    None => bail!("Found log file ending in '.', can't process index."),
                    Some(s) => s,
                };

                let i = file_index.parse::<FileIndexInt>()?;
                max_index = cmp::max(i, max_index);
            }
        }
        Ok(max_index)
    }

    /// Perform file rotation
    fn rotate_current_file(&mut self) -> Result<(), std::io::Error> {
        // TODO: think about if we want to be more careful here, i.e. append to a random file which may already exist and be a totally different format?
        // Could throw an exception, or print a warning and skip that file index. Who logs the loggers...
        let new_file = &format!("{}.{}", self.filename_root, self.index + 1);
        self.current_file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(new_file)?;
        self.index += 1; // Only do this once the above results have passed.
        Ok(())
    }

    /// Given the RotationOption chosen when the struct was created, check if a rotation is in order
    /// NOTE: this currently does no check to see if the file rotation option has changed for a given set of logs, but this will never result in dataloss
    /// just maybe some confusingly-sized logs
    fn rotation_required(&mut self) -> Result<bool, std::io::Error> {
        let rotate = match self.rotation {
            RotationOption::None => false,
            RotationOption::SizeMB(size) => self.file_metadata()?.len() > size * 1_048_576,
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
        if !self.require_newline {
            if self.rotation_required()? {
                self.rotate_current_file()?;
            }
        } else if let Some(last_char) = bytes.last() {
            // Note this will prevent writing just a newline and so could break some stuff
            // TODO: be smarter here in future, not sure how best to distinguish between genuine newline write and broken up log from slog async
            if *last_char == b'\n' && self.rotation_required()? {
                self.rotate_current_file()?;
                if bytes.len() != 1 {
                    self.current_file.write_all(bytes)?;
                }
                return Ok(bytes.len());
            }
        }

        self.current_file.write_all(bytes)?;
        Ok(bytes.len())
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
