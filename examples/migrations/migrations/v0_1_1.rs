use mysql_connector::{Migration, MigrationList, Version};

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
        &self,
        conn: &'a mut mysql_connector::Connection,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), mysql_connector::Error>> + 'a>>
    {
        Box::pin(async {
            conn.execute_query(
                "ALTER TABLE `user`
                    ADD COLUMN `primary_address` INT UNSIGNED NULL,
                    ADD CONSTRAINT `user-primary_address` FOREIGN KEY (`primary_address`) REFERENCES `address`(`id`)"
            ).await?;
            conn.execute_query(
                "UPDATE `user` SET `primary_address` = (SELECT `id` FROM `address` WHERE `user` = `user`.`id`)
                WHERE (SELECT COUNT(*) FROM `address` WHERE `user` = `user`.`id`) = 1"
            ).await?;
            Ok(())
        })
    }

    fn down<'a>(
        &self,
        conn: &'a mut mysql_connector::Connection,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), mysql_connector::Error>> + 'a>>
    {
        Box::pin(async {
            conn.execute_query(
                "ALTER TABLE `user`
                    DROP CONSTRAINT `user-primary_address`,
                    DROP COLUMN `primary_address`",
            )
            .await
            .map(|_| {})
        })
    }
}
