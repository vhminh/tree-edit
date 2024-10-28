use std::collections::HashMap;

use rand::{rngs::StdRng, RngCore, SeedableRng};
use tree_edit::{entry::Entry, fsutils::fsop::FsOp};

fn rand_exp_size(rng: &mut dyn RngCore) -> u64 {
    match rng.next_u64() % 9 {
        0 => 0,
        v => 1 << (v - 1),
    }
}

fn generate_old_entries(rng: &mut dyn RngCore) -> Vec<Entry> {
    let size = rand_exp_size(rng);
    (0..size)
        .into_iter()
        .map(|i| Entry::new(Some(i), format!("{i}.txt")))
        .collect()
}

fn generate_new_entries(old_entries_len: u64, rng: &mut dyn RngCore) -> Vec<Entry> {
    let size = rand_exp_size(rng);
    let new_file_percentage = if old_entries_len == 0 {
        100
    } else {
        rng.next_u64() % 101
    };
    (0..size)
        .into_iter()
        .map(|i| {
            let v = rng.next_u64() % 100;
            let id = if v < new_file_percentage {
                None
            } else {
                Some(rng.next_u64() % old_entries_len)
            };
            Entry::new(id, format!("{i}.txt"))
        })
        .collect()
}

fn sort(entries: &mut Vec<Entry>) {
    entries.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));
}

fn apply(entries: &Vec<Entry>, ops: &Vec<FsOp<'_>>) -> Vec<Entry> {
    let mut fs = HashMap::<String, Option<u64>>::new();
    for entry in entries {
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
    fs.into_iter()
        .map(|(path, maybe_id)| -> Entry { Entry::new(maybe_id, path) })
        .collect::<Vec<_>>()
}

fn run(seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    let old_entries = generate_old_entries(&mut rng);
    let new_entries = generate_new_entries(old_entries.len().try_into().unwrap(), &mut rng);
    let ops = tree_edit::diff(&old_entries, &new_entries).unwrap();
    let entries_after_apply = apply(&old_entries, &ops);
    let mut new_entries = new_entries;
    let mut entries_after_apply = entries_after_apply;
    sort(&mut new_entries);
    sort(&mut entries_after_apply);

    assert_eq!(entries_after_apply, new_entries);
}

fn main() {
    println!("running fuzz testing");
    for i in 1..64200 {
        run(i);
    }
    println!("completed!")
}
