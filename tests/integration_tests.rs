use std::{io::Write, thread::sleep, time::Duration};

use tempdir::TempDir;
use turnstiles::{RotatingFile, RotationOption};

// Duplicated by doctests but i think that's okay? These have fn names, easier to interpret if failing...
#[test]
fn test_file_size() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
    let data: Vec<u8> = vec![0; 1_000_000];
    let mut file = RotatingFile::new(path, RotationOption::SizeMB(1)).unwrap();
    assert!(file.index() == 0);
    file.write_all(&data).unwrap(); //write 1mb to file
    file.write_all(&data).unwrap(); //write 1mb to file
    assert!(file.index() == 1);
    file.write_all(&data).unwrap(); //write 1mb to file
    assert!(file.index() == 2); // should have 3 files now
}

#[test]
fn test_file_duration() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");

    let data: Vec<u8> = vec!["a"; 100_000].join("").as_bytes().to_vec();
    let mut file =
        RotatingFile::new(path, RotationOption::Duration(Duration::from_millis(100))).unwrap();
    dbg!(&file.index());
    file.write_all(&data).unwrap();
    file.write_all(&data).unwrap();
    sleep(Duration::from_millis(200));

    // Bit touch and go but assuming two writes of 100k bytes doesn't take 100ms!
    file.write_all(&data).unwrap();
    file.write_all(&data).unwrap();
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
    file.write_all(&data).unwrap();
    file.write_all(&data).unwrap();
    sleep(Duration::from_millis(200));
    file.write_all(&data).unwrap();
    assert!(file.index() == 1);
}

#[test]
#[should_panic]
/// Try to write to non-existent directory, should fail
fn test_no_dir_simple() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
    drop(dir);

    let data: Vec<u8> = vec!["a"; 100_000].join("").as_bytes().to_vec();
    let mut file =
        RotatingFile::new(path, RotationOption::Duration(Duration::from_millis(100))).unwrap();
    file.write_all(&data).unwrap();
}

#[test]
#[should_panic]
/// Delete directory after initial write, should fail to write again
fn test_no_dir_intermediate() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");

    let data: Vec<u8> = vec!["a"; 100_000].join("").as_bytes().to_vec();
    let mut file =
        RotatingFile::new(path, RotationOption::Duration(Duration::from_millis(100))).unwrap();
    file.write_all(&data).unwrap();
    sleep(Duration::from_millis(200));
    drop(dir);
    file.write_all(&data).unwrap();
}

// Duplicated by doctests but i think that's okay? These have fn names, easier to interpret if failing...
#[test]
fn test_data_integrity() {
    use std::fs;
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");

    let mut file = RotatingFile::new(path, RotationOption::SizeMB(1)).unwrap();
    // assert!(file.index() == 0);
    file.write_all(&vec![0; 1_000_001]).unwrap(); //write 1mb to file
    file.write_all(&vec![1; 1_000_001]).unwrap(); //write 1mb to file
                                                  // assert!(file.index() == 1);
    file.write_all(&vec![2; 1_000_001]).unwrap(); //write 1mb to file
                                                  // assert!(file.index() == 2); // should have 3 files now

    // Original data
    let data = fs::read(path).unwrap();
    assert_eq!(data, vec![0; 1_000_001]);
    // Rotated portions
    for i in 0..2 {
        let data = fs::read(format!("{}.{}", path, i + 1)).unwrap();
        assert_eq!(data, vec![i + 1; 1_000_001]);
    }
}
