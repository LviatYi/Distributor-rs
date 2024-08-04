use std::borrow::Cow;
use std::env;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use crate::distributor_config::DistributorConfiguration;

mod distributor;
mod distributor_config;
mod distributor_cache_db;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the output texture.
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,

    /// do not reset working directory to the directory of the executable.
    #[arg(short, long)]
    no_reset_working_directory: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// add distributor.
    Add {
        /// distributor name.
        name: String,
        /// source root path.
        #[arg(short, long)]
        root: Option<PathBuf>,
        /// target path.
        #[arg(short, long)]
        target: Option<PathBuf>,
    },
    /// add ignore glob of source.
    Ignore {
        /// distributor name.
        name: String,
        /// ignore glob path.
        #[arg(short, long)]
        glob: String,
    },
    /// remove target of source.
    /// if no target is provided, remove them all.
    Remove {
        /// distributor name.
        name: String,
        /// target path.
        #[arg(short, long)]
        target: Option<PathBuf>,
    },
    /// print config.
    List,
    /// clear cache.
    Clear,
    /// run distributor.
    Run {
        /// force run copy.
        #[arg(short, long)]
        force: bool,

        /// silence output.
        #[arg(short, long)]
        silence: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    show_welcome();

    if !cli.no_reset_working_directory {
        set_exe_path_as_current();
    }

    let mut config: DistributorConfiguration;

    let config_path: Cow<'static, Path> = if let Some(cp) = cli.config {
        Cow::Owned(cp)
    } else {
        Cow::Borrowed(Path::new("distributor-config.toml"))
    };

    config = DistributorConfiguration::read_from(config_path.as_ref());
    if let Some(command) = cli.command {
        match command {
            Commands::Add { name, root, target } => {
                if !config.has_distributor(name.as_str()) {
                    if let Some(root) = root {
                        let result = config.add_distributor(name.as_str(),
                                                            root.as_path());
                        if let Err(e) = result {
                            println!("add distributor failed. {:?}", e);
                            return;
                        }
                    } else {
                        println!("add distributor failed. root path is required.");
                        return;
                    }
                }

                if let Some(t) = target {
                    config.add_target(&name, &t).expect("add target failed.");
                }

                config.save_to(config_path.as_ref());
            }
            Commands::Ignore { name, glob } => {
                if config.add_ignore(&name, glob.as_str()).is_ok() {
                    config.save_to(config_path.as_ref());
                }
            }
            Commands::Remove { name, target } => {
                if let Some(t) = target {
                    if config.remove_target(&name, t.as_path()).is_ok() {
                        config.save_to(config_path.as_ref());
                    }
                } else if config.remove_distributor(&name).is_ok() {
                    config.save_to(config_path);
                }
            }
            Commands::List {} => {
                println!("{:#?}", config);
            }
            Commands::Run { force, silence } => {
                let mut distributor = distributor::Distributor::new();
                config.iter().for_each(|config_item| {
                    distributor.do_copy(config_item, force, !silence);
                });
            }
            Commands::Clear => {
                let mut distributor = distributor::Distributor::new();
                distributor.clear_cache()
            }
        }
    }

    fn set_exe_path_as_current() {
        println!("reset working directory.");
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let _ = env::set_current_dir(&exe_dir);
                println!("Current directory: {:?}", env::current_dir().unwrap());
            }
        }
    }

    fn show_welcome() {
        println!("⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠐⠒⠒⠒⠒⠚⠛⣿⡟⠄⠄⢠⠄⠄⠄⡄⠄⠄⣠⡶⠶⣶⠶⠶⠂⣠⣶⣶⠂⠄⣸⡿⠄⠄⢀⣿⠇⠄⣰⡿⣠⡾⠋⠄⣼⡟⠄⣠⡾⠋⣾⠏⠄⢰⣿⠁⠄⠄⣾⡏⠄⠠⠿⠿⠋⠠⠶⠶⠿⠶⠾⠋⠄⠽⠟⠄⠄⠄⠃⠄⠄⣼⣿⣤⡤⠤⠤⠤⠤⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄");
        println!("Welcome to Distributor!");
    }
}