use std::fs::File;
use std::io::Read;
use std::path::{Path};
use crate::distributor_cache_db::FileDistributorCache;

#[derive(Debug)]
pub enum DistributorError {
    IoError(std::io::Error),
    InvalidInput(String),
}

impl From<std::io::Error> for DistributorError {
    fn from(e: std::io::Error) -> Self {
        DistributorError::IoError(e)
    }
}

pub type DistributorResult<T> = Result<T, DistributorError>;

pub struct Distributor {
    pub db_cache: FileDistributorCache,
}

impl Distributor {
    pub fn new() -> Self {
        Distributor {
            db_cache: FileDistributorCache::load(),
        }
    }

    /// Copy file to target paths.
    ///
    /// # Param
    ///
    /// - `source_path` - 要复制的文件的路径。如果是文件夹，将会递归复制文件夹中的文件。
    /// - `target_path` - 目标文件的路径，如果是文件夹，将会在文件夹中创建一个与源文件同名的文件。
    /// - `recursion` - 是否递归复制文件夹中的文件。
    pub fn copy_to(&mut self,
                   source_path: &Path,
                   target_path: &Path,
                   recursion: bool,
                   update_record: bool) -> DistributorResult<Vec<String>> {
        if source_path.is_dir() {
            if target_path.is_file() {
                return Err(DistributorError::InvalidInput("target need to be a dir when source is dir.".to_string()));
            }
            match std::fs::read_dir(source_path) {
                Ok(entries) => {
                    let mut successed = Vec::new();
                    for sub_source_path in entries
                        .filter_map(|result|
                            result
                                .and_then(|e| Ok(e.path()))
                                .ok()) {
                        let result;
                        if sub_source_path.is_dir() && recursion {
                            result = self.copy_to(&sub_source_path, &target_path.join(sub_source_path.file_name().unwrap()), recursion, update_record);
                        } else {
                            result = self.copy_to(&sub_source_path, target_path, recursion, update_record);
                        }
                        match result {
                            Ok(paths) => {
                                successed.extend(paths);
                            }
                            Err(e) => {
                                return Err(e);
                            }
                        }
                    }

                    Ok(successed)
                }
                Err(e) => { Err(DistributorError::IoError(e)) }
            }
        } else {
            let file_name = source_path
                .file_name()
                .and_then(|item| item.to_str())
                .ok_or(DistributorError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidInput, "file name is invalid.")))?;

            let final_target_path = if target_path.is_dir() {
                target_path.join(file_name)
            } else {
                target_path.to_path_buf()
            };

            if final_target_path.exists() && !self.db_cache.is_file_outdated(source_path) {
                println!("[LATEST] file at {:?} is up to date. skip.", final_target_path);
                return Ok(vec![]);
            }

            let result = copy_file_to_with_default_name(source_path, target_path, file_name);
            match result {
                Ok(path) => {
                    if update_record { self.db_cache.update_file_record(source_path); }
                    Ok(vec![path])
                }
                Err(e) => { Err(e) }
            }
        }
    }
}

impl Drop for Distributor {
    fn drop(&mut self) {
        println!("save cache.");
        let _ = self.db_cache.save();
    }
}

/// Copy file to full  target paths.
///
/// # Param
///
/// - `source_file_path` - 要复制的文件的路径。
/// - `target_file_path` - 目标文件的路径，包括文件名。如果路径中的目录不存在，将会被创建。
pub fn copy_file_with_full_target_path(source_file_path: &Path, target_file_path: &Path) -> DistributorResult<String> {
    if target_file_path.exists() {
        if let Ok(cmp_result) = compare_file(source_file_path, target_file_path) {
            if cmp_result {
                return Ok(format!("[Same] {}", target_file_path.to_str().unwrap().to_string()));
            }
        }
    }
    return match std::fs::read(source_file_path) {
        Ok(content) => {
            if let Some(parent_path) = target_file_path.parent() {
                if !parent_path.exists() {
                    std::fs::create_dir_all(parent_path)?;
                }
            }
            return match std::fs::write(target_file_path, &content) {
                Ok(_) => { Ok(format!("[Copied] {}", target_file_path.to_str().unwrap().to_string())) }
                Err(e) => { Err(DistributorError::IoError(e)) }
            };
        }
        Err(e) => {
            Err(DistributorError::IoError(e))
        }
    };
}

