mod fs;
mod host;
mod scan;
mod sync;

use anyhow::Result;

use path_slash::PathExt;

use clap::{Parser, Subcommand};

use host::Host;
use scan::DirectoryScanList;
use sync::Sync;

#[derive(Parser, Debug)]
#[command(name = "git-repo-sync", about = "Git repo sync utility", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Local code root directory.
    #[arg(short, long)]
    local_dir: Option<std::path::PathBuf>,

    /// Whether or not to print verbose logging.
    #[arg(short, long)]
    verbose: bool,

    /// Whether to perform a dry-run.
    #[arg(short, long)]
    dry: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Upload code to remote.
    Up { remote: Remote },
    /// Download code from remote.
    Down { remote: Remote },
}

#[derive(Clone, Debug)]
struct Remote {
    host: Host,
    dir: std::path::PathBuf,
}

impl std::str::FromStr for Remote {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        if let Some((host, dir)) = s.split_once(':') {
            let dir = strip_path_trailing_sep(std::path::PathBuf::from(dir));
            // XXX: Prefixing with ~ to designate home does not work with SFTP, but just using a
            // relative path already will start from home, so stripping it here has the same effect
            // and works fine.
            let dir = if let Ok(stripped_dir) = dir.strip_prefix("~/") {
                stripped_dir.to_path_buf()
            } else {
                dir
            };
            Ok(Remote {
                host: Host::new(host),
                dir,
            })
        } else {
            Err(anyhow::anyhow!("invalid remote: {s}"))
        }
    }
}

impl std::fmt::Display for Remote {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.dir.to_slash_lossy())
    }
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("error: {:#}", err);
        }
    }
}

fn run() -> Result<()> {
    let Cli {
        command,
        local_dir,
        verbose,
        dry,
    } = Cli::parse();

    let local_dir = match local_dir {
        Some(local_dir) => local_dir,
        None => {
            if let Ok(current_dir) = std::env::current_dir() {
                current_dir
            } else {
                return Err(anyhow::anyhow!("failed to determine current directory"));
            }
        }
    };
    let local_dir = strip_path_trailing_sep(local_dir);

    if verbose {
        println!("verbose = {verbose}");
        println!("local dir = {}", local_dir.to_slash_lossy());
    }

    let scan_local =
        DirectoryScanList::from_local_file_system(&local_dir)?.filter_by_gitignore(&local_dir)?;
    if verbose {
        println!(
            "scanned local directory and found {} directories and {} files",
            scan_local.directories().len(),
            scan_local.files().len(),
        );
    }

    let scan_remote_fn = |remote: &Remote| -> Result<DirectoryScanList> {
        let scan_remote = DirectoryScanList::from_remote_over_ssh(&remote.dir, &remote.host)?
            .filter_by_gitignore(&local_dir)?;
        if verbose {
            println!(
                "scanned remote directory and found {} directories and {} files",
                scan_remote.directories().len(),
                scan_remote.files().len(),
            );
        }
        Ok(scan_remote)
    };

    match command {
        Command::Up { remote } => {
            let scan_remote = scan_remote_fn(&remote)?;
            let sync = Sync::unidirectional(scan_local, scan_remote);
            if !dry {
                sync.execute_remote(&local_dir, &remote.dir, &remote.host)?;
                if verbose {
                    print_sync_summary(&sync, &remote.host);
                }
            } else {
                print_sync_dry(&sync, local_dir.to_slash_lossy(), &remote);
            }
            Ok(())
        }
        Command::Down { remote } => {
            let scan_remote = scan_remote_fn(&remote)?;
            let sync = Sync::unidirectional(scan_remote, scan_local);
            if !dry {
                sync.execute_local(&local_dir, &remote.dir, &remote.host)?;
                if verbose {
                    print_sync_summary(&sync, "local host");
                }
            } else {
                print_sync_dry(&sync, &remote, local_dir.to_slash_lossy());
            }
            Ok(())
        }
    }
}

fn print_sync_summary(sync: &Sync, target: impl std::fmt::Display) {
    println!("removed {} files on {target}", sync.remove_files().len());
    println!(
        "removed {} directories on {target}",
        sync.remove_directories().len()
    );
    println!(
        "created {} directories on {target}",
        sync.create_directories().len()
    );
    println!("copied {} files to {target}", sync.copy_files().len());
}

fn print_sync_dry(
    sync: &Sync,
    source_prefix: impl std::fmt::Display,
    target_prefix: impl std::fmt::Display,
) {
    for file in sync.remove_files() {
        println!("remove file: {}/{}", target_prefix, file.to_slash_lossy());
    }
    for directory in sync.remove_directories() {
        println!(
            "remove directory: {}/{}",
            target_prefix,
            directory.to_slash_lossy()
        );
    }
    for directory in sync.create_directories() {
        println!(
            "create directory: {}/{}",
            target_prefix,
            directory.to_slash_lossy()
        );
    }
    for file in sync.copy_files() {
        println!(
            "copy file: {}/{} -> {}/{}",
            source_prefix,
            file.to_slash_lossy(),
            target_prefix,
            file.to_slash_lossy()
        );
    }
}

fn strip_path_trailing_sep(p: std::path::PathBuf) -> std::path::PathBuf {
    let p_str = p.to_string_lossy().to_string();
    if !p_str.is_empty() {
        if let Some(p_str_stripped) = p_str.strip_suffix(std::path::MAIN_SEPARATOR) {
            std::path::PathBuf::from(p_str_stripped)
        } else {
            p
        }
    } else {
        p
    }
}
