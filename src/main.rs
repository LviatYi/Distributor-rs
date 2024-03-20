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
    /// add target of source.
    Add {
        /// source path.
        #[arg(short, long)]
        source: PathBuf,
        /// target path.
        #[arg(short, long)]
        target: PathBuf,
    },
    /// remove target of source.
    /// if no target is provided, remove them all.
    Remove {
        /// source path.
        #[arg(short, long)]
        source: PathBuf,
        /// target path.
        #[arg(short, long, required = false)]
        target: Option<PathBuf>,
    },
    /// print config.
    List {},
    Run,
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
    cli.command.map(|command| match command {
        Commands::Add { source, target } => {
            config.add_target_to_source(source.as_path(), target.as_path());
            config.save_to(config_path.as_ref());
        }
        Commands::Remove { source, target } => {
            if let Some(t) = target {
                config.remove_target_from_source(source.as_path(), t.as_path());
                config.save_to(config_path.as_ref());
            } else {
                config.remove_source(source.as_path());
                config.save_to(config_path.as_ref());
            }
        }
        Commands::List {} => {
            println!("{:?}", config);
        }
        Commands::Run => {
            let mut distributor = distributor::Distributor::new();
            config.iter().for_each(|config_item| {
                config_item.to.iter().for_each(|to| {
                    let result = distributor.copy_to(config_item.from.as_path(), to.as_path(), true)
                                            .map(|results| {
                                                for result in results {
                                                    println!("{}", result);
                                                };
                                            });
                    if result.is_err() {
                        println!("{:?}", result.err().unwrap());
                    }
                });
            });
        }
    });
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
    println!("⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠐⠒⠒⠒⠒⠚⠛⣿⡟⠄⠄⢠⠄⠄⠄⡄⠄⠄⣠⡶⠶⣶⠶⠶⠂⣠⣶⣶⠂⠄⣸⡿⠄⠄⢀⣿⠇⠄⣰⡿⣠⡾⠋⠄⣼⡟⠄⣠⡾⠋⣾⠏⠄⢰⣿⠁⠄⠄⣾⡏⠄⠠⠿⠿⠋⠠⠶⠶⠿⠶⠾⠋⠄⠽⠟⠄⠄⠄⠃⠄⠄⣼⣿⣤⡤⠤⠤⠤⠤⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄⠄");
}