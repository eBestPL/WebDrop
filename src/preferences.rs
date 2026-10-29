//! Persistent user preferences for Windows and Linux.
use std::{fs, io::Write, path::PathBuf};

fn path() -> std::io::Result<PathBuf> {
    directories::ProjectDirs::from("pl", "ebest", "WebDrop")
        .map(|dirs| dirs.config_dir().join("preferences.json"))
        .ok_or_else(|| std::io::Error::other("Cannot locate user configuration directory"))
}

pub fn load<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    let data: serde_json::Value = serde_json::from_slice(&fs::read(path().ok()?).ok()?).ok()?;
    serde_json::from_value(data.get(key)?.clone()).ok()
}

pub fn save<T: serde::Serialize>(key: &str, value: &T) -> std::io::Result<()> {
    let path = path()?;
    let mut data = match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice::<serde_json::Map<String, serde_json::Value>>(&bytes)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => serde_json::Map::new(),
        Err(error) => return Err(error),
    };
    data.insert(key.into(), serde_json::to_value(value)?);
    let directory = path
        .parent()
        .ok_or_else(|| std::io::Error::other("Invalid preferences path"))?;
    fs::create_dir_all(directory)?;
    let mut temp = tempfile::NamedTempFile::new_in(directory)?;
    temp.write_all(&serde_json::to_vec_pretty(&data)?)?;
    temp.as_file().sync_all()?;
    temp.persist(&path).map_err(|error| error.error)?;
    Ok(())
}
