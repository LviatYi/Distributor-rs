use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use glob::glob;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum DistributorConfigError {
    Existed,
    NotExist,
    InvalidGlob,
}

type DistributorConfigResult = Result<(), DistributorConfigError>;

/// # Distributor 配置条目
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct DistributorItem {
    /// distributor name
    pub name: String,

    /// source path
    /// 指向一个文件或目录。
    pub root: PathBuf,

    /// ignore glob
    /// 当 root 指向一个 Directory 时，将会忽略匹配的文件。
    pub ignore: Vec<String>,

    /// destination paths
    pub to: Vec<PathBuf>,
}

impl DistributorItem {
    /// 获取 DistributorItem 所有非根源文件。
    pub fn get_non_root_source_file(&self) -> Result<HashSet<PathBuf>, DistributorConfigError> {
        let mut set = HashSet::new();
        let root_clone = self.root.clone();
        if self.root.is_file() {
            return Ok(set);
        }

        let mut candidates = VecDeque::new();
        candidates.push_back(root_clone);

        let ignores = self.ignore.iter()
                          .map(|pattern| glob(
                              &format!("{}/**/{}",
                                       self.root.to_str().unwrap_or_default(),
                                       pattern))
                              .map_err(|_| DistributorConfigError::InvalidGlob))
                          .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .map(|p| p.map(|p| p.to_path_buf()).unwrap())
            .collect::<HashSet<_>>();

        while !candidates.is_empty() {
            if let Some(candidate) = candidates.pop_front() {
                if candidate.is_dir() {
                    for entry in fs::read_dir(candidate).unwrap() {
                        let entry = entry.unwrap();
                        let path = entry.path();

                        if path.is_dir() {
                            candidates.push_back(path);
                        } else if !ignores.contains(path.as_path()) {
                            set.insert(path);
                        }
                    }
                } else {
                    set.insert(candidate);
                }
            }
        }

        Ok(set)
    }

    /// 是否 DistributorItem 根指向单一文件。
    pub fn is_point_to_file(&self) -> bool {
        self.root.is_file()
    }
}

/// # Distributor 配置
#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct DistributorConfiguration {
    items: Vec<DistributorItem>,
}

impl DistributorConfiguration {
    pub fn read_from(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(config_str) => {
                return toml::from_str(config_str.as_str()).unwrap_or_default();
            }
            Err(_) => {
                println!("config file not exist.");
            }
        }

