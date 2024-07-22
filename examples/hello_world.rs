use std::sync::Arc;

use mysql_connector::{macros::*, model::*, *};

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
    let mut conn = <Connection>::connect(Arc::new(ConnectionOptions::<TcpStream> {
        user: std::env::var("DB_USER").unwrap(),
        password: std::env::var("DB_PASSWORD").unwrap(),
        db_name: Some("hello_world".into()),
        connection: TcpStreamOptions {
            host: std::env::var("DB_HOST").unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }))
    .await
    .unwrap();

    conn.execute_query(
        "CREATE TEMPORARY TABLE `user` (
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