/// Copy file to target path with default name.
///
/// # Param
///
/// - `source_file_path` - 要复制的文件的路径。
/// - `target_path` - 目标文件的路径，如果是文件夹，将会在文件夹中创建一个与源文件同名的文件。
/// - `default_name` - 如果目标路径是文件夹，将会使用这个默认文件名。
pub fn copy_file_to_with_default_name(source_file_path: &Path, target_path: &Path, default_name: &str) -> DistributorResult<String> {
    if target_path.is_file() {
        copy_file_with_full_target_path(source_file_path, target_path)
    } else {
        copy_file_with_full_target_path(source_file_path, &target_path.join(default_name))
    }
}

#[derive(Debug)]
pub enum FileCompareError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for FileCompareError {
    fn from(e: std::io::Error) -> Self {
        FileCompareError::IoError(e)
    }
}

pub type FileCompareResult = Result<bool, FileCompareError>;

/// 比较文件内容。
///
/// # Param
///
/// - source_path - 源文件路径
/// - target_path - 目标文件路径
fn compare_file(source_path: &Path, target_path: &Path) -> FileCompareResult {
    let mut file_source_result = File::open(source_path)?;
    let mut file_target_result = File::open(target_path)?;

    let mut buffer_1 = [0u8; 1024];
    let mut buffer_2 = [0u8; 1024];
    loop {
        let size_1 = file_source_result.read(&mut buffer_1)?;
        let size_2 = file_target_result.read(&mut buffer_2)?;
        if size_1 != size_2 || buffer_1[..size_1] != buffer_2[..size_2] { return Ok(false); }
        if size_1 == size_2 && size_1 == 0 { return Ok(true); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_to() {
        let file_path = Path::new(&"resource/");
        let target_path = Path::new("test-target/copy-to/");

        let _ = Distributor::new().copy_to(file_path, &target_path, true, false);

        assert_eq!(
            std::fs::read_to_string(file_path.join("sub-resource-dir-a/template-a.txt")).unwrap(),
            std::fs::read_to_string(Path::new("test-target/copy-to/sub-resource-dir-a/template-a.txt")).unwrap(),
        );
        assert_eq!(
            std::fs::read_to_string(file_path.join("sub-resource-dir-b/template-b.txt")).unwrap(),
            std::fs::read_to_string(Path::new("test-target/copy-to/sub-resource-dir-b/template-b.txt")).unwrap(),
        );
        assert_eq!(
            std::fs::read_to_string(file_path.join("template.txt")).unwrap(),
            std::fs::read_to_string(Path::new("test-target/copy-to/template.txt")).unwrap(),
        );
    }

    #[test]
    fn test_copy_file_all_full() {
        let source_path = Path::new("resource/template.txt");
        let target_path = Path::new("test-target/copy_file_all_full/test.txt");

        let _ = copy_file_with_full_target_path(source_path, target_path);

        assert_eq!(
            std::fs::read_to_string(source_path).unwrap(),
            std::fs ::read_to_string(target_path).unwrap(),
        )
    }

    #[test]
    fn test_copy_file_with_no_target_file_name() {
        let source_path = Path::new("resource/template.txt");
        let target_path = Path::new("test-target/copy_file_with_no_target_file_name/");

        let _ = copy_file_to_with_default_name(source_path, target_path, "template.txt");

        assert_eq!(
            std::fs::read_to_string(source_path).unwrap(),
            std::fs::read_to_string(target_path.join("template.txt")).unwrap(),
        )
    }

    #[test]
    fn test_compare_file() {
        let source_path = Path::new("resource/sub-resource-dir-a/template-a.txt");
        let target_path = Path::new("resource/sub-resource-dir-b/template-b.txt");

        assert_eq!(
            compare_file(source_path, target_path).unwrap(),
            false,
        );

        let source_path = Path::new("resource/sub-resource-dir-a/template-a.txt");
        let target_path = Path::new("resource/sub-resource-dir-a/template-c.txt");

        assert_eq!(
            compare_file(source_path, target_path).unwrap(),
            true,
        );
    }
}