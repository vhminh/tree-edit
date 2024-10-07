pub mod fsop;
pub mod tmpfile;

use std::{fs, io, path};

pub fn get_paths_recursively(p: &path::PathBuf) -> io::Result<Vec<path::PathBuf>> {
    let mut result: Vec<path::PathBuf> = Vec::new();
    fn process(p: &path::PathBuf, result: &mut Vec<path::PathBuf>) -> io::Result<()> {
        let entries = fs::read_dir(p)?;
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                process(&path, result)?;
            } else {
                result.push(path);
            }
        }
        Ok(())
    }
    process(p, &mut result)?;
    Ok(result)
}
