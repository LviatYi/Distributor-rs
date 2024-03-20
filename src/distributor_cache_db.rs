use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::distributor::DistributorResult;

#[derive(Debug)]
pub enum QueryMetaError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for QueryMetaError {
    fn from(e: std::io::Error) -> Self {
        QueryMetaError::IoError(e)
    }
}

pub type QueryMetaResult<T> = Result<T, QueryMetaError>;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct FileDistributorCache {
    files: HashMap<PathBuf, String>,
}

impl FileDistributorCache {
    pub fn load() -> Self {
        match std::fs::read_to_string(".distributor/distributor_cache_db") {
            Ok(cache_str) => {
                return toml::from_str(cache_str.as_str()).unwrap_or_default();
            }
            Err(_) => {
                println!("cached file not exist.")
            }
        }

        FileDistributorCache::default()
    }

    pub fn save(&self) -> DistributorResult<()> {
        let cache_str = toml::to_string(self).unwrap();
        let cache_dir: &Path = Path::new(".distributor/distributor_cache_db");

        if let Some(parent) = cache_dir.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(".distributor/distributor_cache_db", cache_str)?;
        Ok(())
    }

    fn get_file_record(&self, file_path: &Path) -> Option<u128> {
        self.files
            .get(file_path)
            .map(|t| t.parse().unwrap())
    }

    pub fn update_file_record(&mut self, file_path: &Path) {
        if let Ok(timestamp) = get_file_last_modified_timestamp(file_path) {
            self.files.insert(file_path.to_path_buf(), timestamp.to_string());
        }
    }

    pub fn is_file_outdated(&self, file_path: &Path) -> bool {
        if let Some(timestamp) = self.get_file_record(file_path) {
            if let Ok(current_timestamp) = get_file_last_modified_timestamp(file_path) {
                return current_timestamp > timestamp;
            }
        }
        return true;
    }
}

/// 获取指定文件的最后修改时间.
///
/// # Param
///
/// - `file_path` - 文件路径.
fn get_file_last_modified_timestamp(file_path: &Path) -> QueryMetaResult<u128> {
    let meta = std::fs::metadata(file_path)?;
    let result = meta.modified()?.duration_since(std::time::SystemTime::UNIX_EPOCH);
    return Ok(result.map(|d| d.as_millis()).unwrap());
}