# `git-repo-sync`

Utility to copy code to and from remote hosts over SSH.

## ✨ What it does

`git-repo-sync` synchronizes code to and from a remote host over SSH:

* It respects `.gitignore`: Files that are ignored through the *local*
  `.gitignore` are never synchronized (in both directions).
* It only copies files that are different (based on file size).
* It **will** remove files if they were removed on the other end.

`git-repo-sync` fully supports Linux and macOS. Windows is supported only when
used as the "local" host, not as a remote.

## ✅ Requirements

* Git
* SSH with SFTP support (included by default)

## 📦 Install

Download the latest release from [GitHub releases](https://github.com/oddity-ai/git-repo-sync/releases) and place it anywhere in your path.

## ℹ️ Usage

> [!WARNING]  
> `git-repo-sync` is a tool to **synchronize** files: It **will** delete your
> files on the other end. Make sure to always do a dry run before running it to
> prevent losing files.

### ⬆️ Sync to remote host

To upload the current directory to a remote location:

```bash
git repo-sync up <remote-host>:<target-directory>
```

For example:

```bash
git repo-sync up user@server:/home/user/target-dir
```

If the host is already part of your SSH config:

```bash
git repo-sync up myserver:project
```

The above command will sync the current directory to the project directory on
the remote (in home).

### ⬇️ Sync from remote host to local

To sync the directory from a remote host to the local host:

```bash
git repo-sync down <remote-host>:<target-directory>
```

For example:

```bash
git repo-sync down myserver:project
```

The above command will sync the `project` directory contents back into the
current directory.

### Other options

To specify a different local directory (other than the current directory), use
the `--local-dir` option. For example:

```bash
git repo-sync --local-dir /tmp/project up user@remote:~/project
```

Other options:
* Use the `--verbose` flag to log all actions that have been taken.
* Use the `--dry` flag to **print** what `git-repo-sync` would do, without
  actually doing it.

> [!NOTE]
> All additional flags must be placed before the `up` or `down` command, or they
> will not be recognized.

## ⚖️ License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
