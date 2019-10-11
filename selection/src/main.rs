use common::*;
use dynomite::Item;
use dynomite::{
    attr_map,
    dynamodb::{DeleteItemInput, DynamoDb, DynamoDbClient, PutItemInput, ScanInput},
};
use futures::Future;
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::{debug, error, Level};
use serde::{Deserialize, Serialize};
use serde_json;
use simple_logger;
use std::collections::HashMap;
use std::env;
use std::string::ToString;
use tokio::runtime::Runtime;

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
    let message = event.message().unwrap();
    let messageContent: SelectionMessage = serde_json::from_str(&message.message.unwrap())?;

    match messageContent.role {
        models::Role::AdminDisplay | models::Role::AdminPong => {
            if messageContent.password.unwrap_or_else(|| "_".to_owned())
                == "FikinkoPoznaSvojePrava321"
            {
                let m = SelectionMessage {
                    role: messageContent.role,
                    password: None,
                };
                save_role(m, event)
            } else {
                error!("Wrong admin password");
                Err("Wrong admin password".into())
            }
        }
        _ => save_role(messageContent, event),
    }
}

fn save_role(
    messageContent: SelectionMessage,
    event: events::Event,
) -> Result<responses::HttpResponse, HandlerError> {
    let table_name = env::var("connectionsTable")?;
    let mut rt = Runtime::new().expect("failed to initialize futures runtime");

    let mut expression_attribute_names = HashMap::new();
    expression_attribute_names.insert("#R".to_string(), "role".to_string());

    let res = DDB.with(|ddb| {
        rt.block_on(ddb.scan(ScanInput {
            table_name: table_name.clone(),
            select: Some("COUNT".into()),
            expression_attribute_names: Some(expression_attribute_names),
            expression_attribute_values: Some(attr_map!(
                ":val" =>  messageContent.role
            )),
            filter_expression: Some("#R = :val".into()),
            ..ScanInput::default()
        }))
    });

    let n_existing = res.unwrap().count.unwrap();

    match messageContent.role {
        models::Role::AdminDisplay | models::Role::AdminPong => {
            if n_existing > 0 {
                error!("Someone is trying to become another admin");
                Ok(responses::HttpResponse { status_code: 500 })
            } else {
                set_role(messageContent, table_name, event);
                Ok(responses::HttpResponse { status_code: 200 })
            }
        }
        models::Role::PlayerDisplay => {
            if n_existing > 0 {
                debug!("There is too many players, putting into que");
                put_into_que(messageContent, table_name, event);
                Ok(responses::HttpResponse { status_code: 200 })
            } else {
                set_role(messageContent, table_name, event);
                Ok(responses::HttpResponse { status_code: 200 })
            }
        }
        models::Role::PlayerPong => {
            if n_existing > 1 {
                debug!("There is too many players, putting into que");
                put_into_que(messageContent, table_name, event);
                Ok(responses::HttpResponse { status_code: 200 })
            } else {
                set_role(messageContent, table_name, event);
                Ok(responses::HttpResponse { status_code: 200 })
            }
        }
        _ => Ok(responses::HttpResponse { status_code: 200 }),
    }
}

fn put_into_que(messageContent: SelectionMessage, table_name: String, event: events::Event) {
    let mut rt = Runtime::new().expect("failed to initialize futures runtime");
    let connection = models::Connection {
        id: event.request_context.connection_id,
        role: Some(messageContent.role),
        que: Some(true),
    };
    let res = DDB.with(|ddb| {
        rt.block_on(
            ddb.put_item(PutItemInput {
                table_name,
                item: connection.into(),
                ..PutItemInput::default()
            })
            .map(drop)
            .map_err(connection_enums::ConnectionError::Connect),
        )
    });

    //TODO: add queing and repsond with que number
}

fn set_role(messageConent: SelectionMessage, table_name: String, event: events::Event) {
    let mut rt = Runtime::new().expect("failed to initialize futures runtime");
    let role = messageConent.role;
    let connection = models::Connection {
        id: event.request_context.connection_id.to_owned(),
        role: Some(messageConent.role),
        que: None,
    };
    DDB.with(|ddb| {
        rt.block_on(
            ddb.put_item(PutItemInput {
                table_name,
                item: connection.into(),
                ..PutItemInput::default()
            })
            .map(drop)
            .map_err(connection_enums::ConnectionError::Connect),
        )
    });
    send::role_accepted(event, role);
}
