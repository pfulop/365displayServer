use dynomite::{
    dynamodb::{
        DeleteItemError, DeleteItemInput, DynamoDb, DynamoDbClient, PutItemError, PutItemInput,
    },
};
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::Level;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use simple_logger;

mod models;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CustomOutput {
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


fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();
    return lambda!(handler);
}

fn handler(event: Event, _: Context) -> Result<Value, HandlerError> {
    // let table_name = env::var("tableName")?;
    let connection = models::Connection {
        id: event.request_context.connection_id,
        role: models::Role::Observer
    };
    let DDB = DynamoDbClient::new(Default::default());

    let result = match event.request_context.event_type {
        EventType::Connect => {
            DDB.put_item(PutItemInput { item: connection.into(), ..PutItemInput::default() })
            // DDB.with(|ddb| {
            //     Either::A(
            //         ddb.put_item(PutItemInput {
            //             table_name,
            //             item: connection.clone().into(),
            //             ..PutItemInput::default()
            //         })
            //         .map(drop)
            //         .map_err(Error::Connect),
            //     )
            // })
        }
        EventType::Disconnect => {
            DDB.delete_item(DeleteItemInput { key: connection.key(), ..DeleteItemInput::default() })
        }
    };

    if let Err(err) = RT.with(|rt| rt.borrow_mut().block_on(result)) {
        log::error!("failed to perform connection operation: {:?}", err);
    }

    Ok(json!({
        "statusCode": 200
    }))
}
