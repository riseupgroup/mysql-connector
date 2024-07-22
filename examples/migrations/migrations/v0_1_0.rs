use mysql_connector::{simple_migration, MigrationList, Version};

pub(super) const MIGRATIONS: MigrationList = MigrationList {
    version: Version(0, 1, 0),
    migrations: &[&CreateUser, &CreateAddress],
};

simple_migration! {
    CreateUser,
    "CREATE TABLE `user` (
        `id` INT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
        `name` VARCHAR(255) NOT NULL,
        `email` VARCHAR(255)
    )",
    "DROP TABLE `user`",
}

simple_migration! {
    CreateAddress,
    "CREATE TABLE `address` (
        `id` INT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
        `user` INT UNSIGNED NOT NULL,
        `street` VARCHAR(255) NOT NULL,
        `number` VARCHAR(10) NOT NULL,
        `postal_code` VARCHAR(10) NOT NULL,
        `country` CHAR(2) NOT NULL,
        CONSTRAINT `address-user` FOREIGN KEY (`user`) REFERENCES `user`(`id`)
    )",
    "DROP TABLE `address`",
}
