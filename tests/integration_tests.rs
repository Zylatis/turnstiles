use std::{io::Write, thread::sleep, time::Duration};

use turnstyles::{RotatingFile, RotationOption};
mod common;
use common::TempDir;
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
