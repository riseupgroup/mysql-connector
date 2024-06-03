# mysql-connector

Simple MySQL connector for Rust that allows exchanging the underlying connection.

## Features

* `tcpstream` (enabled by default): implements the `Socket` trait for tokio's `TcpStream`.
* `serde`: implements `serde::Serialize` and `serde::Deserialize` for some types.

## Example

```rust
use std::sync::Arc;

use mysql_connector::{
    macros::{ActiveModel, FromQueryResult, Model, ModelData},
    types::AuthPlugin,
    Connection, ConnectionOptions, TcpStream,
};

#[allow(dead_code)]
#[derive(Debug, ModelData, FromQueryResult, ActiveModel, Model)]
#[mysql_connector(table = "user", primary = "id")]
pub struct User {
    id: u32,
    name: String,
    email: Option<String>,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    let mut conn = <Connection<TcpStream>>::connect(Arc::new(ConnectionOptions {
        user: "user".into(),
        password: std::env::var("PASSWORD").unwrap(),
        db_name: Some("db".into()),
        host: "localhost".into(),
        secure_auth: false,
        auth_plugin: Some(AuthPlugin::Native),
        ..Default::default()
    }))
    .await
    .unwrap();

    conn.execute_query(
        "CREATE TABLE `user` (
            `id` INT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
            `name` VARCHAR(255) NOT NULL,
            `email` VARCHAR(255)
        )",
    )
    .await
    .unwrap();

    conn.execute_query(
        "INSERT INTO `user` (`name`, `email`)
        VALUES ('foo', 'foo@example.com'),
        ('bar', NULL)",
    )
    .await
    .unwrap();

    let users: Vec<User> = conn
        .query("SELECT * from `user`")
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    println!("{users:?}");

    conn.disconnect().await.unwrap();
}
```
