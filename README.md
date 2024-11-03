# tree-edit

Edit file tree in your editor

## Demo

## Usage
Set `$VISUAL` environment variable in your shell to the editor executable:
- Neovim: `export VISUAL=nvim`
- Nano: `export VISUAL=nano`

### Syntax
```sh
$ tree-edit --help
Edit file tree in your editor

Usage: tree-edit [DIR]

Arguments:
  [DIR]  Directory to operate on, default to current working directory

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Example
```
$ VISUAL=nvim tree-edit .
```

## Install
### From source
```sh
> git clone git@github.com:vhminh/tree-edit.git
> cd tree-edit
> cargo install --bin tree-edit --path .
```

## Dev
Run unit test
```sh
$ cargo test
```
Run fuzz test
```sh
$ time cargo run --bin fuzz
```