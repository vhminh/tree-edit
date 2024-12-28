use clap::Parser;
use std::env;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Edit file system tree using a text editor")]
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

fn collect_files(respect_git_ignore: bool, ignore_hidden: bool) -> Vec<PathBuf> {
    ignore::WalkBuilder::new(".")
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
    if let Some(ref dir) = args.dir {
        // no need to reset, app exit right after anyway
        env::set_current_dir(dir)?;
    }
    let paths = collect_files(!args.no_git_ignore, !args.hidden);
    tree_edit::tree_edit(&paths)?;
    Ok(())
}
