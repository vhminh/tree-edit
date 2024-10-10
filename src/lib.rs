mod entry;
mod error;
mod fsutils;
mod ui;

use std::{
    collections::{HashMap, HashSet},
    convert::identity,
    env,
};

use entry::Entry;
use error::TreeEditError;
use fsutils::fsop::FsOp;

pub type Result<T> = std::result::Result<T, TreeEditError>;

pub fn tree_edit() -> Result<()> {
    let current_dir = env::current_dir()?;
    let paths = fsutils::get_paths_recursively(&current_dir)?;
    let paths: Vec<String> = paths
        .iter()
        .map(|p| String::from(p.to_string_lossy()))
        .collect();
    let entries: Vec<entry::Entry> = paths
        .into_iter()
        .enumerate()
        .map(|tuple| entry::Entry::new(Some(tuple.0 as u64), tuple.1))
        .collect();
    let new_entries = ui::user_edit_entries(&entries)?;
    let ops = diff(&entries, &new_entries)?;
    ui::display_ops(&ops);
    if ops.is_empty() {
        eprintln!("nothing to do")
    } else if ui::user_confirm()? {
        fsutils::fsop::exec_all(&ops)?;
        eprintln!("successfully applied {} operation(s)", ops.len())
    }
    Ok(())
}

fn diff<'a: 'b, 'b>(
    old_entries: &'a Vec<Entry>,
    new_entries: &'a Vec<Entry>,
) -> Result<Vec<FsOp<'b>>> {
    validate_old_entries(&old_entries);
    validate_new_entries(&new_entries)?;
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
        .map(|e| {
            let id = e.id.unwrap();
            let old_path = old_id_to_entries
                .get(&id)
                .ok_or(TreeEditError::InvalidFileId(id))?;
            if *old_path != e.path {
                Ok::<Option<FsOp<'_>>, TreeEditError>(Some(FsOp::CopyFile {
                    src: *old_path,
                    dst: &e.path,
                }))
            } else {
                Ok(None)
            }
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter_map(identity);
    let creates = new_entries
        .iter()
        .filter(|e| e.id.is_none())
        .map(|e| FsOp::CreateFile { path: &e.path });
    let mut ops = Vec::new();
    ops.append(&mut copies.collect());
    ops.append(&mut creates.collect());
    Ok(ops)
}

// all errors causes by our internal generated entries should panic
fn validate_old_entries(entries: &Vec<Entry>) {
    // all must have an id
    for entry in entries {
        assert!(entry.id.is_some(), "entry does not have an id {:?}", entry);
    }
    // ids must be unique
    let mut ids = HashSet::<u64>::new();
    for entry in entries {
        let id = entry.id.unwrap();
        assert!(!ids.contains(&id), "duplicate entry id {}", id);
        ids.insert(id);
    }
    // paths must be unique
    validate_unique_paths(entries).unwrap();
}

fn validate_new_entries(entries: &Vec<Entry>) -> Result<()> {
    validate_unique_paths(entries)?;
    Ok(())
}

fn validate_unique_paths(entries: &Vec<Entry>) -> Result<()> {
    let mut paths = HashSet::<&str>::new();
    for entry in entries {
        if paths.contains(&entry.path as &str) {
            return Err(TreeEditError::DuplicatePath(entry.path.clone()));
        }
        paths.insert(&entry.path);
    }
    Ok(())
}
