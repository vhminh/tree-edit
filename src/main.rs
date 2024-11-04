use clap::Parser;
use std::env;
use std::path::{absolute, PathBuf};

#[derive(Parser)]
#[command(version, about = "Edit file tree in your editor")]
struct CliArg {
    /// Directory to operate on, default to current working directory
    dir: Option<PathBuf>,

    /// When set, .gitignore will not be respected.
    #[arg(long)]
    no_git_ignore: bool,

    /// Include hidden files
    #[arg(long)]
    hidden: bool,
}

fn collect_files(dir: &PathBuf, respect_git_ignore: bool, ignore_hidden: bool) -> Vec<PathBuf> {
    ignore::WalkBuilder::new(dir)
        .git_ignore(respect_git_ignore)
        .hidden(ignore_hidden)
        .build()
        .into_iter()
        .filter_map(|result| match result {
            Ok(dir_entry) => match dir_entry.file_type() {
                Some(file_type) if !file_type.is_dir() => Some(PathBuf::from(dir_entry.path())),
                _ => None,
            },
            Err(err) => {
                eprintln!("{err}");
                None
            }
        })
        .collect()
}

fn main() -> anyhow::Result<()> {
    let args: CliArg = CliArg::parse();
    let dir = match args.dir {
        Some(dir) => absolute(dir)?,
        None => absolute(env::current_dir()?)?,
    };
    let paths = collect_files(&dir, !args.no_git_ignore, !args.hidden);
    tree_edit::tree_edit(&paths)?;
    Ok(())
}
