use crate::{Command, Serialize};

#[derive(Debug)]
pub struct StmtClose {
    stmt_id: u32,
}

impl StmtClose {
    #[allow(dead_code)]
    pub fn new(stmt_id: u32) -> Self {
        Self { stmt_id }
    }
}

impl Serialize for StmtClose {
    fn serialize(&self, buf: &mut Vec<u8>) {
        (Command::StmtClose as u8).serialize(buf);
        self.stmt_id.serialize(buf);
    }
}
