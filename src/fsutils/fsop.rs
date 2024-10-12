use std::{borrow::Cow, fs, path::Path};

use crate::error::{DetectedBy, TreeEditError};

#[derive(Debug)]
pub enum FsOp<'a> {
    CreateFile {
        path: Cow<'a, str>,
    },
    MoveFile {
        src: Cow<'a, str>,
        dst: Cow<'a, str>,
    },
    CopyFile {
        src: Cow<'a, str>,
        dst: Cow<'a, str>,
    },
    RemoveFile {
        path: Cow<'a, str>,
    },
}

pub fn exec(op: &FsOp) -> crate::Result<()> {
    match op {
        FsOp::CreateFile { path: path_str } => {
            let path = Path::new(path_str.as_ref());
            if path.exists() {
                return Err(TreeEditError::FsChanged(DetectedBy::FileExists(
                    path_str.to_string(),
                )));
            }
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(&path)?;
        }
        FsOp::MoveFile { src, dst } => todo!(),
        FsOp::CopyFile {
            src: src_str,
            dst: dst_str,
        } => {
            let src = Path::new(src_str.as_ref());
            let dst = Path::new(dst_str.as_ref());
            if !src.exists() {
                return Err(TreeEditError::FsChanged(DetectedBy::FileNotFound(
                    src_str.to_string(),
                )));
            }
            if dst.exists() {
                return Err(TreeEditError::FsChanged(DetectedBy::FileExists(
                    dst_str.to_string(),
                )));
            }
            if let Some(dst_parent) = dst.parent() {
                fs::create_dir_all(dst_parent)?;
            }
            fs::copy(src, dst)?;
        }
        FsOp::RemoveFile { path } => todo!(),
    }
    Ok(())
}

pub fn exec_all(ops: &Vec<FsOp>) -> crate::Result<()> {
    for op in ops {
        exec(&op)?;
    }
    Ok(())
}
