# `git-repo-sync`

## About

Utility to copy code to and from remote hosts over SSH.

## What it does

`git-repo-sync` synchronizes code to and from a remote host over SSH:

* It respects `.gitignore`: Files that are ignored through the *local*
  `.gitignore` are never synchronized (in both directions).
* It only copies files that are different (based on file size).
* It **will** remove files if they were removed on the other end.

`git-repo-sync` fully supports Linux and macOS. Windows is supported only when
used as the "local" host, not as a remote.

## Requirements

* Git
* SSH with SFTP support (included by default)

## Install

TODO

## Usage

### Sync to remote host

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

### Sync from remote host to local

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
