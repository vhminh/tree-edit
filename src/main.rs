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
    id: Option<u64>,
    path: String,
}

impl Entry {
    fn new(id: Option<u64>, path: String) -> Self {
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
        .wait()?;
    eprintln!("editor exit with {}", exit_code);
    // TODO: do something with status code
    let content = fs::read_to_string(&tmp_file.path)?;
    Ok(str_to_entries(&content))
}

fn digit_count(val: u64) -> u32 {
    if val == 0 {
        1
    } else {
        val.ilog10() + 1
    }
}

fn entries_to_str(entries: &Vec<Entry>) -> String {
    let max_id = entries.iter().map(|e| e.id.unwrap()).max();
    match max_id {
        Some(max_id) => {
            let id_col_len = digit_count(max_id) as usize;
            entries
                .iter()
                .map(|e| format!("{:<id_col_len$} {}", e.id.unwrap(), e.path))
                .collect::<Vec<String>>()
                .join("\n")
        }
        None => String::from(""),
    }
}

fn str_to_entries(s: &str) -> Vec<Entry> {
    let lines = s.split("\n");
    lines
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|line| {
            let mut split = line.split_whitespace();
            let maybe_id_str = split.next();
            let maybe_id = maybe_id_str.and_then(|id_str| id_str.parse::<u64>().ok());
            let path = match maybe_id {
                Some(_) => &line[(maybe_id_str.unwrap().chars().count() + 1)..],
                None => line,
            }
            .trim();
            Entry::new(maybe_id, path.to_string())
        })
        .collect()
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
        .map(|tuple| Entry::new(Some(tuple.0 as u64), tuple.1))
        .collect();
    let new_entries = user_edit_entries(&entries);
    eprintln!("new entries {:?}", new_entries);
    Ok(())
}
