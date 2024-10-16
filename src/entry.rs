#[derive(Debug, Eq, PartialEq)]
pub struct Entry {
    pub id: Option<u64>,
    pub path: String,
}

impl Entry {
    pub fn new(id: Option<u64>, path: String) -> Self {
        Entry { id, path }
    }
}
