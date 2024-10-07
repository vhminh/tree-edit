mod entry;
mod fsutils;
mod ui;

use std::{
    collections::{HashMap, HashSet},
    env, io,
};

use entry::Entry;
use fsutils::fsop::FsOp;

pub fn tree_edit() -> io::Result<()> {
    let paths = fsutils::get_paths_recursively(&env::current_dir()?)?;
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
    let ops = diff(&entries, &new_entries);
    ui::display_ops(&ops);
    if ui::user_confirm() {
        fsutils::fsop::apply(&ops)?;
    }
    Ok(())
}

/// assuming that all entries in `old_entries` has a unique id
// TODO: use `Result` to return user errors
fn diff<'a: 'b, 'b>(old_entries: &'a Vec<Entry>, new_entries: &'a Vec<Entry>) -> Vec<FsOp<'b>> {
    validate_old_entries(old_entries);
    validate_new_entries(new_entries);
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

// TODO: return user Error instead of panics
fn validate_old_entries(entries: &Vec<Entry>) {
    for entry in entries {
        assert!(entry.id.is_some());
    }
    validate_unique_paths(entries);
}

fn validate_new_entries(entries: &Vec<Entry>) {
    validate_unique_paths(entries);
}

fn validate_unique_paths(entries: &Vec<Entry>) {
    let mut paths = HashSet::<&str>::new();
    for entry in entries {
        if paths.contains(&entry.path as &str) {
            panic!("duplicate path {}", entry.path);
        }
        paths.insert(&entry.path);
    }
}
