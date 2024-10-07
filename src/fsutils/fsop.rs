use std::{fs, io, path::Path};

#[derive(Debug)]
pub enum FsOp<'a> {
    CreateFile { path: &'a str },
    MoveFile { path: String }, // currently unused
    CopyFile { src: &'a str, dst: &'a str },
    RemoveFile { path: String },
}

pub fn apply(ops: &Vec<FsOp>) -> io::Result<()> {
    for op in ops {
        match op {
            FsOp::CreateFile { path } => {
                let path = Path::new(path);
                if path.exists() {
                    panic!("path {} exists", path.display());
                }
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(&path)?;
            }
            FsOp::MoveFile { path } => todo!(),
            FsOp::CopyFile { src, dst } => {
                let src = Path::new(src);
                let dst = Path::new(dst);
                if !src.exists() {
                    panic!("path {} does not exist", src.display());
                }
                if dst.exists() {
                    panic!("destination path {} already exists", dst.display());
                }
                if let Some(dst_parent) = dst.parent() {
                    fs::create_dir_all(dst_parent)?;
                }
                fs::copy(src, dst)?;
            }
            FsOp::RemoveFile { path } => todo!(),
        }
    }
    Ok(())
}
