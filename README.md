# Command-Line Help for `gitrw`

This document contains the help content for the `gitrw` command-line program.

**Command Overview:**

* [`gitrw`↴](#gitrw)
* [`gitrw contributor`↴](#gitrw-contributor)
* [`gitrw contributor list`↴](#gitrw-contributor-list)
* [`gitrw contributor rewrite`↴](#gitrw-contributor-rewrite)
* [`gitrw remove`↴](#gitrw-remove)
* [`gitrw prune-empty`↴](#gitrw-prune-empty)

## `gitrw`

CLI tool for reading and rewriting history information of a git repository

**Usage:** `gitrw [OPTIONS] [REPOSITORY] <COMMAND>`

###### **Subcommands:**

* `contributor` — Contributor related actions like list and rewrite
* `remove` — Remove files and whole directories from the repository
* `prune-empty` — Remove empty commits that are no merge commits

###### **Arguments:**

* `<REPOSITORY>` — Path to the mirrored/bare repository (do not use on a repository with a working copy)

###### **Options:**

* `-d`, `--dry-run` — Do not change the repository

  Possible values: `true`, `false`




## `gitrw contributor`

Contributor related actions like list and rewrite

**Usage:** `gitrw contributor <COMMAND>`

###### **Subcommands:**

* `list` — Lists all authors and committers
* `rewrite` — Allows to rewrite contributors



## `gitrw contributor list`

Lists all authors and committers

**Usage:** `gitrw contributor list`



## `gitrw contributor rewrite`

Allows to rewrite contributors

**Usage:** `gitrw contributor rewrite <MAPPING_FILE>`

###### **Arguments:**

* `<MAPPING_FILE>` — Format inside file: Old User <old@user.mail> = New User <new@user.mail>



## `gitrw remove`

Remove files and whole directories from the repository

**Usage:** `gitrw remove <--file <FILE>|--directory <DIRECTORY>>`

###### **Options:**

* `-f`, `--file <FILE>` — File to remove. Argument can be specified multiple times
* `-d`, `--directory <DIRECTORY>` — Directory to remove. Argument can be specified multiple times



## `gitrw prune-empty`

Remove empty commits that are no merge commits

**Usage:** `gitrw prune-empty`


