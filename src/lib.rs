#![warn(clippy::panic, clippy::expect_used, clippy::unwrap_used)]
/*!
Library which defines a struct implementing the io::Write trait which will allows file rotation, if applicable, when a file write is done. 

## Warning
<p style="background:rgba(255,181,77,0.16);padding:0.75em;">
This is currently in active development and may change/break often. Every effort will be taken to ensure that breaking changes that occur are reflected in a change of at least the minor version of the package,
both in terms of the API and the generation of log files. Versions prior to 0.2.0 were so riddled with bugs I'm amazed I managed to put my pants on on those days I was writing it.
</p>

## How it works
Rotation works by keeping track of the 'active' file, the one currently being written to, which upon rotation is renamed to include the next log file index. For example when there is only one log file it will be `test.log.ACTIVE`, 
which when rotated will get renamed to `test.log.1` and the `test.log.ACTIVE` will represent a new file being written to. Originally no file renaming was done to keep the surface area with the filesystem as small as possible, 
however this has a few disadvantages and this active-file-approach (courtesy of [flex-logger](https://docs.rs/flexi_logger/latest/flexi_logger/)) was seen as a good compromise. The downside is that the file extension now superifically looks different, but it does mean all logs can be found by simply searching for `test.log*`. 

Log suffix numbers will increase with age, so the first of the rotated logs will be `test.log.1`, second will be `test.log.2` etc until `N-1` after which it will be `test.log.ACTIVE`, the current one.  

## Warning
<p style="background:rgba(255,181,77,0.16);padding:0.75em;">
Little to no protection is given defend against the file indices being modified during the operation of whatever code is using this logger: when `write` is called it does not currently refresh the internal index which tracks the suffix integer, this is only done when the logger is created. Changing this is noted as future work.
</p>

## Error handling
Not all internal errors are handled the same way. For example, if during the process of checking if rotation is required an error occurs, the default is to print a warning to stdout and _not_ rotate. In contrast to this, if an error occurs during the actual rotation procedure, this error is bubbled up through error handling eventually returning as a `std::io::Error` to the caller. However probable future state will outsource all error handling logic to the caller of this library rather than making assumptions.

# Examples
Rotate when a log file exceeds a certain filesize

```
use std::{io::Write, thread::sleep, time::Duration};
use turnstiles::{RotatingFile, RotationCondition, PruneCondition};
use tempdir::TempDir; // Subcrate provided for testing
let dir = TempDir::new();

let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
let data: Vec<u8> = vec![0; 500_000];
// The `false` here is to do with require_newline and is only needed for async loggers
let mut file = RotatingFile::new(path, RotationCondition::SizeMB(1), PruneCondition::None, false)
                .unwrap();

// Write 500k to file creating test.log
file.write(&data).unwrap();
assert!(file.index() == 0);

// Write another 500kb so test.log is 1mb
file.write_all(&data).unwrap();
assert!(file.index() == 0);

// The check for rotation is done _before_ writing, so we don't rotate, and then write 500kb
// so this file is ~1.5mb now, still the same file
file.write_all(&data).unwrap();
assert!(file.index() == 0);


// Now we check if we need to rotate before writing, and it's 1.5mb > the rotation option so
// we make a new file and  write to that
file.write_all(&data).unwrap();
assert!(file.index() == 1);

// Now have test_ACTIVE.log and test.log.1
```

Rotate when a log file is too old (based on filesystem metadata timestamps)

```
use std::{io::Write, thread::sleep, time::Duration};
use turnstiles::{RotatingFile, RotationCondition, PruneCondition};
use tempdir::TempDir; // Subcrate provided for testing
let dir = TempDir::new();
let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");

let max_log_age = Duration::from_millis(100);
let data: Vec<u8> = vec![0; 1_000_000];
let mut file =
    RotatingFile::new(path, RotationCondition::Duration(max_log_age), PruneCondition::None, false)
        .unwrap();

assert!(file.index() == 0);
file.write_all(&data).unwrap();
assert!(file.index() == 0);
file.write_all(&data).unwrap();
assert!(file.index() == 0);
sleep(Duration::from_millis(200));

// Rotation only happens when we call .write() so index remains unchanged after this duration
// even though it exceeds that given in the RotationCondition
assert!(file.index() == 0);
// Bit touch and go but assuming two writes of 1mb bytes doesn't take 100ms!
file.write_all(&data).unwrap();
assert!(file.index() == 1);
file.write_all(&data).unwrap();
assert!(file.index() == 1);

// Will now have test_ACTIVE.log and test.log.1
```


Prune old logs to avoid filling up the disk

```
use std::{io::Write, path::Path};
use tempdir::TempDir;
use turnstiles::{PruneCondition, RotatingFile, RotationCondition}; // Subcrate provided for testing
let dir = TempDir::new();
let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
let data: Vec<u8> = vec![0; 990_000];
let mut file = RotatingFile::new(
    path,
    RotationCondition::SizeMB(1),
    PruneCondition::MaxFiles(3),
    false,
)
.unwrap();

// Generate > 3
// (this will generate 10 files because we're only writing 990kb and rotating on 1mb)
for _ in 0..20 {
    file.write_all(&data).unwrap();
}

// Should now only have the active file and two files with the highest index
// (which will be 8 and 9 in this case)
for i in 1..4 {
    let path = &format!("{}/test.log.{}", &dir.path, i);
    let file = Path::new(path);
    if i < 8 {
        assert!(!file.is_file());
    } else {
        assert!(file.is_file());
    }
}
```

*/
use anyhow::{bail, Result};
use std::time::SystemTime;
use std::{
    cmp,
    fs::{self, remove_file, File, OpenOptions},
    io,
    time::Duration,
};
mod utils;
use regex::Regex;
use utils::{filename_to_details, safe_unwrap_osstr};

