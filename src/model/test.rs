use {
    crate::{self as mysql_connector, macros::*},
    chrono::{Duration, NaiveDateTime},
};

#[derive(Debug)]
struct Vector3 {
    x: f32,
    y: f32,
    z: f32,
}

#[allow(dead_code)]
#[derive(Debug, ModelData, FromQueryResult, ActiveModel, Model)]
#[mysql_connector(table="entity", primary="id", ty=Vector3{x,y,z})]
struct Entity {
    id: u32,
    name: String,
    position: Vector3,
    heading: f32,
    model: u32,
    spawn_after: Duration,
    map: Map,
}

#[allow(dead_code)]
#[derive(Debug, ModelData, FromQueryResult, ActiveModel, Model)]
#[mysql_connector(table = "map", primary = "id")]
struct Map {
    id: u32,
    name: String,
    difficulty: u8,
    last_played: Option<NaiveDateTime>,
}
