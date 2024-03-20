use std::{fs};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum DistributorConfigError {
    Existed,
    NotExist,
}

type DistributorConfigResult = Result<(), DistributorConfigError>;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct DistributorItem {
    pub from: PathBuf,
    pub to: Vec<PathBuf>,
}

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

    pub fn add_target_to_source(&mut self, source: &Path, target: &Path) -> DistributorConfigResult {
        if let Some(item) = self.items
                                .iter_mut()
                                .find(|item| item.from == source) {
            if item.to.iter().any(|p| p == target) {
                return Err(DistributorConfigError::Existed);
            }
            item.to.push(target.to_path_buf());
        } else {
            self.items.push(DistributorItem {
                from: source.to_path_buf(),
                to: vec![target.to_path_buf()],
            });
        }
        return Ok(());
    }

    pub fn remove_target_from_source(&mut self, source: &Path, target: &Path) -> DistributorConfigResult {
        if let Some(item) = self.items.iter_mut().find(|item| item.from == source) {
            if let Some(index) = item.to.iter().position(|t| t == target) {
                item.to.remove(index);
                return Ok(());
            }
        }
        return Err(DistributorConfigError::NotExist);
    }

    pub fn remove_source(&mut self, source: &Path) -> DistributorConfigResult {
        if let Some(index) = self.items.iter().position(|item| item.from == source) {
            self.items.remove(index);
            return Ok(());
        }
        return Err(DistributorConfigError::NotExist);
    }

    pub fn save_to(&self, path: &Path) {
        let config_str = toml::to_string(self).unwrap();
        let path = Path::new(path);

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


#[cfg(test)]
mod tests {
    use std::sync::Once;
    use super::*;

    static INIT: Once = Once::new();

    #[test]
    fn initialize() {
        INIT.call_once(|| {
            match fs::remove_dir_all("test-target") {
                Ok(_) => { println!("init finished.") }
                Err(e) => { println!("init failed. {e:?}") }
            };
        });
    }

    #[test]
    fn test_save_config() {
        initialize();
        let config = DistributorConfiguration {
            items: vec![
                DistributorItem {
                    from: PathBuf::new().join("resource/template.txt"),
                    to: vec![PathBuf::new().join("test-target/config")],
                },
            ],
        };
        config.save_to(Path::new("test-target/test-distributor-config.toml"));

        assert_eq!(
            fs::read_to_string("test-target/test-distributor-config.toml").unwrap(),
            toml::to_string(&config).unwrap(),
            );
    }

    #[test]
    fn test_read_config() {
        initialize();
        let config = DistributorConfiguration::read_from(Path::new("resource/test-distributor-config.toml"));

        assert_eq!(
            config,
            DistributorConfiguration {
                items: vec![
                    DistributorItem {
                        from: PathBuf::new().join("resource/template.txt"),
                        to: vec![PathBuf::new().join("test-target/config")],
                    },
                ],
            }
        )
    }

    #[test]
    fn test_update_config() {
        initialize();
        let mut config = DistributorConfiguration::read_from(Path::new("resource/test-distributor-config.toml"));

        println!("add target to source");
        let _ = config.add_target_to_source(Path::new("resource/template.txt"), Path::new("test-target/config2"));

        assert_eq!(
            config,
            DistributorConfiguration {
                items: vec![
                    DistributorItem {
                        from: PathBuf::new().join("resource/template.txt"),
                        to: vec![
                            PathBuf::new().join("test-target/config"),
                            PathBuf::new().join("test-target/config2"),
                        ],
                    },
                ],
            }
        );

        println!("add target to source");
        let _ = config.add_target_to_source(Path::new("resource/template2.txt"), Path::new("test-target/config"));

        assert_eq!(
            config,
            DistributorConfiguration {
                items: vec![
                    DistributorItem {
                        from: PathBuf::new().join("resource/template.txt"),
                        to: vec![
                            PathBuf::new().join("test-target/config"),
                            PathBuf::new().join("test-target/config2"),
                        ],
                    },
                    DistributorItem {
                        from: PathBuf::new().join("resource/template2.txt"),
                        to: vec![
                            PathBuf::new().join("test-target/config"),
                        ],
                    },
                ],
            }
        );

        println!("remove source");
        let _ = config.remove_source(Path::new("resource/template2.txt"));

        assert_eq!(
            config,
            DistributorConfiguration {
                items: vec![
                    DistributorItem {
                        from: PathBuf::new().join("resource/template.txt"),
                        to: vec![
                            PathBuf::new().join("test-target/config"),
                            PathBuf::new().join("test-target/config2"),
                        ],
                    },
                ],
            }
        );

        println!("remove target from source");
        let _ = config.remove_target_from_source(Path::new("resource/template.txt"), Path::new("test-target/config2"));

        assert_eq!(
            config,
            DistributorConfiguration {
                items: vec![
                    DistributorItem {
                        from: PathBuf::new().join("resource/template.txt"),
                        to: vec![
                            PathBuf::new().join("test-target/config"),
                        ],
                    },
                ],
            }
        );
    }
}