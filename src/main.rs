use std::{env, fs, io, path};

fn get_paths_recursively(p: &path::PathBuf) -> io::Result<Vec<path::PathBuf>> {
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

fn main() -> io::Result<()> {
    let paths = get_paths_recursively(&env::current_dir()?)?;
    let paths: Vec<_> = paths.iter().map(|p| p.display()).collect();
    for path in paths {
        println!("Path {path}")
    }
    Ok(())
}
