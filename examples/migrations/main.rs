use std::sync::Arc;

use mysql_connector::*;
use pool::AsyncPool;

mod migrations;

lazy_static::lazy_static! {
    static ref POOL: AsyncPool<Connection, TcpStream, 10> = AsyncPool::new(Arc::new(ConnectionOptions::<TcpStream> {
        user: std::env::var("DB_USER").unwrap(),
        password: std::env::var("DB_PASSWORD").unwrap(),
        db_name: Some("migrations".into()),
        connection: TcpStreamOptions {
            host: std::env::var("DB_HOST").unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }));
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    let mut migrator = Migrator::new(&*POOL, migrations::MIGRATION_LISTS)
        .await
        .unwrap();
    migrator.to_version(Version(0, 0, 0)).await.unwrap(); // undo all migrations for testing
    migrator.up().await.unwrap();
}
