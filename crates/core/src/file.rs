//! The native job file: the whole document as versioned, self-describing JSON.

use crate::document::Document;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const JOB_EXTENSION: &str = "ajob";
const FORMAT: &str = "aspire-clone-job";
/// Bump when the document's JSON shape changes, and teach `load_job` to read the old one.
const VERSION: u32 = 1;

#[derive(Serialize)]
struct Saved<'a> {
    format: &'static str,
    version: u32,
    document: &'a Document,
}

#[derive(Deserialize)]
struct Header {
    format: String,
    version: u32,
}

#[derive(Deserialize)]
struct Loaded {
    document: Document,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError {
    NotAJob,
    NewerVersion(u32),
    Damaged(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAJob => f.write_str("this is not an Aspire Clone job file"),
            Self::NewerVersion(v) => {
                write!(
                    f,
                    "this job was saved by a newer version (format {v}; this build reads up to {VERSION})"
                )
            }
            Self::Damaged(why) => write!(f, "the job file is damaged: {why}"),
        }
    }
}

impl std::error::Error for LoadError {}

pub fn save_job(doc: &Document) -> Vec<u8> {
    serde_json::to_vec_pretty(&Saved {
        format: FORMAT,
        version: VERSION,
        document: doc,
    })
    .expect("documents always serialize")
}

pub fn load_job(bytes: &[u8]) -> Result<Document, LoadError> {
    let header: Header = serde_json::from_slice(bytes).map_err(|_| LoadError::NotAJob)?;
    if header.format != FORMAT {
        return Err(LoadError::NotAJob);
    }
    if header.version > VERSION {
        return Err(LoadError::NewerVersion(header.version));
    }
    let loaded: Loaded =
        serde_json::from_slice(bytes).map_err(|e| LoadError::Damaged(e.to_string()))?;
    Ok(loaded.document)
}
