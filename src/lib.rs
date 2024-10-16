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
                    src: Cow::Borrowed(*old_path),
                    dst: Cow::Borrowed(&e.path),
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
        .map(|e| FsOp::CreateFile {
            path: Cow::Borrowed(&e.path),
        });
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
    fn test_copy_an_existing_file() -> Result<()> {
        diff_and_apply_ops(
            &vec![entry(1, "a.txt")],
            &vec![entry(1, "a.txt"), entry(1, "b.txt")],
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
