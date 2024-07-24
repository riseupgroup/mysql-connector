use mysql_connector::{
    pool::AsyncPoolTrait, types::Value, Connection, Migration, MigrationList, ResultSet, Version,
};

pub(super) const MIGRATIONS: MigrationList = MigrationList {
    version: Version(0, 1, 1),
    migrations: &[&UserAddPrimaryAddress],
};

struct UserAddPrimaryAddress;

impl Migration for UserAddPrimaryAddress {
    fn name(&self) -> &'static str {
        "UserAddPrimaryAddress"
    }

    fn up<'a>(
        &'a self,
        pool: &'a dyn AsyncPoolTrait<Connection>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), mysql_connector::Error>> + 'a>>
    {
        Box::pin(async {
            let mut conn1 = pool.get().await?;
            conn1.execute_query(
                "ALTER TABLE `user`
                    ADD COLUMN `primary_address` INT UNSIGNED NULL,
                    ADD CONSTRAINT `user-primary_address` FOREIGN KEY (`primary_address`) REFERENCES `address`(`id`)"
            ).await?;

            let mut conn2 = pool.get().await?;
            let mut conn3 = pool.get().await?;

            let mut users: ResultSet<_, Vec<Value>> =
                conn1.query("SELECT `id` FROM `user`").await?;
            let mut addresses_stmt = conn2
                .prepare_statement("SELECT `id` FROM `address` WHERE `user` = ? LIMIT 2")
                .await?;
            let mut set_address_stmt = conn3
                .prepare_statement("UPDATE `user` SET `primary_address` = ? WHERE `id` = ?")
                .await?;

            while let Some(user) = users.next().await? {
                let mut addresses: ResultSet<_, Vec<Value>> = addresses_stmt.query(&user).await?;
                if let Some(address) = addresses.next().await? {
                    if addresses.one().await?.is_none() && user.len() == 1 && address.len() == 1 {
                        set_address_stmt.execute(&[&address[0], &user[0]]).await?;
                    }
                }
            }
            Ok(())
        })
    }

    fn down<'a>(
        &self,
        pool: &'a dyn AsyncPoolTrait<Connection>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), mysql_connector::Error>> + 'a>>
    {
        Box::pin(async {
            pool.get()
                .await?
                .execute_query(
                    "ALTER TABLE `user`
                    DROP CONSTRAINT `user-primary_address`,
                    DROP COLUMN `primary_address`",
                )
                .await
                .map(|_| {})
        })
    }
}
