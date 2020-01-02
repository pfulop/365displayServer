use super::connection_enums::*;
use super::models::*;
use dynomite::{
    attr_map,
    dynamodb::{DynamoDb, DynamoDbClient, GetItemInput, ScanInput, UpdateItemInput},
    FromAttributes, Item,
};
use failure::{bail, Error};
use log::debug;
use std::collections::HashMap;
use std::env;

thread_local! {
    static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
}

fn get_connections_table() -> String {
    env::var("connectionsTable").unwrap_or_default()
}
fn find_connection_in_db(connection: UnresolvedConnection) -> Result<Connection, Error> {
    let res = DDB.with(|ddb| {
        ddb.get_item(GetItemInput {
            table_name: get_connections_table(),
            key: connection.key(),
            ..GetItemInput::default()
        })
        .sync()
    });
    res.map_err(Error::from)
        .and_then(|result| {
            result
                .item
                .ok_or_else(|| ConnectionItemError::NoConnection)
                .map_err(Error::from)
        })
        .map(Connection::from_attrs)
        .and_then(|connection| connection.map_err(Error::from))
}

pub fn find_user(id: String) -> Result<Connection, Error> {
    let unresolved_connection = UnresolvedConnection { id };
    find_connection_in_db(unresolved_connection)
}

pub fn find_admin(role: Role) -> Result<Connection, Error> {
    let admin_role = match role {
        Role::PlayerPong => Role::AdminPong,
        Role::PlayerDisplay => Role::AdminDisplay,
        _ => bail!("Unknown player"),
    };

    let mut expression_attribute_names = HashMap::new();
    expression_attribute_names.insert("#R".to_string(), "role".to_string());
    let res = DDB.with(|ddb| {
        ddb.scan(ScanInput {
            table_name: get_connections_table(),
            expression_attribute_names: Some(expression_attribute_names),
            expression_attribute_values: Some(attr_map!(
                ":val" =>  admin_role
            )),
            filter_expression: Some("#R = :val".into()),
            ..ScanInput::default()
        })
        .sync()
    });
    let items = res?.items.ok_or(ConnectionItemError::NoConnection)?;
    let admin_item = items.get(0).ok_or(ConnectionItemError::NoConnection)?;
    let admin = Connection::from_attrs(admin_item.to_owned()).map_err(Error::from);
    admin
}

pub fn find_players(role: Role) -> Result<Vec<Connection>, Error> {
    let player_role = match role {
        Role::AdminPong => Role::PlayerPong,
        Role::AdminDisplay => Role::PlayerDisplay,
        _ => bail!("Unknown player"),
    };

    let mut expression_attribute_names = HashMap::new();
    expression_attribute_names.insert("#R".to_string(), "role".to_string());
    let res = DDB.with(|ddb| {
        ddb.scan(ScanInput {
            table_name: get_connections_table(),
            expression_attribute_names: Some(expression_attribute_names),
            expression_attribute_values: Some(attr_map!(
                ":val" =>  player_role
            )),
            filter_expression: Some("#R = :val".into()),
            ..ScanInput::default()
        })
        .sync()
    });
    let players = res?.items.ok_or(ConnectionItemError::NoConnection)?;
    let player_connections: Vec<_> = players
        .iter()
        .map(|player| Connection::from_attrs(player.to_owned()).unwrap())
        .collect();
    Ok(player_connections)
}

pub fn find_next_in_que(role: Role) -> Result<Connection, Error> {
    let mut expression_attribute_names = HashMap::new();
    expression_attribute_names.insert("#R".to_string(), "role".to_string());
    expression_attribute_names.insert("#Q".to_string(), "que".to_string());

    let res = DDB.with(|ddb| {
        ddb.scan(ScanInput {
            table_name: get_connections_table(),
            expression_attribute_names: Some(expression_attribute_names),
            expression_attribute_values: Some(attr_map!(
                ":val" =>  role,
                ":que" => true
            )),
            filter_expression: Some("#R = :val and #Q = :que".into()),
            ..ScanInput::default()
        })
        .sync()
    });
    let items = res?.items.ok_or(ConnectionItemError::NoConnection)?;
    let player_item = items.get(0).ok_or(ConnectionItemError::NoConnection)?;
    let player = Connection::from_attrs(player_item.to_owned()).map_err(Error::from);
    player
}

pub fn mark_player_active(id: String) {
    let unresolved_connection = UnresolvedConnection { id };

    let res = DDB.with(|ddb| {
        ddb.update_item(UpdateItemInput {
            table_name: get_connections_table(),
            update_expression: Some("SET que = true".to_string()),
            key: unresolved_connection.key(),
            ..UpdateItemInput::default()
        })
        .sync()
    });
    if let Err(err) = res {
        debug!("error setting que {}", err);
    }
}

pub fn has_player(role: Role) -> bool {
    let mut expression_attribute_names = HashMap::new();
    expression_attribute_names.insert("#R".to_string(), "role".to_string());
    expression_attribute_names.insert("#Q".to_string(), "que".to_string());

    let res = DDB.with(|ddb| {
        ddb.scan(ScanInput {
            table_name: get_connections_table(),
            expression_attribute_names: Some(expression_attribute_names),
            expression_attribute_values: Some(attr_map!(
                ":val" =>  role,
                ":que" => false
            )),
            select: Some("COUNT".to_string()),
            filter_expression: Some("#R = :val and #Q = :que".into()),
            ..ScanInput::default()
        })
        .sync()
    });
    let count = res.map(|res| res.count.unwrap_or(0)).unwrap_or(0);
    count > 0
}
