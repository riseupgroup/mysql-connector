# mysql-connector

Simple MySQL connector for Rust that allows exchanging the underlying connection.

## Features

* `tcpstream` (enabled by default): implements the `Socket` trait for tokio's `TcpStream`.
* `serde`: implements `serde::Serialize` and `serde::Deserialize` for some types.

## Example

```rust
use std::sync::Arc;

use mysql_connector::{
    macros::*, model::*, types::AuthPlugin, Connection, ConnectionOptions, TcpStream,
};

#[derive(Debug, ModelData, FromQueryResult, ActiveModel, IntoQuery, Model)]
#[mysql_connector(table = "user", primary = "id", auto_increment = "true")]
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

    User {
        id: 0,
        name: String::from("foo"),
        email: Some(String::from("foo@example.com")),
    }
    .into_active_model()
    .insert(&mut conn)
    .await
    .unwrap();

    User {
        id: 0,
        name: String::from("bar"),
        email: None,
    }
    .into_active_model()
    .insert(&mut conn)
    .await
    .unwrap();

    let users: Vec<User> = conn
        .query(&User::build_query())
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    println!("{users:?}");

    conn.disconnect().await.unwrap();
}
```
