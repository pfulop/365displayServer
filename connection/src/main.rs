use common::*;
use dynomite::dynamodb::{DeleteItemInput, DynamoDb, DynamoDbClient, PutItemInput};
use dynomite::Item;
use futures::Future;
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::{error, Level};
use simple_logger;
use std::env;
use tokio::runtime::Runtime;

fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();
    lambda!(handler)
}

fn handler(event: events::Event, _: Context) -> Result<responses::HttpResponse, HandlerError> {
    let table_name = env::var("connectionsTable")?;
    let mut rt = Runtime::new().expect("failed to initialize futures runtime");
    let d_d_b = DynamoDbClient::new(Default::default());
    let result = match event.request_context.event_type.as_ref() {
        "CONNECT" => {
            let connection = models::Connection {
                id: event.request_context.connection_id,
                role: Some(models::Role::Observer),
                que: false,
            };
            let res = rt.block_on(
                d_d_b
                    .put_item(PutItemInput {
                        table_name,
                        item: connection.into(),
                        ..PutItemInput::default()
                    })
                    .map(drop)
                    .map_err(connection_enums::ConnectionError::Connect),
            );
            res
        }
        "DISCONNECT" => {
            let connection = models::Connection {
                id: event.request_context.connection_id,
                role: None,
                que: false,
            };
            let res = rt.block_on(
                d_d_b
                    .delete_item(DeleteItemInput {
                        table_name,
                        key: connection.key(),
                        ..DeleteItemInput::default()
                    })
                    .map(drop)
                    .map_err(connection_enums::ConnectionError::Disconnect),
            );
            res
        }
        _ => send::pong(event),
    };

    if let Err(err) = result {
        error!("Failed to work with connection: {:?}", err);
        return Ok(responses::HttpResponse { status_code: 500 });
    }

    Ok(responses::HttpResponse { status_code: 200 })
}
