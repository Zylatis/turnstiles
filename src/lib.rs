use std::fs;
use std::path::PathBuf;
use std::{
    fs::{File, Metadata, OpenOptions},
    io,
    time::Duration,
};
// Example:
// file.log
// file.log.1
// file.log.2
// in increasing order of oldness
// So when we boot we have to query the file system to see where
#[derive(Debug)]
pub struct RotatingFile {
    path: PathBuf,
    rotation: RotationOption,
    current_file: File,
}

impl RotatingFile {
    pub fn new(path: &str, rotation: RotationOption) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            path: PathBuf::from(path),
            rotation,
            current_file: file,
        })
    }

    pub fn rotate_existing_files(&self) {
        let dir = match self.path.parent() {
            None => "/",
            Some(s) => match s.to_str().unwrap() {
                "" => ".",
                x => x,
            },
        };

        let files = fs::read_dir(&dir).unwrap().map(|x| x.unwrap());
        let mut log_files = vec![];
        let prefix = self.path.file_name().unwrap().to_str().unwrap();
        for f in files {
            if f.file_name().to_str().unwrap().contains(prefix) {
                log_files.push(f);
            }
        }
        dbg!(log_files);
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
        if self.rotate() {}
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
    use crate::{RotatingFile, RotationOption};

    #[test]
    fn test() {
        let file = RotatingFile::new("logs/test.log", RotationOption::SizeMB(1)).unwrap();
        file.rotate_existing_files();
    }
}
