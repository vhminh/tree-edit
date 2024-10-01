use std::{env, fs, io, path, process::{self, ExitStatus}};

use rand::{distributions::Alphanumeric, Rng};

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

struct Entry {
    id: i64,
    path: String,
}

impl Entry {
    fn new(id: i64, path: String) -> Self {
        Entry { id, path }
    }
}

fn get_tmp_file_path() -> path::PathBuf {
    let name: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();
    let mut path = env::temp_dir();
    path.push(name);
    path.set_extension("txt");
    path
}

fn edit_entries(entries: &Vec<Entry>) -> io::Result<Vec<Entry>> {
    let tmp_file = get_tmp_file_path();
    let _ = fs::File::create(&tmp_file)?;
    // TODO: populate entries
    let exit_code = process::Command::new("vi")
        .arg(tmp_file)
        .spawn()?
        .wait()?; // TODO: do something with status code
    Ok(Vec::new())
}

fn main() -> io::Result<()> {
    let paths = get_paths_recursively(&env::current_dir()?)?;
    let paths: Vec<String> = paths
        .iter()
        .map(|p| String::from(p.to_string_lossy()))
        .collect();
    let entries: Vec<Entry> = paths.into_iter().map(|p| Entry::new(0, p)).collect();
    let new_entries = edit_entries(&entries);
    Ok(())
}
