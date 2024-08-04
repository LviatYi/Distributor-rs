use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::distributor::DistributorResultType::{Copied, Same, UpToDate};
use crate::distributor_cache_db::FileDistributorCache;
use crate::distributor_config::DistributorItem;

#[derive(Debug)]
pub enum DistributorError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for DistributorError {
    fn from(e: std::io::Error) -> Self {
        DistributorError::IoError(e)
    }
}

#[derive(Debug)]
pub enum DistributorResultType {
    Copied(String, String),
    Same(String, String),
    Saved,
    UpToDate(String),
}

pub type DistributorResult = Result<DistributorResultType, DistributorError>;

pub struct Distributor {
    pub db_cache: FileDistributorCache,
}

impl Distributor {
    pub fn new() -> Self {
        Distributor {
            db_cache: FileDistributorCache::load(None),
        }
    }

    pub fn do_copy(&mut self, config_item: &DistributorItem, force: bool, debug: bool) {
        let mut results = vec![];
        if config_item.is_point_to_file() {
            if !force && !self.db_cache.is_file_outdated(&config_item.root) {
                results.push(
                    Ok(DistributorResultType::UpToDate(
                        config_item.root
                                   .to_str()
                                   .unwrap()
                                   .to_string())));
            } else {
                let file_name = config_item.root
                                           .file_name()
                                           .and_then(|item| item.to_str())
                                           .ok_or(DistributorError::IoError(
                                               std::io::Error::new(std::io::ErrorKind::InvalidInput,
                                                                   "file name is invalid.")
                                           ))
                                           .unwrap();
                for to in config_item.to.iter() {
                    results.push(copy_file_to_with_default_name(
                        &config_item.root.to_path_buf(),
                        to,
                        file_name));
                }
                self.db_cache.update_file_record(&config_item.root);
            }
        } else if let Ok(source_set) = config_item.get_non_root_source_file() {
            let outdated_source: HashSet<&Path> = source_set
                .iter()
                .filter(|source| {
                    return if force || self.db_cache.is_file_outdated(source) {
                        true
                    } else {
                        results.push(Ok(UpToDate(source.to_str().unwrap().to_string())));
                        false
                    };
                })
                .map(|item| { item.as_path() })
                .collect();

            for to in config_item.to.iter() {
                self.copy_by_source_to(&config_item.root, &outdated_source, to)
                    .into_iter()
                    .for_each(|r| {
                        results.push(r);
                    });

                source_set.iter().for_each(|source| {
                    self.db_cache.update_file_record(source);
                });
            }
        }

        if debug {
            for result in results {
                match result {
                    Ok(tp) => {
                        match tp {
                            Copied(f, t) => {
                                println!("[Copied]{:?}{:?}", f, t);
                            }
                            Same(f, t) => {
                                println!("[Same]{:?}{:?}", f, t);
                            }
                            UpToDate(f) => {
                                println!("[UpToDate]{:?}", f);
                            }
                            DistributorResultType::Saved => {}
                        }
                        self.db_cache.update_file_record(&config_item.root);
                    }
                    Err(e) => {
                        println!("[Error {:?}]", e);
                    }
                }
            }
        }
    }

    /// Copy files by source_path to target dir.
    ///
    /// # Param
    ///
    /// - `root` - 待复制的文件的根路径。
    /// - `source_path` - 待复制的文件的路径。
    /// - `to` - 目标目录。
    fn copy_by_source_to(&mut self,
                         root: &Path,
                         source_paths: impl IntoIterator<Item=impl AsRef<Path>>,
                         to: &Path) -> Vec<DistributorResult> {
        let mut successed: Vec<DistributorResult> = Vec::new();

        for source in source_paths {
            let target_path = to.join(source.as_ref().strip_prefix(root).unwrap());

            successed.push(copy_file_with_full_target_path(source.as_ref(), &target_path));
        }

        successed
    }

    pub fn clear_cache(&mut self) {
        let _ = FileDistributorCache::clear(None);
        self.db_cache = FileDistributorCache::default();
    }
}

impl Drop for Distributor {
    fn drop(&mut self) {
        if !self.db_cache.is_empty() {
            println!("save cache.");
            let _ = self.db_cache.save(None);
        }
    }
}

/// Copy file to full target paths.
///
/// # Param
///
/// - `source_file_path` - 待复制的文件的路径。
/// - `target_file_path` - 目标文件的路径，包括文件名。如果路径中的目录不存在，将会被创建。
pub fn copy_file_with_full_target_path(source_file_path: &Path,
                                       target_file_path: &Path) -> DistributorResult {
    if target_file_path.is_file() {
        if let Ok(cmp_result) = compare_file(source_file_path, target_file_path) {
            if cmp_result {
                return Ok(Same(source_file_path.to_str().unwrap().to_string(),
                               target_file_path.to_str().unwrap().to_string()));
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
            return match std::fs::write(target_file_path, content) {
                Ok(_) => {
                    Ok(Copied(source_file_path.to_str().unwrap().to_string(),
                              target_file_path.to_str().unwrap().to_string()))
                }
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
/// - `source_file_path` - 待复制的文件的路径。
/// - `target_path` - 目标文件的路径，如果是文件夹，将会在文件夹中创建一个与源文件同名的文件。
/// - `default_name` - 如果目标路径是文件夹，将会使用此默认文件名。
pub fn copy_file_to_with_default_name(source_file_path: &Path,
                                      target_path: &Path,
                                      default_name: &str) -> DistributorResult {
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
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_copy_to() {
        let file_path = Path::new(&"resource/");
        // let target_path = Path::new("test-target/copy-to/");

        // let _ = Distributor::new().copy_to(file_path, &target_path, true);

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

    #[test]
    fn lab() {
        println!("{:?}", std::env::current_dir().unwrap());
        let path = PathBuf::from("resource//////sub-resource-dir-a");
        let path2 = PathBuf::from("resource\\sub-resource-dir-a");

        let root = "resource";

        println!("{:?}", path.eq(&path2));
        if path.starts_with(root) {
            println!("{:?}", path.strip_prefix(root).unwrap());
        };
    }
}