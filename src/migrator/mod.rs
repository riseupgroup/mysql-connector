mod migration;
mod migrator_inner;
mod model;

pub use {
    migration::{Migration, MigrationList, Version},
    migrator_inner::Migrator,
};

#[macro_export]
macro_rules! simple_migration {
    ($name:ident, $up:literal, $down:literal $(,)?) => {
        struct $name;

        impl mysql_connector::Migration for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn up<'a>(
                &'a self,
                pool: &'a dyn mysql_connector::pool::AsyncPoolTrait<mysql_connector::Connection>,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Result<(), mysql_connector::error::Error>>
                        + 'a,
                >,
            > {
                Box::pin(async { pool.get().await?.execute_query($up).await.map(|_| {}) })
            }

            fn down<'a>(
                &'a self,
                pool: &'a dyn mysql_connector::pool::AsyncPoolTrait<mysql_connector::Connection>,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Result<(), mysql_connector::error::Error>>
                        + 'a,
                >,
            > {
                Box::pin(async { pool.get().await?.execute_query($down).await.map(|_| {}) })
            }
        }
    };
}
