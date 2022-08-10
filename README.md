# Turnstiles

<a href="https://github.com/Zylatis/turnstiles/actions/workflows/rust.yml"><img src="https://github.com/Zylatis/turnstiles/actions/workflows/rust.yml/badge.svg" /></a>
<a href="https://crates.io/crates/turnstiles"><img src="https://raster.shields.io/crates/v/turnstiles.png" /></a>

A WIP library which wraps the `io::Write` trait to enable file rotation i.e. for logs. The goal is to enable file rotation at the file handle level and do so with as few dependencies as possible.

Implemented/planned rotation conditions:
- [x] None (never rotate)
- [x] SizeMB (file size)
- [x] Duration (time since last modified)
- [ ] SizeLines (number of lines in file) 

There are also three options to prune old logs:
- [x] None
- [x] MaxFiles
- [x] MaxAge

## Warning:
This is currently in active development and may change/break often. Every effort will be taken to ensure that breaking changes that occur are reflected in a change of at least the minor version of the package, both in terms of the API and the generation of log files. Versions prior to 0.2.0 were so riddled with bugs I'm amazed I managed to put my pants on on those days I was writing it.

## Note:
Rotation works by keeping track of the 'active' file, the one currently being written to, which upon rotation is renamed to include the next log file index. For example when there is only one log file it will be `test.log.ACTIVE`, which when rotated will get renamed to `test.log.1` and the `test.log.ACTIVE` will represent a new file being written to. Originally no file renaming was done to keep the surface area with the filesystem as small as possible, however this has a few disadvantages and this active-file-approach (courtesy of [flex-logger](https://docs.rs/flexi_logger/latest/flexi_logger/)) was seen as a good compromise. The downside is that the file extension now superifically looks different, but it does mean all logs can be found by simply searching for `test.log*`. 

Log suffix numbers will increase with age, so the first of the rotated logs will be `test.log.1`, second will be `test.log.2` etc until `N-1` after which it will be `test.log.ACTIVE`, the current one.  

## Warning:
Little to no protection is given defend against the file indices being modified during the operation of whatever code is using this logger: when `write` is called it does not currently refresh the internal index which tracks the suffix integer, this is only done when the logger is created. Changing this is noted as future work.

## Note:
Not all internal errors are handled the same way. For example, if during the process of checking if rotation is required an error occurs, the default is to print a warning to stdout and _not_ rotate. In contrast to this, if an error occurs during the actual rotation procedure, this error is bubbled up through error handling eventually returning as a `std::io::Error` to the caller. However probable future state will outsource all error handling logic to the caller of this library rather than making assumptions.

# Examples
See docs [here](https://docs.rs/turnstiles/latest/turnstiles/) for example usage. 

## Future work
- Refresh internal index when rotation requested, not just at logger creation
- Be more careful around edgecases for example rotating on 1mb files and writing exactly 1mb to disk
- More direct integration with dedicated logging libraries, i.e. `impl log::Log`.
- Investigate integration with things like [`atomicwrites`](https://crates.io/crates/atomicwrites)
- More flexible rotation options
## Why `turnstiles`?
It's a metal thing that rotates, and also the name of the Billy Joel album which has [`Summer, Highland Falls`](https://youtu.be/WsNhuJypNjM) on it, one of my favourite songs.