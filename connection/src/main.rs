use common::*;
use dynomite::dynamodb::{DeleteItemInput, DynamoDb, DynamoDbClient, PutItemInput};
use dynomite::Item;
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::{error, Level};
use simple_logger;
use std::env;

thread_local! {
    static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
}

fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();
    lambda!(handler)
}

fn handler(event: events::Event, _: Context) -> Result<responses::HttpResponse, HandlerError> {
    let table_name = env::var("connectionsTable")?;
    let result = match event.request_context.event_type.as_ref() {
        "CONNECT" => {
            let connection = models::Connection {
                id: event.request_context.connection_id,
                role: Some(models::Role::Observer),
                que: false,
            };
            DDB.with(|ddb| {
                ddb.put_item(PutItemInput {
                    table_name,
                    item: connection.into(),
                    ..PutItemInput::default()
                })
                .sync()
                .map(drop)
                .map_err(connection_enums::ConnectionError::Connect)
            })
        }
        "DISCONNECT" => {
            let connection = models::Connection {
                id: event.request_context.connection_id,
                role: None,
                que: false,
            };
            DDB.with(|ddb| {
                ddb.delete_item(DeleteItemInput {
                    table_name,
                    key: connection.key(),
                    ..DeleteItemInput::default()
                })
                .sync()
                .map(drop)
                .map_err(connection_enums::ConnectionError::Disconnect)
            })
        }
        _ => send::pong(event),
    };

    if let Err(err) = result {
        error!("Failed to work with connection: {:?}", err);
        return Ok(responses::HttpResponse { status_code: 500 });
    }

    Ok(responses::HttpResponse { status_code: 200 })
}
