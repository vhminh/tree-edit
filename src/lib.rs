mod entry;
mod error;
mod fsutils;
mod ui;

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
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
    let allowed_ids = old_entries
        .iter()
        .map(|e| e.id.unwrap())
        .collect::<HashSet<u64>>();
    validate_new_entries(&new_entries, &allowed_ids)?;
    let copy_rm_move_ops = move_files_around_ops(&old_entries, &new_entries);
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

fn validate_new_entries(entries: &Vec<Entry>, allowed_ids: &HashSet<u64>) -> Result<()> {
    for entry in entries {
        if let Some(id) = entry.id {
            if !allowed_ids.contains(&id) {
                return Err(TreeEditError::InvalidFileId(id));
            }
        }
    }
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

fn gen_backup_path(path: &str, existing_names: &HashSet<String>) -> String {
    // FIXME: this can have exponential runtime
    // if a lot of files has the same name as back up (rarely)
    for i in 0..(existing_names.len() + 4) {
        let tmp_path = if i == 0 {
            format!("{path}.backup")
        } else {
            format!("{path}.backup-{i}")
        };
        if !existing_names.contains(&tmp_path) {
            return tmp_path;
        }
    }
    panic!("unreachable*")
}

fn move_files_around_ops<'a: 'b, 'b>(
    old_entries: &'a Vec<Entry>,
    new_entries: &'a Vec<Entry>,
) -> impl Iterator<Item = FsOp<'b>> {
    struct Lookup<'a> {
        old_id_to_path: HashMap<u64, &'a str>,
        old_path_to_id: HashMap<&'a str, u64>,
        new_id_to_paths: HashMap<u64, Vec<&'a str>>,
    }
    let lookup = Lookup {
        old_id_to_path: {
            let mut builder = HashMap::<u64, &str>::new();
            for entry in old_entries {
                builder.insert(entry.id.unwrap(), &entry.path);
            }
            builder
        },
        old_path_to_id: {
            let mut builder = HashMap::<&str, u64>::new();
            for entry in old_entries {
                builder.insert(&entry.path, entry.id.unwrap());
            }
            builder
        },
        new_id_to_paths: {
            let mut builder = HashMap::<u64, Vec<&str>>::new();
            for entry in new_entries {
                if let Some(id) = entry.id {
                    let v = builder.entry(id).or_insert(Vec::new());
                    v.push(&entry.path);
                }
            }
            builder
        },
    };
    // FIXME: HashSet of Cow<'_, String> ???
    let mut existing_names: HashSet<String> = lookup
        .old_path_to_id
        .keys()
        .cloned()
        .map(String::from)
        .collect();
    let mut ops = Vec::<FsOp>::new();
    let mut locked = HashSet::<u64>::new();
    let mut dirty = HashMap::<u64, FsOp>::new();
    let mut processed = HashSet::<u64>::new();
    fn process<'a>(
        id: u64,
        existing_names: &mut HashSet<String>,
        ops: &mut Vec<FsOp<'a>>,
        processed: &mut HashSet<u64>,
        locked: &mut HashSet<u64>,
        dirty: &mut HashMap<u64, FsOp<'a>>,
        lookup: &Lookup<'a>,
    ) {
        if processed.contains(&id) {
            return;
        }
        let old_path = lookup.old_id_to_path.get(&id).unwrap();
        locked.insert(id);
        // copy to new entries
        let new_paths = Vec::new();
        let new_paths = lookup.new_id_to_paths.get(&id).unwrap_or(&new_paths);
        let keep_old_path = new_paths.contains(old_path);
        let mut new_path_iter = new_paths.iter().filter(|p| *p != old_path).peekable();
        while let Some(new_path) = new_path_iter.next() {
            // move file if we don't need to keep it at the original location
            // and this is the last file in the list
            let move_instead_of_copy = !keep_old_path && new_path_iter.peek().is_none();
            if let Some(existing_id_at_new_path) = lookup.old_path_to_id.get(*new_path) {
                if locked.contains(existing_id_at_new_path) {
                    // cycle detected, push to dirty list
                    let backup_path = gen_backup_path(old_path, &existing_names);
                    assert!(existing_names.insert(backup_path.clone()));
                    if move_instead_of_copy {
                        ops.push(FsOp::MoveFile {
                            src: Cow::Borrowed(old_path),
                            dst: Cow::Owned(backup_path.clone()),
                        });
                    } else {
                        ops.push(FsOp::CopyFile {
                            src: Cow::Borrowed(old_path),
                            dst: Cow::Owned(backup_path.clone()),
                        });
                    }
                    dirty.insert(
                        *existing_id_at_new_path,
                        FsOp::MoveFile {
                            src: Cow::Owned(backup_path),
                            dst: Cow::Borrowed(new_path),
                        },
                    );
                    continue;
                } else {
                    process(
                        *existing_id_at_new_path,
                        existing_names,
                        ops,
                        processed,
                        locked,
                        dirty,
                        lookup,
                    );
                }
            }
            existing_names.insert(new_path.to_string());
            if move_instead_of_copy {
                ops.push(FsOp::MoveFile {
                    src: Cow::Borrowed(old_path),
                    dst: Cow::Borrowed(new_path),
                });
            } else {
                ops.push(FsOp::CopyFile {
                    src: Cow::Borrowed(old_path),
                    dst: Cow::Borrowed(new_path),
                });
            }
        }
        if new_paths.is_empty() {
            ops.push(FsOp::RemoveFile {
                path: Cow::Borrowed(old_path),
            })
        }
        locked.remove(&id);
        // push remaining ops from dirty list
        if let Some(op) = dirty.remove(&id) {
            ops.push(op);
        }
        processed.insert(id);
    }
    for id in lookup.old_id_to_path.keys() {
        process(
            *id,
            &mut existing_names,
            &mut ops,
            &mut processed,
            &mut locked,
            &mut dirty,
            &lookup,
        );
    }
    assert!(dirty.is_empty());
    ops.into_iter()
}

