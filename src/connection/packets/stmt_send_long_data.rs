use {
    crate::{Command, Serialize},
    bytes::BufMut,
};

pub struct StmtSendLongData<'a> {
    stmt_id: u32,
    param_index: u16,
    data: &'a [u8],
}

impl<'a> StmtSendLongData<'a> {
    pub fn new(id: u32, param_index: u16, data: &'a [u8]) -> Self {
        Self {
            stmt_id: id,
            param_index,
            data,
        }
    }
}

impl Serialize for StmtSendLongData<'_> {
    fn serialize(&self, buf: &mut Vec<u8>) {
        (Command::StmtSendLongData as u8).serialize(buf);
        self.stmt_id.serialize(buf);
        self.param_index.serialize(buf);
        buf.put_slice(self.data);
    }
}
