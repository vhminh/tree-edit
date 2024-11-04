pub mod fsop;
pub mod tmpfile;

use std::path::PathBuf;

use ignore;

pub fn get_paths_recursively(p: &PathBuf) -> Vec<PathBuf> {
    ignore::WalkBuilder::new(p)
        .hidden(true)
        .build()
        .into_iter()
        .filter_map(|result| match result {
            Ok(dir_entry) => Some(PathBuf::from(dir_entry.path())),
            Err(err) => {
                eprintln!("{err}");
                None
            }
        })
        .collect()
}
