use anyhow::{bail, Result};
use std::{ffi::OsStr, path::PathBuf};
pub fn filename_to_details(path_str: &str) -> Result<(String, String)> {
    let pathbuf = PathBuf::from(path_str);

    let filename: String = match pathbuf.file_name() {
        None => bail!("Could not get filename"),
        Some(f_osstr) => safe_unwrap_osstr(f_osstr)?,
    };

    let parent = match pathbuf.parent() {
        None => "/",
        Some(s) => match s.to_str() {
            None => bail!("Could not convert OsStr to &str"),
            Some("") => ".",
            Some(s) => s,
        },
    }
    .to_string();
    Ok((filename, parent))
}

pub fn safe_unwrap_osstr(s: &OsStr) -> Result<String> {
    let string = match s.to_str() {
        None => bail!("Could not convert OsStr to &str"),
        Some(f_str) => f_str.to_string(),
    };
    Ok(string)
}
