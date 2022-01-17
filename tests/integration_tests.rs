use std::{collections::HashSet, fs, io::Write, thread::sleep, time::Duration};
use tempdir::TempDir;
use turnstiles::{PruneMethod, RotatingFile, RotationMethod};

// Duplicated by doctests but i think that's okay? These have fn names, easier to interpret if failing...
#[test]
fn test_file_size() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
    let data: Vec<u8> = vec![0; 500_000];
    let mut file =
        RotatingFile::new(path, RotationMethod::SizeMB(1), PruneMethod::None, false).unwrap();

    file.write_all(&data).unwrap(); // write 500k to file

    assert!(file.index() == 0);
    file.write_all(&data).unwrap();
    assert!(file.index() == 0);

    file.write_all(&data).unwrap();
    assert!(file.index() == 0);
    file.write_all(&data).unwrap();
    assert!(file.index() == 1);
    assert_correct_files(&dir.path, vec!["ACTIVE_test.log", "test.log.1"]);
}

#[test]
fn test_file_size_no_rotate() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
    let data: Vec<u8> = vec![0; 1_000];
    let mut file =
        RotatingFile::new(path, RotationMethod::SizeMB(1), PruneMethod::None, false).unwrap();
    assert!(file.index() == 0);
    file.write_all(&data).unwrap();
    assert!(file.index() == 0);
    file.write_all(&data).unwrap();
    assert!(file.index() == 0);
    file.write_all(&data).unwrap();
    assert!(file.index() == 0);
    assert_correct_files(&dir.path, vec!["ACTIVE_test.log"]);
}

#[test]
fn test_file_duration() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");

    let data: Vec<u8> = vec!["a"; 100_000].join("").as_bytes().to_vec();
    let mut file = RotatingFile::new(
        path,
        RotationMethod::Duration(Duration::from_millis(100)),
        PruneMethod::None,
        false,
    )
    .unwrap();

    assert!(file.index() == 0);
    file.write_all(&data).unwrap();
    assert!(file.index() == 0);
    file.write_all(&data).unwrap();
    assert!(file.index() == 0);
    sleep(Duration::from_millis(200));

    // Rotation only happens when we call .write() so index remains unchanged after this duration even though it exceeds
    // that given in the RotationOption
    assert!(file.index() == 0);
    // Bit touch and go but assuming two writes of 100k bytes doesn't take 100ms!
    file.write_all(&data).unwrap();
    assert!(file.index() == 1);
    file.write_all(&data).unwrap();
    assert!(file.index() == 1);

    sleep(Duration::from_millis(200));
    assert!(file.index() == 1);
    // Bit touch and go but assuming two writes of 100k bytes doesn't take 100ms!
    file.write_all(&data).unwrap();
    assert!(file.index() == 2);
    file.write_all(&data).unwrap();
    assert!(file.index() == 2);

    assert_correct_files(
        &dir.path,
        vec!["ACTIVE_test.log", "test.log.1", "test.log.2"],
    );
}

#[test]
#[should_panic]
fn test_file_duration_delay_fail() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");

    let data: Vec<u8> = vec!["a"; 100_000].join("").as_bytes().to_vec();
    let mut file = RotatingFile::new(
        path,
        RotationMethod::Duration(Duration::from_millis(100)),
        PruneMethod::None,
        false,
    )
    .unwrap();
    sleep(Duration::from_millis(200)); // the constructor makes the file and so the timer starts from then, this should cause it to fail
    assert!(file.index() == 0);
    file.write_all(&data).unwrap();
    assert!(file.index() == 1);
    file.write_all(&data).unwrap();
    sleep(Duration::from_millis(200));
    file.write_all(&data).unwrap();
    assert!(file.index() == 1); // Should fail
}

#[test]
#[should_panic]
/// Try to write to non-existent directory, should fail
fn test_no_dir_simple() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
    drop(dir);

    let data: Vec<u8> = vec!["a"; 100_000].join("").as_bytes().to_vec();
    let mut file = RotatingFile::new(
        path,
        RotationMethod::Duration(Duration::from_millis(100)),
        PruneMethod::None,
        false,
    )
    .unwrap();
    file.write_all(&data).unwrap();
}

#[test]
#[should_panic]
/// Delete directory after initial write, should fail to write again
fn test_no_dir_intermediate() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");

    let data: Vec<u8> = vec!["a"; 100_000].join("").as_bytes().to_vec();
    let mut file = RotatingFile::new(
        path,
        RotationMethod::Duration(Duration::from_millis(100)),
        PruneMethod::None,
        false,
    )
    .unwrap();
    file.write_all(&data).unwrap();
    sleep(Duration::from_millis(200));
    drop(dir);
    file.write_all(&data).unwrap();
}

