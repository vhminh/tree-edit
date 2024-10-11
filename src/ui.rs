use std::{fs, io, process};

use crate::entry::Entry;
use crate::error::TreeEditError;
use crate::fsutils::fsop::FsOp;
use crate::fsutils::tmpfile;
use crate::fsutils::tmpfile::TmpFile;

pub fn user_edit_entries(entries: &Vec<Entry>) -> crate::Result<Vec<Entry>> {
    let tmp_file = TmpFile::new(&tmpfile::get_tmp_file_name(), "txt")?;
    fs::write(&tmp_file.path(), entries_to_str(&entries))?;
    eprintln!("opening file {} in {}", tmp_file.path().display(), "nvim");
    let exit_code = process::Command::new("nvim")
        .arg(tmp_file.path().as_os_str())
        .spawn()?
        .wait()?;
    if !exit_code.success() {
        return Err(TreeEditError::EditorExitFailure(exit_code));
    }
    let content = fs::read_to_string(&tmp_file.path())?;
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

pub fn display_ops(ops: &Vec<FsOp>) {
    for op in ops {
        match op {
            FsOp::CreateFile { path } => eprintln!("\x1b[32mCREATE {}\x1b[0m", path),
            FsOp::MoveFile { src, dst } => eprintln!("\x1b[33mMOVE   {} => {}\x1b[0m", src, dst),
            FsOp::CopyFile { src, dst } => eprintln!("\x1b[32mCOPY   {} => {}\x1b[0m", src, dst),
            FsOp::RemoveFile { path } => eprintln!("\x1b[31mREMOVE {}\x1b[0m", path),
        }
    }
}

pub fn user_confirm() -> io::Result<bool> {
    eprint!("do you want to proceed? [y/N] ");
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    let normalized = buffer.to_lowercase();
    let normalized = normalized.trim();
    Ok(normalized == "yes" || normalized == "y")
}
