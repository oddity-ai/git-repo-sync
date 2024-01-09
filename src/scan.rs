use std::io::{BufRead, Write};

use anyhow::{Context, Result};

use path_slash::PathExt;

use crate::fs::{Directory, File};
use crate::host::Host;

#[derive(Debug)]
pub struct DirectoryScanList {
    directories: Vec<Directory>,
    files: Vec<File>,
}

impl DirectoryScanList {
    /// Scans a local directory.
    ///
    /// Recursively finds every item in the directory. If one or more entries cannot be walked, the
    /// function fails as a whole.
    ///
    /// Note: Symlinks are ignored.
    ///
    /// # Arguments
    ///
    /// * `root` - Path of root directory to scan.
    pub fn from_local_file_system(root: &std::path::Path) -> Result<DirectoryScanList> {
        let mut directories = Vec::new();
        let mut files = Vec::new();
        for entry in walkdir::WalkDir::new(root).into_iter() {
            let entry = entry.context("failed to walk entry")?;
            let relative_path = entry.path().strip_prefix(root).unwrap().to_path_buf();
            if entry.file_type().is_file() {
                files.push(File::new(
                    relative_path,
                    entry
                        .metadata()
                        .context("failed to fetch file metadata")?
                        .len(),
                ));
            } else if entry.file_type().is_dir() && relative_path.components().count() > 0 {
                directories.push(Directory::new(relative_path));
            }
        }
        Ok(DirectoryScanList { directories, files })
    }

