use std::io::Write;

use anyhow::{Context, Result};

use path_slash::PathExt;

use crate::host::Host;
use crate::scan::DirectoryScanList;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sync {
    remove_files: Vec<std::path::PathBuf>,
    remove_directories: Vec<std::path::PathBuf>,
    create_directories: Vec<std::path::PathBuf>,
    copy_files: Vec<std::path::PathBuf>,
}

impl Sync {
    pub fn unidirectional(source: DirectoryScanList, target: DirectoryScanList) -> Sync {
        let mut remove_files = Vec::new();
        let mut remove_directories = Vec::new();
        let mut create_directories = Vec::new();
        let mut copy_files = Vec::new();

        let (mut source_directories, mut source_files) = source.into_parts();
        let (mut target_directories, mut target_files) = target.into_parts();

        source_directories.sort_by_key(|directory| directory.path.clone());
        target_directories.sort_by_key(|directory| directory.path.clone());
        let mut source_directories = std::collections::VecDeque::from(source_directories);
        let mut target_directories = std::collections::VecDeque::from(target_directories);
        loop {
            match (
                !source_directories.is_empty(),
                !target_directories.is_empty(),
            ) {
                (true, true) => match source_directories[0].path.cmp(&target_directories[0].path) {
                    std::cmp::Ordering::Equal => {
                        // Nothing to do except for popping them from the queue.
                        source_directories.pop_front().unwrap();
                        target_directories.pop_front().unwrap();
                    }
                    std::cmp::Ordering::Less => {
                        // Target list is missing directory.
                        let source_directory = source_directories.pop_front().unwrap();
                        create_directories.push(source_directory.path);
                    }
                    std::cmp::Ordering::Greater => {
                        // Target list has directory that we do not have.
                        let target_directory = target_directories.pop_front().unwrap();
                        remove_directories.push(target_directory.path);
                    }
                },
                (true, false) => {
                    // Target list is missing directory.
                    let source_directory = source_directories.pop_front().unwrap();
                    create_directories.push(source_directory.path);
                }
                (false, true) => {
                    // Target list has directory that we do not have.
                    let target_directory = target_directories.pop_front().unwrap();
                    remove_directories.push(target_directory.path);
                }
                (false, false) => {
                    break;
                }
            }
        }

        source_files.sort_by_key(|file| file.path.clone());
        target_files.sort_by_key(|file| file.path.clone());
        let mut source_files = std::collections::VecDeque::from(source_files);
        let mut target_files = std::collections::VecDeque::from(target_files);
        loop {
            match (!source_files.is_empty(), !target_files.is_empty()) {
                (true, true) => match source_files[0].path.cmp(&target_files[0].path) {
                    std::cmp::Ordering::Equal => {
                        let source_file = source_files.pop_front().unwrap();
                        let target_file = target_files.pop_front().unwrap();
                        if source_file.size != target_file.size {
                            copy_files.push(source_file.path);
                        }
                    }
                    std::cmp::Ordering::Less => {
                        // Target list is missing file.
                        let source_file = source_files.pop_front().unwrap();
                        copy_files.push(source_file.path);
                    }
                    std::cmp::Ordering::Greater => {
                        // Target list has file that we do not have.
                        let target_file = target_files.pop_front().unwrap();
                        remove_files.push(target_file.path);
                    }
                },
                (true, false) => {
                    // Target list is missing file.
                    let source_file = source_files.pop_front().unwrap();
                    copy_files.push(source_file.path);
                }
                (false, true) => {
                    // Target list has file that we do not have.
                    let target_file = target_files.pop_front().unwrap();
                    remove_files.push(target_file.path);
                }
                (false, false) => {
                    break;
                }
            }
        }

        Sync {
            remove_directories,
            remove_files,
            create_directories,
            copy_files,
        }
    }

