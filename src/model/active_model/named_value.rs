use crate::{
    error::SerializeError,
    types::{SimpleValue, Value},
};

#[derive(Debug)]
pub struct NamedValue(pub &'static str, pub Value);

impl NamedValue {
    pub fn name(&self) -> &'static str {
        self.0
    }

    pub fn value(&self) -> &Value {
        &self.1
    }

    pub(super) fn into_insert(
        values: &[NamedValue],
        table: &str,
    ) -> Result<String, SerializeError> {
        let mut stmt = String::from("insert into `");
        stmt += table;
        stmt += "` (";
        for (i, value) in values.iter().enumerate() {
            if i != 0 {
                stmt += ", ";
            }
            stmt += "`";
            stmt += value.name();
            stmt += "`";
        }
        stmt += ") values (";
        for i in 0..values.len() {
            if i != 0 {
                stmt += ", ";
            }
            stmt += "?";
        }
        stmt += ")";
        Ok(stmt)
    }

    pub(super) fn into_update(
        values: &[NamedValue],
        table: &str,
        primary: &str,
    ) -> Result<String, SerializeError> {
        let mut stmt = String::from("update `");
        stmt += table;
        stmt += "` set ";
        for (i, value) in values.iter().enumerate() {
            if i != 0 {
                stmt += ", ";
            }
            stmt += "`";
            stmt += value.name();
            stmt += "` = ?";
        }
        stmt += " where `";
        stmt += primary;
        stmt += "` = ?";
        Ok(stmt)
    }
}

impl SimpleValue for NamedValue {
    fn value(&self) -> &Value {
        &self.1
    }
}
