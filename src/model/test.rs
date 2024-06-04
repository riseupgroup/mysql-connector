use {
    crate::{self as mysql_connector, macros::*, model::IntoQuery},
    chrono::{Duration, NaiveDateTime},
};

#[derive(Debug)]
struct Vector3 {
    x: f32,
    y: f32,
    z: f32,
}

#[allow(dead_code)]
#[derive(Debug, ModelData, FromQueryResult, ActiveModel, IntoQuery, Model)]
#[mysql_connector(table="entity", primary="id", auto_increment="true", ty=Vector3{x,y,z})]
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
#[derive(Debug, ModelData, FromQueryResult, ActiveModel, IntoQuery, Model)]
#[mysql_connector(table="map", primary="id", auto_increment="true")]
struct Map {
    id: u32,
    name: String,
    difficulty: u8,
    last_played: Option<NaiveDateTime>,
}

#[test]
fn test_into_query() {
    assert_eq!(
        Map::build_query(),
        "select `map`.`id`, `map`.`name`, `map`.`difficulty`, `map`.`last_played` from `map`"
    );
    assert_eq!(
        Entity::build_query(),
        concat!(
            "select `entity`.`id`, `entity`.`name`, `entity`.`heading`, `entity`.`model`, `entity`.`spawn_after`, ",
            "`entity`.`position_x`, `entity`.`position_y`, `entity`.`position_z`, ",
            "`entity.map`.`id`, `entity.map`.`name`, `entity.map`.`difficulty`, `entity.map`.`last_played` ",
            "from `entity` join `map` as `entity.map` on (`entity`.`map` = `entity.map`.`id`)",
        ),
    );
}