// TODO: template this maybe? Or just make it u128 and fugheddaboutit?
type FileIndexInt = u32;
const BYTES_TO_MB: u64 = 1_048_576;

// Changed from prefix to suffix here to make wildcarding less of a faff.
fn active_filename(root_filename: &str) -> String {
    format!("{}{}", root_filename, ".ACTIVE")
}
#[derive(Debug)]
/// Struct masquerades as a file handle and is written to by whatever you like
pub struct RotatingFile {
    filename_root: String,
    active_file_path: String,
    active_file_name: String,
    rotation_method: RotationCondition,
    prune_method: PruneCondition,
    current_file: File,
    index: FileIndexInt,
    require_newline: bool, // Should be type to avoid runtime cost?
    parent: String,
    file_regex: Regex,
}

impl RotatingFile {
    /// Create a new RotatingFile given a desired filename and rotation option. The filename represents the stem or root of the files
    /// to be generated.
    pub fn new(
        path_str: &str,
        rotation_method: RotationCondition,
        prune_method: PruneCondition,
        require_newline: bool,
    ) -> Result<Self> {
        Self::check_options(&rotation_method, &prune_method)?;
        // TODO: throw error if path_str (rootname) ends in digit as this will break the numbering stuff
        let (path_filename, parent) = filename_to_details(path_str)?;
        let file_regex = Regex::new(&format!(r"^{}.[0-9]+$", path_filename)).map_err(|e| {
            // Thanks I hate it.
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Regex failed with error {}", e),
            )
        })?;

