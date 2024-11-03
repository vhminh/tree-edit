use clap::Parser;
use std::env;
use std::path::{absolute, PathBuf};

#[derive(Parser)]
#[command(version, about = "Edit file tree in your editor")]
struct CliArg {
    /// Directory to operate on, default to current working directory
    dir: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args: CliArg = CliArg::parse();
    let dir = match args.dir {
        Some(dir) => absolute(dir)?,
        None => absolute(env::current_dir()?)?,
    };
    tree_edit::tree_edit(&dir)?;
    Ok(())
}
