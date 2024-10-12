mod entry;
mod error;
mod fsutils;
mod ui;

use std::{
    borrow::Cow,
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
    let copy_rm_move_ops = move_files_around_ops(&old_entries, &new_entries)?;
    let create_ops = create_files_ops(new_entries);
    let mut ops = Vec::new();
    ops.append(&mut copy_rm_move_ops.collect());
    ops.append(&mut create_ops.collect());
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

fn gen_backup_path(path: &str, existing_names: &HashSet<&str>) -> String {
    // FIXME: this can have exponential runtime
    // if a lot of files has the same name as back up (rarely)
    for i in 0..(existing_names.len() + 4) {
        let tmp_path = if i == 0 {
            format!("{path}.backup")
        } else {
            format!("{path}.backup-{i}")
        };
        if !existing_names.contains(&tmp_path as &str) {
            return tmp_path;
        }
    }
    panic!("unreachable*")
}

fn move_files_around_ops<'a: 'b, 'b>(
    old_entries: &'a Vec<Entry>,
    new_entries: &'a Vec<Entry>,
) -> Result<impl Iterator<Item = FsOp<'b>>> {
    let old_id_to_paths = {
        let mut builder = HashMap::<u64, &str>::new();
        for entry in old_entries {
            builder.insert(entry.id.unwrap(), &entry.path);
        }
        builder
    };
    let old_path_to_id = {
        let mut builder = HashMap::<&str, u64>::new();
        for entry in old_entries {
            builder.insert(&entry.path, entry.id.unwrap());
        }
        builder
    };
    let new_id_to_path = {
        let mut builder = HashMap::<u64, Vec<&str>>::new();
        for entry in new_entries {
            if let Some(id) = entry.id {
                let v = builder.entry(id).or_insert(Vec::new());
                v.push(&entry.path);
            }
        }
        builder
    };
    let new_path_to_id = {
        let mut builder = HashMap::<&str, Option<u64>>::new();
        for entry in new_entries {
            builder.insert(&entry.path, entry.id);
        }
        builder
    };
    let mut existing_names: HashSet<&str> = old_path_to_id.keys().cloned().collect();
    let mut ops = Vec::<FsOp>::new();
    let mut locked = HashSet::<u64>::new();
    let mut process = |id: u64| -> () {
        let old_path = old_id_to_paths.get(&id).unwrap();
        if locked.contains(&id) {
            let backup_path = gen_backup_path(old_path, &existing_names);
            assert!(existing_names.remove(*old_path));
            assert!(existing_names.insert(&backup_path));
            ops.push(FsOp::CopyFile {
                src: Cow::Borrowed(old_path),
                dst: Cow::Owned(backup_path),
            });
        }
        locked.insert(id);
        // copy to new entries
        let need_copy_to = new_id_to_path.get(&id).unwrap_or(&Vec::new());

        locked.remove(&id);
    };
    for id in old_id_to_paths.keys() {
        process(*id);
    }
    let ops = new_entries
        .iter()
        .filter(|e| e.id.is_some())
        .map(|e| {
            let id = e.id.unwrap();
            let old_path = old_id_to_paths
                .get(&id)
                .ok_or(TreeEditError::InvalidFileId(id))?;
            if *old_path != e.path {
                Ok::<Option<FsOp<'_>>, TreeEditError>(Some(FsOp::CopyFile {
                    src: Cow::Borrowed(old_path),
                    dst: Cow::Borrowed(&e.path),
                }))
            } else {
                Ok(None)
            }
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter_map(identity);
    Ok(ops)
}

fn create_files_ops<'a: 'b, 'b>(new_entries: &'a Vec<Entry>) -> impl Iterator<Item = FsOp<'b>> {
    new_entries
        .iter()
        .filter(|e| e.id.is_none())
        .map(|e| FsOp::CreateFile {
            path: Cow::Borrowed(&e.path),
        })
}
