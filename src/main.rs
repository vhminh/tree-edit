use std::{
    env, fs, io, path,
    process::{self, ExitStatus},
};

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

#[derive(Debug)]
struct Entry {
    id: u64,
    path: String,
}

impl Entry {
    fn new(id: u64, path: String) -> Self {
        Entry { id, path }
    }
}

struct TmpFile {
    path: path::PathBuf,
    _private: (), // please call TmpFile::new instead of initializing this directly
}

impl TmpFile {
    fn new(filename: &str, ext: &str) -> io::Result<TmpFile> {
        let mut path = env::temp_dir();
        path.push(filename);
        path.set_extension(ext);
        let _ = fs::File::create(&path)?;
        Ok(TmpFile { path, _private: () })
    }
}

impl Drop for TmpFile {
    fn drop(&mut self) {
        fs::remove_file(&self.path).unwrap();
    }
}

fn get_tmp_file_name() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect()
}

fn user_edit_entries(entries: &Vec<Entry>) -> io::Result<Vec<Entry>> {
    let tmp_file = TmpFile::new(&get_tmp_file_name(), "txt")?;
    fs::write(&tmp_file.path, entries_to_str(&entries))?;
    eprintln!("opening file {} in {}", tmp_file.path.display(), "nvim");
    let exit_code = process::Command::new("nvim")
        .arg(tmp_file.path.as_os_str())
        .spawn()?
        .wait()?; // TODO: do something with status code
    eprintln!("editor exit with {}", exit_code);
    Ok(Vec::new())
}

fn digit_count(val: u64) -> u32 {
    if val == 0 {
        1
    } else {
        val.ilog10() + 1
    }
}

fn entries_to_str(entries: &Vec<Entry>) -> String {
    let max_id = entries.iter().map(|e| e.id).max();
    match max_id {
        Some(max_id) => {
            let id_col_len = digit_count(max_id) as usize;
            entries
                .iter()
                .map(|e| format!("{:<id_col_len$} {}", e.id, e.path))
                .collect::<Vec<String>>()
                .join("\n")
        }
        None => String::from(""),
    }
}

fn str_to_entry(s: &str) -> Entry {
    todo!()
}

fn main() -> io::Result<()> {
    let paths = get_paths_recursively(&env::current_dir()?)?;
    let paths: Vec<String> = paths
        .iter()
        .map(|p| String::from(p.to_string_lossy()))
        .collect();
    let entries: Vec<Entry> = paths
        .into_iter()
        .enumerate()
        .map(|tuple| Entry::new(tuple.0 as u64, tuple.1))
        .collect();
    let new_entries = user_edit_entries(&entries);
    Ok(())
}