fn create_files_ops<'a: 'b, 'b>(new_entries: &'a Vec<Entry>) -> impl Iterator<Item = FsOp<'b>> {
    new_entries
        .iter()
        .filter(|e| e.id.is_none())
        .map(|e| FsOp::CreateFile {
            path: Cow::Borrowed(&e.path),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_change() -> Result<()> {
        diff_and_apply_ops(
            &vec![Entry::new(Some(1), "a.txt".to_string())],
            &vec![Entry::new(Some(1), "a.txt".to_string())],
        )
    }

    #[test]
    fn test_create_1_new_file() -> Result<()> {
        diff_and_apply_ops(
            &vec![entry(1, "a.txt")],
            &vec![entry(1, "a.txt"), new_entry("b.txt")],
        )
    }

    #[test]
    fn test_remove_1_file() -> Result<()> {
        diff_and_apply_ops(&vec![entry(1, "a.txt")], &vec![])
    }

    #[test]
    fn test_copy_an_existing_file() -> Result<()> {
        diff_and_apply_ops(
            &vec![entry(1, "a.txt")],
            &vec![entry(1, "a.txt"), entry(1, "b.txt")],
        )
    }

    #[test]
    fn test_copy_an_existing_file_rev() -> Result<()> {
        diff_and_apply_ops(
            &vec![entry(2, "b.txt")],
            &vec![entry(2, "a.txt"), entry(2, "b.txt")],
        )
    }

    #[test]
    fn test_user_input_invalid_id() {
        let result = diff_and_apply_ops(
            &vec![entry(1, "a.txt")], // there was previously no entry with id 2
            &vec![entry(1, "a.txt"), entry(2, "b.txt")],
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, TreeEditError::InvalidFileId(2)))
    }

    #[test]
    fn test_copy_dependency_without_cycle() -> Result<()> {
        diff_and_apply_ops(
            &vec![entry(1, "a.txt"), entry(2, "b.txt")],
            &vec![entry(1, "a.txt"), entry(1, "b.txt"), entry(2, "c.txt")],
        )
    }

    #[test]
    fn test_copy_dependency_without_cycle_rev() -> Result<()> {
        diff_and_apply_ops(
            &vec![entry(2, "b.txt"), entry(3, "c.txt")],
            &vec![entry(2, "a.txt"), entry(3, "b.txt"), entry(3, "c.txt")],
        )
    }

    #[test]
    fn test_copy_dependency_with_cycle_of_2() -> Result<()> {
        diff_and_apply_ops(
            &vec![entry(1, "a.txt"), entry(2, "b.txt")],
            &vec![entry(2, "a.txt"), entry(1, "b.txt")],
        )
    }

    fn entry(id: u64, path: &str) -> Entry {
        Entry::new(Some(id), String::from(path))
    }

    fn new_entry(path: &str) -> Entry {
        Entry::new(None, String::from(path))
    }

    fn diff_and_apply_ops(old_entries: &Vec<Entry>, new_entries: &Vec<Entry>) -> Result<()> {
        let ops = diff(old_entries, new_entries)?;
        println!("old: {old_entries:?}");
        println!("new: {new_entries:?}");
        println!("ops: {ops:?}");
        let mut fs = HashMap::<String, Option<u64>>::new();
        for entry in old_entries {
            assert_eq!(fs.insert(entry.path.clone(), Some(entry.id.unwrap())), None);
        }
        for op in ops {
            match op {
                FsOp::CreateFile { path } => {
                    assert!(!fs.contains_key(path.as_ref()));
                    fs.insert(path.to_string(), None);
                }
                FsOp::MoveFile { src, dst } => {
                    assert!(fs.contains_key(src.as_ref()));
                    assert!(!fs.contains_key(dst.as_ref()));
                    let maybe_id = fs.remove(src.as_ref()).unwrap();
                    fs.insert(dst.to_string(), maybe_id);
                }
                FsOp::CopyFile { src, dst } => {
                    assert!(fs.contains_key(src.as_ref()));
                    assert!(!fs.contains_key(dst.as_ref()));
                    let maybe_id = fs.get(src.as_ref()).unwrap().clone();
                    fs.insert(dst.to_string(), maybe_id);
                }
                FsOp::RemoveFile { path } => {
                    assert!(fs.contains_key(path.as_ref()));
                    fs.remove(path.as_ref());
                }
            }
        }
        let mut entries_after_apply: Vec<_> = fs
            .into_iter()
            .map(|(path, maybe_id)| -> Entry { Entry::new(maybe_id, path) })
            .collect::<Vec<_>>();

        entries_after_apply.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));

        assert_eq!(&entries_after_apply, new_entries);
        Ok(())
    }
}
