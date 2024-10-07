use std::{env, fs, io, path::PathBuf};

use rand::{distributions::Alphanumeric, Rng};

pub struct TmpFile {
    path: PathBuf,
}

impl TmpFile {
    pub fn new(filename: &str, ext: &str) -> io::Result<TmpFile> {
        let mut path = env::temp_dir();
        path.push(filename);
        path.set_extension(ext);
        let _ = fs::File::create(&path)?;
        Ok(TmpFile { path })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for TmpFile {
    fn drop(&mut self) {
        fs::remove_file(&self.path).unwrap();
    }
}

pub fn get_tmp_file_name() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect()
}
