use mysql_connector::MigrationList;

mod v0_1_0;
mod v0_1_1;

pub(super) const MIGRATION_LISTS: &[MigrationList] = &[v0_1_0::MIGRATIONS, v0_1_1::MIGRATIONS];
