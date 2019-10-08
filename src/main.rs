use std::error::Error;

use dynomite::dynamodb::{
    DeleteItemError, DeleteItemInput, DynamoDb, DynamoDbClient, PutItemError, PutItemInput,
};
use dynomite::Item;
use futures::Future;
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::{error, Level};
use rusoto_core::RusotoError;
use serde::{Deserialize, Serialize};
use simple_logger;
use tokio::runtime::Runtime;

mod models;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HttpResponse {
    status_code: i16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Event {
    request_context: RequestContext,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestContext {
    event_type: EventType,
    connection_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum EventType {
    Connect,
    Disconnect,
}

#[derive(Debug)]
enum ConnectionError {
    Connect(RusotoError<PutItemError>),
    Disconnect(RusotoError<DeleteItemError>),
}

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(Level::Info).unwrap();
    lambda!(handler);

    Ok(())
}

fn handler(event: Event, _: Context) -> Result<HttpResponse, HandlerError> {
    // let table_name = env::var("tableName")?;
    let mut rt = Runtime::new().expect("failed to initialize futures runtime");
    let connection = models::Connection {
        id: event.request_context.connection_id,
        role: models::Role::Observer,
    };
    let d_d_b = DynamoDbClient::new(Default::default());

    let result = match event.request_context.event_type {
        EventType::Connect => {
            let res = rt.block_on(
                d_d_b
                    .put_item(PutItemInput {
                        item: connection.into(),
                        ..PutItemInput::default()
                    })
                    .map(drop)
                    .map_err(ConnectionError::Connect),
            );
            res
        }
        EventType::Disconnect => {
            let res = rt.block_on(
                d_d_b
                    .delete_item(DeleteItemInput {
                        key: connection.key(),
                        ..DeleteItemInput::default()
                    })
                    .map(drop)
                    .map_err(ConnectionError::Disconnect),
            );
            res
        }
    };

    if let Err(_) = result {
        error!("Cannot work with db");
    }

    Ok(HttpResponse { status_code: 200 })
}
