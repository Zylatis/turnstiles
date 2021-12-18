# Turnstyles

A WIP library which wraps the `io::Write` trait to enable file rotation i.e. for logs.

Implemented/planned rotation conditions:
- [x] None (never rotate)
- [x] SizeMB (file size)
- [x] Duration (time since last modified)
- [ ] SizeLines (number of lines in file) 

# Examples
Rotate when a log file exceeds a certain filesize

```rust
let some_bytes: Vec<u8> = vec![0; 1_000_000];
let mut log_file =
    RotatingFile::new("logs/super_important_service.log", RotationOption::SizeMB(500))
    .expect("failed to create RotatingFile");
file.write(&some_bytes).expect("Failed to write bytes to file");
```

Rotate when a log file is too old (based on filesystem metadata timestamps)

```rust
let max_log_age = Duration::from_secs(3600);
let some_bytes: Vec<u8> = vec![0; 10_000_000];
let mut log_file =
    RotatingFile::new("logs/super_important_service.log", RotationOption::Duration(max_log_age))
    .expect("failed to create RotatingFile");
file.write(&some_bytes).expect("Failed to write bytes to file");
```