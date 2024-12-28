# tree-edit

CLI tool to edit file system tree using a text editor, inspired by [oil.nvim](https://github.com/stevearc/oil.nvim)

- List all files in the directory recursively and write the list into a temporary file.

- Open the temporary file in the text editor (configured by the user with `$VISUAL` or `$EDITOR`).

- User edits the content of the buffer to reflect the desired state.

- Upon saving, determine the necessary file system operations (create/copy/move/delete) to transform the initial file tree into the new one.

- If the user confirms, apply those operations.

## Demo

Bulk create files

https://github.com/user-attachments/assets/e19072fc-40c1-4a01-a702-022bc5274001

Bulk rename files

https://github.com/user-attachments/assets/efa953d7-4279-40d4-8e58-3f4c5629e36c

Swap 2 files

https://github.com/user-attachments/assets/5ab5e6af-51a4-4b14-b6a4-bff5123976a3

## Usage
Set `$VISUAL` or `$EDITOR` environment variable to your editor of choice:
- Neovim: `export VISUAL=nvim`
- Nano: `export VISUAL=nano`

### Syntax
```console
$ tree-edit --help
Edit file system tree using a text editor

Usage: tree-edit [OPTIONS] [DIR]

Arguments:
  [DIR]  Directory to operate on, default to current working directory

Options:
      --no-git-ignore  When set, .gitignore will not be respected
      --hidden         Include hidden files
  -h, --help           Print help
  -V, --version        Print version
```

### Example
```console
$ VISUAL=nvim tree-edit .
```

## Install
### From source
```console
$ git clone git@github.com:vhminh/tree-edit.git
$ cd tree-edit
$ cargo install --bin tree-edit --path .
```

## Dev
Run unit test
```console
$ cargo test
```
Run fuzz test
```console
$ time cargo run --release --bin fuzz
```
