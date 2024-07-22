use {
    super::{MigrationList, Version},
    crate::{error::Error, migrator::model::MigrationModel, types::Value, Connection},
    std::collections::HashMap,
};

pub struct Migrator<'a> {
    conn: &'a mut Connection,
    migrations: &'a [MigrationList],
    applied: HashMap<Version, Vec<String>>,
}

impl<'a> Migrator<'a> {
    pub async fn new(
        conn: &'a mut Connection,
        migrations: &'a [MigrationList],
    ) -> Result<Self, Error> {
        debug_assert!(MigrationList::ordered(migrations));

        let mut migrations_table = conn.query::<Vec<Value>>("select 1 from `information_schema`.`PARTITIONS` where `TABLE_NAME` = \"migrations\" and `TABLE_SCHEMA` = DATABASE()").await?;
        if migrations_table.collect().await?.is_empty() {
            conn.execute_query(
                "create table `migrations` (
                    `version_0` smallint unsigned not null,
                    `version_1` smallint unsigned not null,
                    `version_2` smallint unsigned not null,
                    `name` varchar(255) not null,
                    `applied_at` datetime not null default current_timestamp,
                    unique (`version_0`, `version_1`, `version_2`, `name`)
                )",
            )
            .await?;
        }

        let mut query = conn
            .query::<MigrationModel>(
                "select `version_0`, `version_1`, `version_2`, `name` from `migrations`",
            )
            .await?;
        let mut applied: HashMap<Version, Vec<String>> = HashMap::new();
        while let Some(row) = query.next().await? {
            let mut found = false;
            'outer: for migration_list in migrations {
                if migration_list.version == row.version {
                    for migration in migration_list.migrations {
                        if migration.name() == row.name {
                            found = true;
                            break 'outer;
                        }
                    }
                    break 'outer;
                }
            }
            if !found {
                panic!("unknown migration: {}: \"{}\"", row.version, row.name)
            }
            Self::insert_applied(&mut applied, row.version, row.name);
        }
        Ok(Self {
            conn,
            migrations,
            applied,
        })
    }

    pub async fn up(&mut self) -> Result<(), Error> {
        self.up_to_version(None).await
    }

    fn insert_applied(applied: &mut HashMap<Version, Vec<String>>, version: Version, name: String) {
        match applied.get_mut(&version) {
            Some(list) => list.push(name),
            None => {
                applied.insert(version, vec![name]);
            }
        };
    }

    pub fn get_applied<'b>(
        applied: &'b mut HashMap<Version, Vec<String>>,
        version: &Version,
        name: &str,
    ) -> Option<(&'b mut Vec<String>, usize)> {
        if let Some(migrations) = applied.get_mut(version) {
            return migrations
                .iter()
                .position(|x| x == name)
                .map(|pos| (migrations, pos));
        }
        None
    }

    pub async fn up_to_version(&mut self, version: Option<Version>) -> Result<(), Error> {
        for migration_list in self.migrations {
            match &version {
                Some(version) if migration_list.version > *version => (),
                _ => {
                    for migration in migration_list.migrations {
                        if Self::get_applied(
                            &mut self.applied,
                            &migration_list.version,
                            migration.name(),
                        )
                        .is_none()
                        {
                            migration.up(self.conn).await?;
                            Self::insert_applied(
                                &mut self.applied,
                                migration_list.version,
                                migration.name().to_owned(),
                            );
                            self.conn.execute_query(&format!("insert into `migrations` (`version_0`, `version_1`, `version_2`, `name`) values ({}, {}, {}, \"{}\")", migration_list.version.0, migration_list.version.1, migration_list.version.2, migration.name())).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn down_to_version(&mut self, version: Version) -> Result<(), Error> {
        for migration_list in self.migrations.iter().rev() {
            if migration_list.version > version {
                for migration in migration_list.migrations.iter().rev() {
                    if let Some((applied, index)) = Self::get_applied(
                        &mut self.applied,
                        &migration_list.version,
                        migration.name(),
                    ) {
                        migration.down(self.conn).await?;
                        applied.swap_remove(index);
                        self.conn.execute_query(&format!("delete from `migrations` where `version_0` = {} and `version_1` = {} and `version_2` = {} and `name` = \"{}\"", migration_list.version.0, migration_list.version.1, migration_list.version.2, migration.name())).await?;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn to_version(&mut self, version: Version) -> Result<(), Error> {
        self.up_to_version(Some(version)).await?;
        self.down_to_version(version).await
    }

    #[cfg(debug_assertions)]
    pub async fn one_down(&mut self) -> Result<bool, Error> {
        for migration_list in self.migrations.iter().rev() {
            for migration in migration_list.migrations.iter().rev() {
                if let Some((applied, index)) =
                    Self::get_applied(&mut self.applied, &migration_list.version, migration.name())
                {
                    migration.down(self.conn).await?;
                    applied.swap_remove(index);
                    self.conn.execute_query(&format!("delete from `migrations` where `version_0` = {} and `version_1` = {} and `version_2` = {} and `name` = \"{}\"", migration_list.version.0, migration_list.version.1, migration_list.version.2, migration.name())).await?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
