use std::{
    collections::HashMap,
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

/// assuming that all entries in `old_entries` has a unique id
// TODO: use `Result` to return user errors
fn diff<'a: 'b, 'b>(old_entries: &'a Vec<Entry>, new_entries: &'a Vec<Entry>) -> Vec<FsOp<'b>> {
    let old_id_to_entries = {
        let mut builder = HashMap::<u64, &str>::new();
        for entry in old_entries {
            builder.insert(entry.id.unwrap(), &entry.path);
        }
        builder
    };
    let new_id_to_entries = {
        let mut builder = HashMap::<u64, Vec<&str>>::new();
        for entry in new_entries {
            if let Some(id) = entry.id {
                let v = builder.entry(id).or_insert(Vec::new());
                v.push(&entry.path);
            }
        }
        builder
    };
    let copies = new_entries
        .iter()
        .filter(|e| e.id.is_some())
        .filter_map(|e| {
            let id = e.id.unwrap();
            let old_path = old_id_to_entries.get(&id).unwrap(); // panics if id doesn't exist,
                                                                // TODO: return as user error
            if *old_path != e.path {
                Some(FsOp::CopyFile {
                    src: *old_path,
                    dst: &e.path,
                })
            } else {
                None
            }
        });
    let creates = new_entries
        .iter()
        .filter(|e| e.id.is_none())
        .map(|e| FsOp::CreateFile { path: &e.path });
    let mut ops = Vec::new();
    ops.append(&mut copies.collect());
    ops.append(&mut creates.collect());
    return ops;
}

#[derive(Debug)]
enum FsOp<'a> {
    CreateFile { path: &'a str },
    MoveFile { path: String }, // currently unused
    CopyFile { src: &'a str, dst: &'a str },
    RemoveFile { path: String },
}

fn apply(ops: &Vec<FsOp>) -> io::Result<()> {
    for op in ops {
        match op {
            FsOp::CreateFile { path } => {
                let path = path::Path::new(path);
                if path.exists() {
                    panic!("path {} exists", path.display());
                }
                fs::File::create(path)?;
            }
            FsOp::MoveFile { path } => todo!(),
            FsOp::CopyFile { src, dst } => {
                let src = path::Path::new(src);
                let dst = path::Path::new(dst);
                if !src.exists() {
                    panic!("path {} does not exist", src.display());
                }
                if dst.exists() {
                    panic!("destination path {} already exists", dst.display());
                }
                fs::copy(src, dst)?;
            }
            FsOp::RemoveFile { path } => todo!(),
        }
    }
    Ok(())
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
    let new_entries = user_edit_entries(&entries)?;
    let ops = diff(&entries, &new_entries);
    eprintln!("ops {:?}", ops);
    apply(&ops)?;
    Ok(())
}