        let active_file_name = active_filename(&path_filename);
        let active_file_path = format!("{}/{}", parent, &active_file_name);
        let current_index = Self::detect_latest_file_index(&file_regex, &parent)?;
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(active_file_path.clone())?;
        Ok(Self {
            rotation_method,
            prune_method,
            current_file: file,
            index: current_index,
            filename_root: path_filename,
            require_newline,
            active_file_path,
            active_file_name,
            parent,
            file_regex,
        })
    }

    /// Check we're given valid options on startup
    fn check_options(
        rotation_method: &RotationCondition,
        prune_method: &PruneCondition,
    ) -> Result<()> {
        if let RotationCondition::SizeMB(0) = rotation_method {
            bail!("Invalid option: RotationCondition::SizeMB(0)");
        }
        if let PruneCondition::MaxFiles(0) = prune_method {
            bail!("Invalid option: PruneCondition::MaxFiles(0)");
        }
        Ok(())
    }

    /// Given a filename stem and folder path, list all files which are the `filename.<index>` (where filename includes the extension).
    /// Uses regex to match on `r"^<filename>.[0-9]+$"`
    fn list_rotated_log_files(
        file_regex: &Regex,
        folder_path: &str,
    ) -> Result<Vec<String>, std::io::Error> {
        let files = fs::read_dir(&folder_path)?;

        let mut log_files = vec![];
        for f in files {
            let filename_str = safe_unwrap_osstr(&f?.file_name())?;
            if file_regex.is_match(&filename_str) {
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
    fn detect_latest_file_index(file_regex: &Regex, folder_path: &str) -> Result<FileIndexInt> {
        let log_files = Self::list_rotated_log_files(file_regex, folder_path)?;
        let mut max_index = 0;
        for filename_string in log_files {
            let i = Self::rotated_file_index(&filename_string)?;
            max_index = cmp::max(i, max_index);
        }

        Ok(max_index)
    }

    fn rotated_file_index(filename: &str) -> Result<FileIndexInt> {
        let file_index = match filename.split('.').last() {
            None => bail!("Found log file ending in '.', can't process index."),
            Some(s) => s,
        };
        Ok(file_index.parse::<FileIndexInt>()?)
    }

    /// Perform file rotation
    fn rotate_current_file(&mut self) -> Result<(), std::io::Error> {
        // TODO: think about if we want to be more careful here, i.e. append to a random file which may already exist and be a totally different format?
        // Could throw an exception, or print a warning and skip that file index. Who logs the loggers...

        // TODO: fix naughtyness of renaming file while handle still open, should prob be an option which we take and shutdown
        // let mut result = || -> Result<(), std::io::Error> {
        // fsync before rotation
        self.current_file.sync_all()?;

        let new_file = &format!("{}/{}.{}", self.parent, self.filename_root, self.index + 1);
        fs::rename(&self.active_file_path, new_file)?;
        self.current_file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&self.active_file_path)?;
        self.index += 1; // Only do this once the above results have passed.

        Ok(())
        // };
        // if let Err(e) = result() {
        //     println!(
        //         "WARN: turnstiles caught error in rotate_current_file(), will attempt to continue writing to current file.\nErr: {}",
        //         e
        //     );
        // };
    }

    /// Given the RotationCondition chosen when the struct was created, check if a rotation is in order
    /// NOTE: this currently does no check to see if the file rotation option has changed for a given set of logs, but this will never result in dataloss
    /// just maybe some confusingly-sized logs
    fn rotation_required(&mut self) -> bool {
        // NOTE: we used to fsync before getting metadata for this but was removed as veeery slow, seems reasonable?
        // Now we juts explicitly fsync before rotation
        let result = || -> Result<bool, std::io::Error> {
            let rotate = match self.rotation_method {
                RotationCondition::None => false,
                RotationCondition::SizeMB(size) => {
                    self.current_file.metadata()?.len() > size * BYTES_TO_MB
                }
                // RotationCondition::SizeLines(len) => false,
                RotationCondition::Duration(duration) => {
                    match self.current_file.metadata()?.created()?.elapsed() {
                        Ok(elapsed) => elapsed > duration,
                        Err(e) => {
                            println!("WARN: failed to determine time since log file created - not rotating, got error {}.", e);
                            false
                        }
                    }
                }
            };
            Ok(rotate)
        };
        match result() {
            Ok(r) => r,
            Err(e) => {
                println!("WARN: turnstiles caught error in rotation_required(), defaulting to not rotating.\nErr: {}",e);
                false
            }
        }
    }

    fn prune_logs(&mut self) {
        // TODO: tidy this horribleness and seek out corner cases
        let result = || -> Result<(), std::io::Error> {
            let log_file_list = Self::list_rotated_log_files(&self.file_regex, &self.parent)?;
            match self.prune_method {
                PruneCondition::None => {}
                PruneCondition::MaxAge(d) => {
                    let modified_cutoff = SystemTime::now() - d;
                    for filename in log_file_list {
                        let path = format!("{}/{}", self.parent, filename);
                        let metadata = fs::metadata(&path)?;
                        if metadata.modified()? < modified_cutoff {
                            remove_file(path)?;
                        }
                    }
                }
                PruneCondition::MaxFiles(n) => {
                    let index_u = self.index as usize;
                    // This works but I hate it; juggling usize stuff
                    // TODO: invert search to make more performant
                    if log_file_list.len() > n - 1 && index_u + 2 > 1 + n {
                        for i in 1..index_u - n + 2 {
                            let file_to_delete = &format!("{}.{}", self.filename_root, i);
                            if log_file_list.contains(file_to_delete) {
                                remove_file(format!("{}/{}", self.parent, file_to_delete))?;
                            }
                        }
                    }
                }
            };
            Ok(())
        }();
        match result {
            Ok(r) => r,
            Err(e) => {
                println!("WARN: turnstiles caught error in prune_logs().\nErr: {}", e);
            }
        }
    }

    pub fn current_file(&self) -> &File {
        &self.current_file
    }

    pub fn current_file_path_str(&self) -> &str {
        &self.active_file_path
    }

    pub fn current_file_name_str(&self) -> &str {
        &self.active_file_name
    }
}

impl io::Write for RotatingFile {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, std::io::Error> {
        // Note: only the rotate and write methods here can return errors, the errors in prune and rotation_required are suppressed to try ensure max uptime of logging
        // If rotation_required() fails it will return false so the current file will continue to be written to (or at least, attempted)

        if !self.require_newline {
            if self.rotation_required() {
                self.rotate_current_file()?;
                self.prune_logs();
            }
        } else if let Some(last_char) = bytes.last() {
            // Note this will prevent writing just a newline and so could break some stuff
            // TODO: be smarter here in future, not sure how best to distinguish between genuine newline write and broken up log from slog async
            if *last_char == b'\n' && self.rotation_required() {
                self.rotate_current_file()?;
                if bytes.len() != 1 {
                    self.current_file.write_all(bytes)?;
                }
                self.prune_logs();
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
pub enum RotationCondition {
    None,
    SizeMB(u64),
    Duration(Duration),
    // SizeLines(u64),
}
/// Enum for possible file prune options.
#[derive(Debug)]
pub enum PruneCondition {
    None,
    MaxFiles(usize),
    MaxAge(Duration),
}
