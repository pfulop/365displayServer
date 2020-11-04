use super::error::Error;
use super::models::*;
use dynomite::{
    attr_map,
    dynamodb::{
        DeleteItemInput, DynamoDb, DynamoDbClient, GetItemInput, PutItemInput, ScanInput,
        UpdateItemInput,
    },
    FromAttributes, Item,
};
use log::debug;
use std::collections::HashMap;
use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn get_connections_table() -> String {
    env::var("connectionsTable").unwrap_or_default()
}

pub async fn find_connection_in_db(connection: UnresolvedConnection) -> Result<Connection, Error> {
    let client = DynamoDbClient::new(Default::default());

    let res = client
        .get_item(GetItemInput {
            table_name: get_connections_table(),
            key: connection.key(),
            ..GetItemInput::default()
        })
        .await?;

    let item = res
        .item
        .map(Connection::from_attrs)
        .ok_or("Missing Connection")?;
    item.map_err(|e| e.into())
}

pub async fn find_admin(role: Role) -> Result<Connection, Error> {
    let client = DynamoDbClient::new(Default::default());
    let admin_role = match role {
        Role::PlayerPong => Role::AdminPong,
        Role::PlayerDisplay => Role::AdminDisplay,
        _ => return Err("Unknown player".into()),
    };

    let mut expression_attribute_names = HashMap::new();
    expression_attribute_names.insert("#R".to_string(), "role".to_string());
    let res = client
        .scan(ScanInput {
            table_name: get_connections_table(),
            expression_attribute_names: Some(expression_attribute_names),
            expression_attribute_values: Some(attr_map!(
            ":val" =>  admin_role
            )),
            filter_expression: Some("#R = :val".into()),
            ..ScanInput::default()
        })
        .await?;

    let items = res.items.ok_or("No admin found")?;
    let item = items.first().ok_or("No admin found")?.clone();
    let admin_item = Connection::from_attrs(item);

    admin_item.map_err(|e| e.into())
}

pub async fn find_players(role: Role) -> Result<Vec<Connection>, Error> {
    let client = DynamoDbClient::new(Default::default());
    let player_role = match role {
        Role::AdminPong => Role::PlayerPong,
        Role::AdminDisplay => Role::PlayerDisplay,
        _ => return Err("Unknown player".into()),
    };

    let mut expression_attribute_names = HashMap::new();
    expression_attribute_names.insert("#R".to_string(), "role".to_string());

    let res = client
        .scan(ScanInput {
            table_name: get_connections_table(),
            expression_attribute_names: Some(expression_attribute_names),
            expression_attribute_values: Some(attr_map!(
            ":val" =>  player_role
            )),
            filter_expression: Some("#R = :val".into()),
            ..ScanInput::default()
        })
        .await?;

    let items = res.items.ok_or("No players found")?;
    let player_connections: Vec<_> = items
        .iter()
        .map(|player| Connection::from_attrs(player.to_owned()))
        .map(|player| player.expect("Can't convert player"))
        .collect();
    Ok(player_connections)
}

pub async fn find_next_in_que(role: Role) -> Result<Connection, Error> {
    let client = DynamoDbClient::new(Default::default());
    let mut expression_attribute_names = HashMap::new();
    expression_attribute_names.insert("#R".to_string(), "role".to_string());
    expression_attribute_names.insert("#Q".to_string(), "que".to_string());

    let res = client
        .scan(ScanInput {
            table_name: get_connections_table(),
            expression_attribute_names: Some(expression_attribute_names),
            expression_attribute_values: Some(attr_map!(
            ":val" =>  role,
            ":que" => true
            )),
            filter_expression: Some("#R = :val and #Q = :que".into()),
            ..ScanInput::default()
        })
        .await?;

    let items = res.items.ok_or("No next player found")?;
    let item = items.first().ok_or("No admin found")?.clone();
    let admin_item = Connection::from_attrs(item);

    admin_item.map_err(|e| e.into())
}

pub async fn mark_player_active(id: String) {
    let client = DynamoDbClient::new(Default::default());
    let unresolved_connection = UnresolvedConnection { id };

    let res = client
        .update_item(UpdateItemInput {
            table_name: get_connections_table(),
            update_expression: Some("SET que = true".to_string()),
            key: unresolved_connection.key(),
            ..UpdateItemInput::default()
        })
        .await;

    if let Err(err) = res {
        debug!("error setting que {}", err);
    }
}

