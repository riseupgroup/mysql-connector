use super::ModelData;

macro_rules! append {
    ($str: expr; $($part:expr),* $(,)?) => {
        $(
            $str += $part;
        )*
    };
}

/// Data for column that contains a reference.
///
/// As foreign key this would look like: `foreign key (<column>) references <table>(<key>)`
pub struct QueryColumnReference {
    /// Column in which the reference is stored
    pub column: &'static str,
    /// Referenced table
    pub table: &'static str,
    /// Referenced key
    pub key: &'static str,
    /// Columns of referenced table
    pub columns: &'static [QueryColumn],
}

impl QueryColumnReference {
    /// Join the referenced table and append all columns to the select,
    /// which may include some more joins.
    fn join(&self, namespace: &str, select: &mut String, join: &mut String) {
        let new_namespace = namespace.to_owned() + "." + self.column;
        append!(*join; " join `", self.table, "` as `", &new_namespace, "` on (`", namespace, "`.`", self.column, "` = `", &new_namespace, "`.`", self.key, "`)");

        for (i, column) in self.columns.iter().enumerate() {
            if i != 0 {
                *select += ", ";
            }
            column.append_to_select(&new_namespace, select, join);
        }
    }
}

pub enum QueryColumn {
    Column(&'static str),
    Reference(QueryColumnReference),
}

impl QueryColumn {
    fn append_to_select(&self, namespace: &str, select: &mut String, join: &mut String) {
        match self {
            Self::Column(column) => {
                append!(*select; "`", namespace, "`.`", *column, "`");
            }
            Self::Reference(r#struct) => {
                r#struct.join(namespace, select, join);
            }
        }
    }
}

pub trait IntoQuery: ModelData {
    const COLUMNS: &'static [QueryColumn];

    fn build_query() -> String {
        let mut select = String::from("select ");
        let mut join = String::new();
        for (i, column) in Self::COLUMNS.iter().enumerate() {
            if i != 0 {
                select += ", ";
            }
            column.append_to_select(Self::TABLE, &mut select, &mut join);
        }
        append!(select; " from `", Self::TABLE, "`");
        select += &join;
        select
    }
}