    pub fn execute_remote(
        &self,
        local_path: &std::path::Path,
        remote_path: &std::path::Path,
        remote: &Host,
    ) -> Result<()> {
        // The order of operations is important:
        // 1. Remove files.
        // 2. Remove directories.
        // 3. Create directories.
        // 4. Copy files.
        //
        // This ordering makes sure that no conflicts arise:
        // * Files should be removed before directories to prevent removing non-empty directories.
        // * Files should be removed before directories are created to prevent file/directory naming
        //   conflicts.
        // * Files must be copied after directories are created to prevent copying files into
        //   directories that do not exist yet.

        let mut sftp_process = std::process::Command::new("sftp")
            // Batched mode triggers correct exit status code when one of the
            // operations fails.
            .args(["-b", "-"])
            .arg(format!("{remote}"))
            .stdin(std::process::Stdio::piped())
            // XXX: Pipe output to /dev/null. Not doing so will cause the stdout to fill up and
            // SFTP will stack blocking (both stdout and stderr must be piped).
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("failed to spawn sftp process")?;
        // XXX: Skipping deleting remote directories! To do it correctly (only remove directories
        // that are non-empty) there are two options: Either we do some magic to figure out if the
        // directory is empty beforehand (we could pull that info out of `DirectoryScanList`) OR
        // we'd need to run a separate "sftp" instance without batch mode (`-b`) in which case we
        // could just use `rmdir` and all non-empty dirs would be ignored. For now we do neither.
        // The reason that we can't delete directories without knowing if they have contents is
        // that it might be possible that the other side holds ignored files inside the directory.
        for file in &self.remove_files {
            writeln!(
                sftp_process.stdin.as_mut().unwrap(),
                "rm {}",
                remote_path.join(file).to_slash_lossy(),
            )
            .context("failed to write data to sftp process")?;
        }
        for directory in &self.create_directories {
            writeln!(
                sftp_process.stdin.as_mut().unwrap(),
                "mkdir {}",
                remote_path.join(directory).to_slash_lossy(),
            )
            .context("failed to write data to sftp process")?;
        }
        for file in &self.copy_files {
            writeln!(
                sftp_process.stdin.as_mut().unwrap(),
                "put {} {}",
                local_path.join(file).to_slash_lossy(),
                remote_path.join(file).to_slash_lossy(),
            )
            .context("failed to write data to sftp process")?;
        }
        let exit_status = sftp_process.wait().context("failed to run sftp command")?;
        if exit_status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("sftp failed: {exit_status}"))
        }
    }

    pub fn execute_local(
        &self,
        local_path: &std::path::Path,
        remote_path: &std::path::Path,
        remote: &Host,
    ) -> Result<()> {
        // The order of operations is important:
        // 1. Remove files.
        // 2. Remove directories.
        // 3. Create directories.
        // 4. Copy files.
        //
        // This ordering makes sure that no conflicts arise:
        // * Files should be removed before directories to prevent removing non-empty directories.
        // * Files should be removed before directories are created to prevent file/directory naming
        //   conflicts.
        // * Files must be copied after directories are created to prevent copying files into
        //   directories that do not exist yet.

        for file in &self.remove_files {
            std::fs::remove_file(local_path.join(file)).context("failed to remove file")?;
        }
        for directory in &self.remove_directories {
            // XXX: Only remove the target directory if it is empty! It is possible that the target
            // directory contains ignored files that are not present on the source, which should
            // not be removed.
            if std::fs::read_dir(local_path.join(directory))
                .context("failed to open directory")?
                .next()
                .is_none()
            {
                std::fs::remove_dir(local_path.join(directory))
                    .context("failed to remove directory")?;
            }
        }
        for directory in &self.create_directories {
            std::fs::create_dir_all(directory).context("failed to create directory")?;
        }
        let mut sftp_process = std::process::Command::new("sftp")
            // Batched mode triggers correct exit status code when one of the
            // operations fails.
            .args(["-b", "-"])
            .arg(format!("{remote}"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("failed to spawn sftp process")?;
        for file in &self.copy_files {
            writeln!(
                sftp_process.stdin.as_mut().unwrap(),
                "get {} {}",
                remote_path.join(file).to_slash_lossy(),
                local_path.join(file).to_slash_lossy(),
            )
            .context("failed to write data to sftp process")?;
        }
        let exit_status = sftp_process.wait().context("failed to run sftp command")?;
        if exit_status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("sftp failed: {exit_status}"))
        }
    }

    pub fn remove_files(&self) -> &[std::path::PathBuf] {
        &self.remove_files
    }

    pub fn remove_directories(&self) -> &[std::path::PathBuf] {
        &self.remove_directories
    }

    pub fn create_directories(&self) -> &[std::path::PathBuf] {
        &self.create_directories
    }

    pub fn copy_files(&self) -> &[std::path::PathBuf] {
        &self.copy_files
    }
}