        DistributorConfiguration::default()
    }

    pub fn add_distributor(&mut self, name: &str, root: &Path) -> DistributorConfigResult {
        if self.items
               .iter_mut()
               .any(|item| item.name == name) {
            Err(DistributorConfigError::Existed)
        } else {
            self.items.push(DistributorItem {
                name: name.to_string(),
                root: root.to_path_buf(),
                ignore: vec![],
                to: vec![],
            });

            Ok(())
        }
    }

    pub fn has_distributor(&self, name: &str) -> bool {
        self.items.iter().any(|item| item.name == name)
    }

    pub fn remove_distributor(&mut self, name: &str) -> DistributorConfigResult {
        if let Some(index) = self.items.iter()
                                 .position(|item| item.name == name) {
            self.items.remove(index);

            return Ok(());
        }

        Err(DistributorConfigError::NotExist)
    }

    pub fn add_ignore(&mut self, name: &str, ignore_glob: &str) -> DistributorConfigResult {
        if let Some(item) = self.items
                                .iter_mut()
                                .find(|item| item.name == name) {
            if item.ignore.iter().any(|item| item == ignore_glob) {
                return Err(DistributorConfigError::Existed);
            }
            item.ignore.push(ignore_glob.to_string());

            Ok(())
        } else {
            Err(DistributorConfigError::NotExist)
        }
    }

    pub fn remove_ignore(&mut self, name: &str, ignore_glob: &str) -> DistributorConfigResult {
        if let Some(item) = self.items
                                .iter_mut()
                                .find(|item| item.name == name) {
            if let Some(index) = item.ignore.iter().position(|item| item == ignore_glob) {
                item.ignore.remove(index);

                return Ok(());
            }
        }

        Err(DistributorConfigError::NotExist)
    }

    pub fn add_target(&mut self, name: &str, target: &Path) -> DistributorConfigResult {
        if let Some(item) = self.items
                                .iter_mut()
                                .find(|item| item.name == name) {
            if item.to.iter().any(|item| item == target) {
                return Err(DistributorConfigError::Existed);
            }
            item.to.push(target.to_path_buf());
        } else {
            return Err(DistributorConfigError::NotExist);
        }

        Ok(())
    }

    pub fn remove_target(&mut self, name: &str, target: &Path) -> DistributorConfigResult {
        if let Some(item) = self.items
                                .iter_mut()
                                .find(|item| item.name == name) {
            if let Some(index) = item.to.iter().position(|item| item == target) {
                item.to.remove(index);

                return Ok(());
            }
        }

        Err(DistributorConfigError::NotExist)
    }

    pub fn save_to<P: AsRef<Path>>(&self, path: P) {
        let config_str = toml::to_string(self).unwrap();
        let path = Path::new(path.as_ref());

        if path.is_file() || path.extension().is_some() {
            if let Some(path_parent) = path.parent() {
                if !path_parent.exists() { let _ = fs::create_dir_all(path_parent); }
            }
            fs::write(path, config_str).unwrap();
        } else {
            if !path.exists() { let _ = fs::create_dir_all(path); }
            fs::write(path.join("distributor-config.toml"), config_str).unwrap();
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, DistributorItem> {
        self.items.iter()
    }
}

//region TTD

#[cfg(test)]
mod tests {
    use std::env;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_save_load_config() {
        let config_save_path = tempdir()
            .unwrap()
            .into_path()
            .join("test-distributor-config.toml");
        let config = DistributorConfiguration {
            items: vec![
                DistributorItem {
                    name: "test".to_string(),
                    root: PathBuf::from("resource/template.txt"),
                    ignore: vec![],
                    to: vec![PathBuf::from("test-target/config")],
                },
            ],
        };

        config.save_to(&config_save_path);

        assert_eq!(
            fs::read_to_string(&config_save_path).unwrap(),
            toml::to_string(&config).unwrap(),
            );

        let config = DistributorConfiguration::read_from(&config_save_path);

        assert_eq!(
            config,
            DistributorConfiguration {
                items: vec![
                    DistributorItem {
                        name: "test".to_string(),
                        root: PathBuf::from("resource/template.txt"),
                        ignore: vec![],
                        to: vec![PathBuf::from("test-target/config")],
                    },
                ],
            }
        )
    }

    #[test]
    fn test_update_config_add() {
        let mut config = DistributorConfiguration {
            items: vec![
                DistributorItem {
                    name: "test".to_string(),
                    root: PathBuf::from("resource"),
                    ignore: vec![],
                    to: vec![PathBuf::from("test-target/tar1")],
                },
            ],
        };

        println!("add ignore & target to distributor");
        let _ = config.add_ignore("test", "template.txt");
        let _ = config.add_target("test", Path::new("test-target/tar2"));

        assert_eq!(
            config,
            DistributorConfiguration {
                items: vec![
                    DistributorItem {
                        name: "test".to_string(),
                        root: PathBuf::from("resource"),
                        ignore: vec!["template.txt".to_string()],
                        to: vec![
                            PathBuf::new().join("test-target/tar1"),
                            PathBuf::new().join("test-target/tar2"),
                        ],
                    },
                ],
            }
        );
    }

    #[test]
    fn test_update_config_remove() {
        let mut config = DistributorConfiguration {
            items: vec![
                DistributorItem {
                    name: "test".to_string(),
                    root: PathBuf::from("resource"),
                    ignore: vec![
                        "template.txt".to_string(),
                        "template2.txt".to_string(),
                    ],
                    to: vec![
                        PathBuf::new().join("test-target/tar1"),
                        PathBuf::new().join("test-target/tar2"),
                    ],
                },
            ],
        };

        println!("remove from distributor");
        let _ = config.remove_ignore("test", "template2.txt");
        let _ = config.remove_target("test", Path::new("test-target/tar2"));

        assert_eq!(
            config,
            DistributorConfiguration {
                items: vec![
                    DistributorItem {
                        name: "test".to_string(),
                        root: PathBuf::from("resource"),
                        ignore: vec!["template.txt".to_string()],
                        to: vec![
                            PathBuf::new().join("test-target/tar1"),
                        ],
                    },
                ],
            }
        );

        println!("remove distributor");
        let _ = config.remove_distributor("test");

        assert_eq!(
            config,
            DistributorConfiguration {
                items: vec![],
            }
        );
    }

    #[test]
    fn test_get_source() {
        let temp_path = tempdir()
            .unwrap()
            .into_path();

        let origin_current_dir = env::current_dir().unwrap();
        let _ = env::set_current_dir(temp_path);

        let _ = fs::create_dir("resource");
        let _ = fs::write("resource/template.txt", "test1");
        let _ = fs::write("resource/template2.txt", "test2");

        let config = DistributorConfiguration {
            items: vec![
                DistributorItem {
                    name: "test".to_string(),
                    root: PathBuf::from("resource"),
                    ignore: vec![
                        "template.txt".to_string(),
                    ],
                    to: vec![],
                },
            ],
        };

        let res = config.items.get(0)
                        .unwrap()
                        .get_non_root_source_file()
                        .unwrap();

        println!("{:#?}", res);

        let _ = env::set_current_dir(origin_current_dir);
    }
}

//endregion ⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠐⠒⠒⠒⠒⠚⠛⣿⡟⠄⠄⢠⠄⠄⠄⡄⠄⠄⣠⡶⠶⣶⠶⠶⠂⣠⣶⣶⠂⠄⣸⡿⠄⠄⢀⣿⠇⠄⣰⡿⣠⡾⠋⠄⣼⡟⠄⣠⡾⠋⣾⠏⠄⢰⣿⠁⠄⠄⣾⡏⠄⠠⠿⠿⠋⠠⠶⠶⠿⠶⠾⠋⠄⠽⠟⠄⠄⠄⠃⠄⠄⣼⣿⣤⡤⠤⠤⠤⠤⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄
