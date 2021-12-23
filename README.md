# Turnstiles

<a href="https://github.com/Zylatis/turnstiles/actions/workflows/rust.yml"><img src="https://github.com/Zylatis/turnstiles/actions/workflows/rust.yml/badge.svg" /></a>
<a href="https://crates.io/crates/turnstiles"><img src="https://raster.shields.io/crates/v/turnstiles.png" /></a>

A WIP library which wraps the `io::Write` trait to enable file rotation i.e. for logs. The goal is to enable file rotation at the file handle level and do so with as few dependencies as possible.

Implemented/planned rotation conditions:
- [x] None (never rotate)
- [x] SizeMB (file size)
- [x] Duration (time since last modified)
- [ ] SizeLines (number of lines in file) 

## Warning:
This is currently in active development and may change/break often. Every effort will be taken to ensure that breaking changes that occur are reflected in a change of at least the minor version of the package, both in terms of the API and the generation of log files. Versions prior to 0.2.0 were so riddled with bugs I'm amazed I managed to put my pants on on those days I was writing it.

## Note:
Currently this library only supports rotation by creating new files when a rotation is required, rather than creating a new file _and_ renaming existing files.
For example if `my_file.log` is given then when the first rotation occurs a file with the name `my_file.log.1` will be created and written to. This means the latest file has the highest index, not the original filename. This is done to minimize surface area with the filesystem, but it is part of future work to potentially include the case where `my_file.log` is always the latest. 

# Examples
Rotate when a log file exceeds a certain filesize

```rust
let data: Vec<u8> = vec![0; 500_000];
let mut file = RotatingFile::new("test.log", RotationOption::SizeMB(1)).unwrap();
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

```rust
let max_log_age = Duration::from_millis(100);
let data: Vec<u8> = vec![0; 1_000_000];
let mut file =
    RotatingFile::new(path, RotationOption::Duration(max_log_age)).unwrap();

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

## Future work
- Be more careful around edgecases for example rotating on 1mb files and writing exactly 1mb to disk
- More direct integration with dedicated logging libraries, i.e. `impl log::Log`.
- Investigate integration with things like [`atomicwrites`](https://crates.io/crates/atomicwrites)
- More flexible rotation options
## Why `turnstiles`?
It's a metal thing that rotates, and also the name of the Billy Joel album which has [`Summer, Highland Falls`](https://youtu.be/WsNhuJypNjM) on it, one of my favourite songs.