pub async fn has_player(role: Role) -> bool {
    let client = DynamoDbClient::new(Default::default());
    let mut expression_attribute_names = HashMap::new();
    expression_attribute_names.insert("#R".to_string(), "role".to_string());
    expression_attribute_names.insert("#Q".to_string(), "que".to_string());

    let res = client
        .scan(ScanInput {
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
        .await;

    let count = res.map(|res| res.count.unwrap_or(0)).unwrap_or(0);
    count > 0
}

pub async fn delete_player(id: String) {
    let client = DynamoDbClient::new(Default::default());
    let connection = Connection {
        id,
        role: None,
        que: false,
    };
    let res = client
        .delete_item(DeleteItemInput {
            table_name: env::var("tableName").expect("failed to resolve table"),
            key: connection.key(),
            ..DeleteItemInput::default()
        })
        .await;

    if let Err(err) = res {
        debug!("error deleting connection {:?}", err);
    }
}

pub async fn save_player(id: String) {
    let client = DynamoDbClient::new(Default::default());
    let connection = Connection {
        id,
        role: Some(Role::Observer),
        que: false,
    };

    let res = client
        .put_item(PutItemInput {
            table_name: get_connections_table(),
            item: connection.into(),
            ..PutItemInput::default()
        })
        .await;

    if let Err(err) = res {
        debug!("error creating connection {:?}", err);
    }
}

pub async fn put_into_que(id: String, role: Role) {
    let client = DynamoDbClient::new(Default::default());
    let connection = Connection {
        id,
        role: Some(role),
        que: true,
    };

    let res = client
        .put_item(PutItemInput {
            table_name: get_connections_table(),
            item: connection.into(),
            ..PutItemInput::default()
        })
        .await;

    if let Err(err) = res {
        debug!("error creating connection {:?}", err);
    }
}

pub async fn get_player_count_by_role(role: Role) -> Result<i64, Error> {
    let client = DynamoDbClient::new(Default::default());

    let mut expression_attribute_names = HashMap::new();
    expression_attribute_names.insert("#R".to_string(), "role".to_string());

    let res = client
        .scan(ScanInput {
            table_name: get_connections_table(),
            select: Some("COUNT".into()),
            expression_attribute_names: Some(expression_attribute_names),
            expression_attribute_values: Some(attr_map!(
                ":val" =>  role
            )),
            filter_expression: Some("#R = :val".into()),
            ..ScanInput::default()
        })
        .await?;

    let n_existing = res.count.ok_or("Can't find players for role");
    n_existing.map_err(|e| e.into())
}

pub async fn save_connection(connection: Connection) -> Result<Connection, Error> {
    let client = DynamoDbClient::new(Default::default());

    client
        .put_item(PutItemInput {
            table_name: get_connections_table(),
            item: connection.clone().into(),
            ..PutItemInput::default()
        })
        .await?;

    Ok(connection)
}

pub async fn time_out_first_in_que(role: Role) {
    let client = DynamoDbClient::new(Default::default());

    let mut expression_attribute_names = HashMap::new();
    expression_attribute_names.insert("#R".to_string(), "role".to_string());
    expression_attribute_names.insert("#Q".to_string(), "que".to_string());

    let now = SystemTime::now();
    let clear_at = now.checked_add(Duration::new(10, 0)).unwrap();
    let since_the_epoch = clear_at
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    let res = client
        .update_item(UpdateItemInput {
            table_name: get_connections_table(),
            expression_attribute_names: Some(expression_attribute_names),
            expression_attribute_values: Some(attr_map!(
                ":roleval" =>  role,
                ":queval" =>  false,
                ":clearAt" => since_the_epoch.as_secs(),
            )),
            condition_expression: Some("#R = :roleval AND #Q = :queval".into()),
            update_expression: Some("SET clearAt = :clearAt".into()),
            ..UpdateItemInput::default()
        })
        .await;

    if let Err(err) = res {
        debug!("error chanigng que, {}", err);
    }
}