#[test]
fn test_data_integrity() {
    use std::fs;
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");

    let mut file =
        RotatingFile::new(path, RotationMethod::SizeMB(1), PruneMethod::None, false).unwrap();
    assert!(file.index() == 0);

    file.write_all(&vec![0; 600_000]).unwrap();
    assert!(file.index() == 0);

    file.write_all(&vec![0; 600_000]).unwrap();
    assert!(file.index() == 0);

    file.write_all(&vec![1; 600_000]).unwrap();
    assert!(file.index() == 1);

    // Original data
    let data = fs::read(format!("{}.1", path)).unwrap();
    assert_eq!(data, vec![0; 1_200_000]);

    // Rotated data
    let data = fs::read(file.current_file_path_str()).unwrap();
    assert_eq!(data, vec![1; 600_000]);
    assert_correct_files(&dir.path, vec!["ACTIVE_test.log", "test.log.1"]);
}

#[test]
fn test_restart() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
    let data: Vec<u8> = vec![0; 600_000];
    let mut file =
        RotatingFile::new(path, RotationMethod::SizeMB(1), PruneMethod::None, false).unwrap();

    file.write_all(&data).unwrap();

    assert!(file.index() == 0);
    file.write_all(&data).unwrap();
    assert!(file.index() == 0);

    file.write_all(&data).unwrap();
    assert!(file.index() == 1);
    file.write_all(&data).unwrap();
    assert!(file.index() == 1);
    assert_correct_files(&dir.path, vec!["ACTIVE_test.log", "test.log.1"]);
    // Start again and make sure we pickup where we left off
    drop(file);
    let mut file =
        RotatingFile::new(path, RotationMethod::SizeMB(1), PruneMethod::None, false).unwrap();

    file.write_all(&data).unwrap();

    assert!(file.index() == 2);
    file.write_all(&data).unwrap();
    assert!(file.index() == 2);

    file.write_all(&data).unwrap();
    assert!(file.index() == 3);
    file.write_all(&data).unwrap();
    assert!(file.index() == 3);

    assert_correct_files(
        &dir.path,
        vec!["ACTIVE_test.log", "test.log.1", "test.log.2", "test.log.3"],
    );
}

#[test]
fn test_slog_json_async() {
    // Check that passing the 'expect_newline' works when we're writing with slog json which writes asynchronously

    use slog::{info, o, Drain, Logger};
    use std::io::BufRead;
    use std::sync::Mutex;
    use std::time::SystemTime;
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");

    let log_file = RotatingFile::new(
        path,
        RotationMethod::Duration(Duration::from_millis(100)), // any shorter than this and we run the risk of OS i/o stuff getting in the way :/
        PruneMethod::None,
        true,
    )
    .unwrap();

    let log_drain = slog_json::Json::default(log_file);
    let logger = Logger::root(Mutex::new(log_drain).fuse(), o!());

    let start = SystemTime::now();
    while start.elapsed().unwrap() < Duration::from_millis(210) {
        info!(
            logger,
            "abcd--------------------------------------------------------------"
        );
    }
    // TODO: tidy
    let expected_files = vec!["ACTIVE_test.log", "test.log.1", "test.log.2"];
    assert_correct_files(&dir.path, expected_files.clone());

    for filename in expected_files {
        let file = std::fs::File::open(format!("{}/{}", &dir.path, filename)).unwrap();
        let data = std::io::BufReader::new(file).lines();
        for line in data {
            assert!(line.unwrap().ends_with('}'));
        }
    }
}

#[test]
#[should_panic]
fn test_slog_json_async_binary_fail() {
    // Check that passing the 'expect_newline' works when we're writing with slog json which writes asynchronously

    use slog::{info, o, Drain, Logger};
    use std::io::BufRead;
    use std::sync::Mutex;
    use std::time::SystemTime;
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
    // TODO: refactor common bits of these two tests
    let log_file = RotatingFile::new(
        path,
        RotationMethod::Duration(Duration::from_millis(100)), // any shorter than this and we run the risk of OS i/o stuff getting in the way :/
        PruneMethod::None,
        false,
    )
    .unwrap();

    let log_drain = slog_json::Json::default(log_file);
    let logger = Logger::root(Mutex::new(log_drain).fuse(), o!());

    let start = SystemTime::now();
    while start.elapsed().unwrap() < Duration::from_millis(210) {
        info!(
            logger,
            "abcd--------------------------------------------------------------"
        );
    }
    // TODO: tidy
    let expected_files = vec!["ACTIVE_test.log", "test.log.1", "test.log.2"];
    assert_correct_files(&dir.path, expected_files.clone());

    for filename in expected_files {
        let file = std::fs::File::open(format!("{}/{}", &dir.path, filename)).unwrap();
        let data = std::io::BufReader::new(file).lines();
        for line in data {
            assert!(line.unwrap().ends_with('}'));
        }
    }
}

