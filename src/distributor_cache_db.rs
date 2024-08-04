use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::distributor::{DistributorResult, DistributorResultType};

#[derive(Debug)]
pub enum QueryMetaError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for QueryMetaError {
    fn from(e: std::io::Error) -> Self {
        QueryMetaError::IoError(e)
    }
}

static DEFAULT_DB_PATH: &str = ".distributor/distributor_cache.db";

pub type QueryMetaResult<T> = Result<T, QueryMetaError>;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct FileDistributorCache {
    files_touch_time_record: HashMap<PathBuf, String>,

    loaded_path: PathBuf,
}

impl FileDistributorCache {
    pub fn load(path: Option<&Path>) -> Self {
        let path = path.unwrap_or(Path::new(DEFAULT_DB_PATH));
        let mut dtb_cache: Self;
        match std::fs::read_to_string(path) {
            Ok(cache_str) => {
                dtb_cache = bincode::deserialize(cache_str.as_bytes()).unwrap_or_default();
            }
            Err(_) => {
                println!("cached file not exist.");
                dtb_cache = FileDistributorCache::default();
            }
        }

        dtb_cache.loaded_path = path.to_path_buf();
        dtb_cache
    }

    pub fn save(&self, path: Option<&Path>) -> DistributorResult {
        let path = path.unwrap_or(self.loaded_path.as_path());
        let cache_str = bincode::serialize(self).unwrap();

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        std::fs::write(path, cache_str)?;
        Ok(DistributorResultType::Saved)
    }

    pub fn update_file_record(&mut self, file_path: &Path) {
        if let Ok(timestamp) = get_file_last_modified_timestamp(file_path) {
            self.files_touch_time_record.insert(
                file_path.to_path_buf(),
                timestamp.to_string());
        }
    }

    pub fn is_file_outdated(&self, file_path: &Path) -> bool {
        if let Some(distribute_time) = self.get_file_record(file_path) {
            if let Ok(last_change) = get_file_last_modified_timestamp(file_path) {
                return last_change > distribute_time;
            }
        }

        true
    }

    pub fn clear(path: Option<&Path>) -> std::io::Result<()> {
        let path = path.unwrap_or(Path::new(DEFAULT_DB_PATH));
        std::fs::remove_file(path)
    }

    fn get_file_record(&self, file_path: &Path) -> Option<u128> {
        self.files_touch_time_record
            .get(file_path)
            .map(|t| t.parse().unwrap())
    }

    pub fn is_empty(&self) -> bool {
        self.files_touch_time_record.is_empty()
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
    Ok(result.map(|d| d.as_millis()).unwrap())
}