    /// Scans a remote directory.
    ///
    /// Internally, this function issues a `find` command on the remote host over SSH.
    ///
    /// Note: Symlinks are ignored.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to scan.
    /// * `target` - SSH host to scan.
    pub fn from_remote_over_ssh(
        path: &std::path::Path,
        target: &Host,
    ) -> Result<DirectoryScanList> {
        let output = std::process::Command::new("ssh")
            .args([
                format!("{target}"),
                // This command indexes the remote directory and file structure:
                //
                // First, it runs `mkdir -p` to create the target directory if it does not yet
                // exist.
                //
                // The `find` command is used to list all files and directories on the remote.
                // We're only interested in files and directories. The most portable method for
                // speciyfing this is by splitting up the invocation in two and use the `-o` option
                // to indicate that both invocations match. Apart from selecting a different type
                // of `-type f` versus `-type d`, the invocations are equivalent.
                //
                // The `-printf` options is used to format each file with the info that we'll be
                // needing:
                // * `%P`: the file path relative to the starting-point (the target directory).
                // * `%y`: the file type: `d` for directory, `f` for file.
                // * `%s`: the file size in bytes.
                //
                // The `-mindepth 1` makes sure that `find` does not print the starting-point
                // directory (we do not need it).
                format!(
                    "mkdir -p {0}; find {0} -type f -printf \"%P %y %s\n\" -mindepth 1 -o -type d -printf \"%P %y %s\n\" -mindepth 1",
                    path.to_slash_lossy()
                ),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("failed to spawn ssh command")?
            .wait_with_output()
            .context("failed to run ssh command")?;
        if output.status.success() {
            let stdout = String::from_utf8(output.stdout).unwrap_or_default();
            let stdout_lines = stdout.trim().lines();
            let mut directories = Vec::new();
            let mut files = Vec::new();
            for line in stdout_lines {
                if let Some((entry_p1, entry_size)) = line.trim().rsplit_once(' ') {
                    if let Some((entry_path, entry_type)) = entry_p1.trim().rsplit_once(' ') {
                        let path = std::path::Path::new(entry_path).to_path_buf();
                        match entry_type.trim() {
                            "f" => files.push(File::new(
                                path,
                                entry_size.parse().context("failed to parse file size")?,
                            )),
                            "d" => {
                                if path.components().count() > 0 {
                                    directories.push(Directory::new(path));
                                }
                            }
                            _ => {
                                return Err(anyhow::anyhow!(
                                    "malformed find output line (incorrect file type): {line}"
                                ))
                            }
                        }
                    } else {
                        return Err(anyhow::anyhow!("malformed find output line: {line}"));
                    }
                } else {
                    return Err(anyhow::anyhow!("malformed find output line: {line}"));
                }
            }
            Ok(DirectoryScanList { directories, files })
        } else {
            let stdout = String::from_utf8(output.stdout)
                .unwrap_or_default()
                .trim()
                .to_string();
            let stderr = String::from_utf8(output.stderr)
                .unwrap_or_default()
                .trim()
                .to_string();
            let reason = match (!stdout.is_empty(), !stderr.is_empty()) {
                (true, true) => format!("{stdout} {stderr}"),
                (true, false) => stdout,
                (false, true) => stderr,
                (false, false) => "<command has no output>".to_string(),
            };
            Err(anyhow::anyhow!(
                "remote command failed with status code {}: {}",
                output
                    .status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "<no status code>".to_string()),
                reason,
            ))
        }
    }

    /// Create a filtered version of the directory scan list that only contains items matched by
    /// `git` (with `gitignore` rules applied).
    ///
    /// # Arguments
    ///
    /// * `local_dir` - Path of local git directory.
    pub fn filter_by_gitignore(
        &mut self,
        local_dir: &std::path::Path,
    ) -> Result<DirectoryScanList> {
        let mut git_check_ignore_process = std::process::Command::new("git")
            .args([
                // Execute from local directory context.
                "-C",
                &local_dir.to_slash_lossy(),
                // Git subcommand to check gitignore matching.
                "check-ignore",
                // By default `check-ignore` only returns the paths of ignored files. We also want
                // to see any paths that were matched.
                "--non-matching",
                // Take input via stdin.
                "--stdin",
                // Include some extra information such as the line that actually matched. We use
                // this to figure out if git included or excluded the file.
                "--verbose",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("failed to spawn git command")?;

        fn is_output_line_match(git_check_ignore_line: &str) -> Result<bool> {
            let mut output_cols = git_check_ignore_line.split('\t');
            let header = output_cols.next().ok_or(anyhow::anyhow!(
                "git check-ignore output missing first part"
            ))?;
            let mut header_cols = header.split(':');
            let _source = header_cols
                .next()
                .ok_or(anyhow::anyhow!("git check-ignore output missing source"))?;
            let _linenum = header_cols
                .next()
                .ok_or(anyhow::anyhow!("git check-ignore output missing linenum"))?;
            let pattern = header_cols
                .next()
                .ok_or(anyhow::anyhow!("git check-ignore output missing pattern"))?;
            let path = output_cols
                .next()
                .ok_or(anyhow::anyhow!(
                    "git check-ignore output missing second part"
                ))?
                .trim();
            let is_git_dir = path == ".git" || path.starts_with(".git/");
            Ok((pattern.is_empty() || pattern.starts_with('!')) && !is_git_dir)
        }

        let git_check_ignore_stdin = git_check_ignore_process.stdin.as_mut().unwrap();
        let mut git_check_ignore_stdout =
            std::io::BufReader::new(git_check_ignore_process.stdout.take().unwrap());

        let mut matched_directories = Vec::new();
        for directory in &self.directories {
            writeln!(
                git_check_ignore_stdin,
                "{}",
                &directory.path.to_slash_lossy()
            )
            .context("failed to write to git check-ignore")?;
            let mut output_line = String::new();
            git_check_ignore_stdout.read_line(&mut output_line)?;
            if is_output_line_match(&output_line)? {
                matched_directories.push(directory.clone());
            }
        }

        let mut matched_files = Vec::new();
        for file in &self.files {
            writeln!(git_check_ignore_stdin, "{}", &file.path.to_slash_lossy())
                .context("failed to write to git check-ignore")?;
            let mut output_line = String::new();
            git_check_ignore_stdout.read_line(&mut output_line)?;
            if is_output_line_match(&output_line)? {
                matched_files.push(file.clone());
            }
        }

        let exit_status = git_check_ignore_process
            .wait()
            .context("failed to run git command")?;
        match exit_status.code() {
            // XXX: `git-check-ignore` returns 1 sometimes as part of normal operation
            Some(0 | 1) => Ok(DirectoryScanList {
                directories: matched_directories,
                files: matched_files,
            }),
            _ => Err(anyhow::anyhow!("git check-ignore failed: {exit_status}")),
        }
    }

    pub fn directories(&self) -> &[Directory] {
        &self.directories
    }

    pub fn files(&self) -> &[File] {
        &self.files
    }

    pub fn into_parts(self) -> (Vec<Directory>, Vec<File>) {
        (self.directories, self.files)
    }
}