#[test]
fn test_slog_json_async_data_integrity() {
    // Ensure if we write a bunch of stuff to disk we get all the data back, no gaps etc
    use serde::{Deserialize, Serialize};
    // Check that passing the 'expect_newline' works when we're writing with slog json which writes asynchronously
    #[derive(Serialize, Deserialize)]
    struct JsonLog {
        msg: String,
        level: String,
        ts: String,
    }

    use slog::{info, o, Drain, Logger};
    use std::io::BufRead;
    use std::sync::Mutex;

    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");

    let log_file = RotatingFile::new(
        path,
        RotationMethod::Duration(Duration::from_millis(5)), // any shorter than this and we run the risk of OS i/o stuff getting in the way :/
        PruneMethod::None,
        true,
    )
    .unwrap();

    let log_drain = slog_json::Json::default(log_file);
    let logger = Logger::root(Mutex::new(log_drain).fuse(), o!());

    let mut data = HashSet::new();
    let n = 100;
    for i in 0..1000 {
        let row: Vec<u32> = (n * i..n + n * i).collect();
        info!(logger, "{:?}", &row);
        data.insert(format!("{:?}", &row));
    }
    // read the data back in, get the msg component, and confirm all data written
    let mut json_data = HashSet::new();
    for filename in get_dir_files_hashset(&dir.path) {
        let file = std::fs::File::open(format!("{}/{}", &dir.path, filename)).unwrap();
        let data = std::io::BufReader::new(file).lines();
        for line in data {
            let row_data: JsonLog = serde_json::from_str(&line.unwrap()).unwrap();
            json_data.insert(row_data.msg);
        }
    }
    // XOR the two sets (almost certainly a better way - retain mutates tho?)
    assert!(json_data.iter().filter(|x| !data.contains(*x)).count() == 0);
    assert!(data.iter().filter(|x| !json_data.contains(*x)).count() == 0);
}

#[test]
fn test_file_number_prune() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
    let data: Vec<u8> = vec![0; 990_000];
    let mut file = RotatingFile::new(
        path,
        RotationMethod::SizeMB(1),
        PruneMethod::MaxFiles(3),
        false,
    )
    .unwrap();

    for _ in 0..20 {
        file.write_all(&data).unwrap();
    }

    assert_correct_files(
        &dir.path,
        vec!["ACTIVE_test.log", "test.log.8", "test.log.9"],
    );
}

#[test]
fn test_file_age_prune() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
    let data: Vec<u8> = vec![0; 990_000];
    let mut file = RotatingFile::new(
        path,
        RotationMethod::SizeMB(1),
        PruneMethod::MaxAge(Duration::from_millis(1000)),
        false,
    )
    .unwrap();

    for _ in 0..20 {
        file.write_all(&data).unwrap();
    }
    sleep(Duration::from_millis(1000));
    file.write_all(&data).unwrap();
    file.write_all(&data).unwrap();
    assert_correct_files(&dir.path, vec!["ACTIVE_test.log"]);
}

#[test]
fn test_invalid_options() {
    let dir = TempDir::new();
    let path = &vec![dir.path.clone(), "test.log".to_string()].join("/");
    assert!(RotatingFile::new(
        path,
        RotationMethod::SizeMB(1),
        PruneMethod::MaxAge(Duration::from_millis(1000)),
        false,
    )
    .is_ok());

    assert!(RotatingFile::new(
        path,
        RotationMethod::SizeMB(0), // not valid
        PruneMethod::MaxAge(Duration::from_millis(1000)),
        false,
    )
    .is_err());

    assert!(RotatingFile::new(
        path,
        RotationMethod::SizeMB(1),
        PruneMethod::MaxFiles(0), // not valid
        false,
    )
    .is_err());
}

// Some helpers
fn get_dir_files_hashset(dir: &str) -> HashSet<String> {
    let mut files = HashSet::new();
    for file in fs::read_dir(dir).unwrap() {
        let filename = file.unwrap().file_name().to_str().unwrap().to_string();
        files.insert(filename);
    }
    files
}

fn assert_correct_files(dir: &str, log_filenames: Vec<&str>) {
    // TODO: change to ref of vec, prob doesn't need ownership
    // TODO: fix this complete shitshow
    let log_files: HashSet<String> = get_dir_files_hashset(dir);
    let log_files_str: HashSet<&str> = log_files.iter().map(AsRef::as_ref).collect();
    let expected: HashSet<&str> = log_filenames.into_iter().collect();

    assert_eq!(log_files_str, expected);
}
