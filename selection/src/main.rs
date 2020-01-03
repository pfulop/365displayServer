use common::*;
use dynomite::{
    attr_map,
    dynamodb::{DynamoDb, DynamoDbClient, PutItemInput, ScanInput, UpdateItemInput},
};
use failure::Error;
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::{debug, error, Level};
use serde::{Deserialize, Serialize};
use serde_json;
use simple_logger;
use std::collections::HashMap;
use std::env;
use std::string::ToString;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
struct SelectionMessage {
    role: models::Role,
    password: Option<String>,
}

thread_local!(
    static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
);

fn main() {
    simple_logger::init_with_level(Level::Debug).unwrap();
    lambda!(handler)
}

fn handler(event: events::Event, _: Context) -> Result<responses::HttpResponse, HandlerError> {
    let message = event.message();
    let message_content: SelectionMessage = serde_json::from_str(&message)?;
    match message_content.role {
        models::Role::AdminDisplay | models::Role::AdminPong => {
            if message_content.password.unwrap_or_else(|| "_".to_owned())
                == "FikinkoPoznaSvojePrava321"
            {
                let m = SelectionMessage {
                    role: message_content.role,
                    password: None,
                };
                save_role(m, event)
            } else {
                error!("Wrong admin password");
                Err("Wrong admin password".into())
            }
        }
        _ => save_role(message_content, event),
    }
}

fn save_role(
    message_content: SelectionMessage,
    event: events::Event,
) -> Result<responses::HttpResponse, HandlerError> {
    let table_name = env::var("connectionsTable")?;
    let mut expression_attribute_names = HashMap::new();
    expression_attribute_names.insert("#R".to_string(), "role".to_string());

    let res = DDB.with(|ddb| {
        ddb.scan(ScanInput {
            table_name: table_name.clone(),
            select: Some("COUNT".into()),
            expression_attribute_names: Some(expression_attribute_names),
            expression_attribute_values: Some(attr_map!(
                ":val" =>  message_content.role
            )),
            filter_expression: Some("#R = :val".into()),
            ..ScanInput::default()
        })
        .sync()
    });

    let n_existing = res.unwrap().count.unwrap();

    match message_content.role {
        models::Role::AdminDisplay | models::Role::AdminPong => {
            if n_existing > 0 {
                error!("Someone is trying to become another admin");
                Ok(responses::HttpResponse { status_code: 500 })
            } else {
                set_role(message_content, table_name, event);
                Ok(responses::HttpResponse { status_code: 200 })
            }
        }
        models::Role::PlayerDisplay => {
            if n_existing > 0 {
                debug!("There is too many players, putting into que");
                put_into_que(message_content, table_name, event, n_existing);
                Ok(responses::HttpResponse { status_code: 200 })
            } else {
                set_role(message_content, table_name, event);
                Ok(responses::HttpResponse { status_code: 200 })
            }
        }
        models::Role::PlayerPong => {
            if n_existing > 1 {
                debug!("There is too many players, putting into que");
                put_into_que(message_content, table_name, event, n_existing);
                Ok(responses::HttpResponse { status_code: 200 })
            } else {
                let connection = set_role(message_content, table_name.clone(), event.clone());
                send::inform_server(event, connection.id, table_name, "CONNECTED".to_string()); //TODO: fix this
                Ok(responses::HttpResponse { status_code: 200 })
            }
        }
        _ => Ok(responses::HttpResponse { status_code: 200 }),
    }
}

fn put_into_que(
    message_content: SelectionMessage,
    table_name: String,
    event: events::Event,
    n_existing: i64,
) {
    let connection = models::Connection {
        id: event.request_context.connection_id.to_owned(),
        role: Some(message_content.role),
        que: true,
    };
    let res = DDB.with(|ddb| {
        ddb.put_item(PutItemInput {
            table_name: table_name.clone(),
            item: connection.into(),
            ..PutItemInput::default()
        })
        .sync()
        .map(drop)
        .map_err(Error::from)
    });

    if let Err(err) = res {
        error!("There has been an error setting role {}", err);
    } else {
        match message_content.role {
            models::Role::PlayerDisplay => {
                let mut expression_attribute_names = HashMap::new();
                expression_attribute_names.insert("#R".to_string(), "role".to_string());
                expression_attribute_names.insert("#Q".to_string(), "que".to_string());
                let now = SystemTime::now();
                let clear_at = now.checked_add(Duration::new(10, 0)).unwrap();
                let since_the_epoch = clear_at
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");
                let res = DDB.with(|ddb| {
                    ddb.update_item(UpdateItemInput {
                        table_name: table_name,
                        expression_attribute_names: Some(expression_attribute_names),
                        expression_attribute_values: Some(attr_map!(
                            ":roleval" =>  message_content.role,
                            ":queval" =>  false,
                            ":clearAt" => since_the_epoch.as_secs(),
                        )),
                        condition_expression: Some("#R = :roleval AND #Q = :queval".into()),
                        update_expression: Some("SET clearAt = :clearAt".into()),
                        ..UpdateItemInput::default()
                    })
                    .sync()
                });
                if let Err(err) = res {
                    error!("There has been an error setting que ttl {}", err);
                }
            }
            _ => {}
        }
        send::put_in_que(event, message_content.role, n_existing);
    }
}

fn set_role(
    message_conent: SelectionMessage,
    table_name: String,
    event: events::Event,
) -> models::Connection {
    let role = message_conent.role;
    let connection = models::Connection {
        id: event.request_context.connection_id.to_owned(),
        role: Some(message_conent.role),
        que: false,
    };
    let return_connection = connection.clone();
    let res = DDB.with(|ddb| {
        ddb.put_item(PutItemInput {
            table_name,
            item: connection.into(),
            ..PutItemInput::default()
        })
        .sync()
        .map(drop)
        .map_err(Error::from)
    });

    if let Err(err) = res {
        error!("There has been an error setting role {}", err);
    } else {
        send::role_accepted(event.to_owned(), role);
        if let Ok(admin) = connection_operations::find_admin(message_conent.role) {
            send::inform_server(
                event,
                return_connection.clone().id,
                admin.id,
                "CONNECTED".to_string(),
            );
        }
    }
    return_connection
}
