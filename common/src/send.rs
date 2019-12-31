use crate::connection_enums;
use crate::events;
use crate::models;
use bytes::Bytes;
use dynomite::dynamodb::{DeleteItemInput, DynamoDb, DynamoDbClient, GetItemInput};
use dynomite::{FromAttributes, Item};
use failure::Error;
use log::error;
use rusoto_apigatewaymanagementapi::{
    ApiGatewayManagementApi, ApiGatewayManagementApiClient, PostToConnectionError,
    PostToConnectionRequest,
};
use rusoto_core::{Region, RusotoError};
use serde_json::json;
use std::env;

thread_local! {
    static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
}

fn endpoint(ctx: &events::RequestContext) -> String {
    format!("https://{}/{}", ctx.domain_name, ctx.stage)
}

pub fn pong(event: events::Event) -> Result<(), Error> {
    let table_name = env::var("connectionsTable")?;

    let connection = models::Connection {
        id: event.request_context.connection_id.clone(),
        role: None,
        que: false,
    };

    let res = DDB.with(|ddb| {
        ddb.get_item(GetItemInput {
            table_name,
            key: connection.key(),
            ..GetItemInput::default()
        })
        .sync()
    });

    res.unwrap()
        .item
        .map(models::Connection::from_attrs)
        .unwrap()
        .map(|connection| serde_json::to_string(&connection))
        .unwrap()
        .map_err(|err| {
            error!("Cannot find connection: {:?}", err);
            connection_enums::ConnectionError::Default
        })
        .map(|message_string| {
            let connection_id = event.request_context.connection_id.clone();
            send(event, connection_id.clone(), message_string);
            Ok(())
        })
        .unwrap()
}

pub fn role_accepted(event: events::Event, role: models::Role) {
    let message =
        serde_json::to_string(&json!({ "role": role, "status": "accepted" })).unwrap_or_default();
    let connection_id = event.request_context.connection_id.clone();
    send(event, connection_id, message);
}

pub fn put_in_que(event: events::Event, role: models::Role, order: i64) {
    let message = serde_json::to_string(&json!({ "role": role, "status": "que", "order": order }))
        .unwrap_or_default();
    let connection_id = event.request_context.connection_id.clone();
    send(event, connection_id, message);
}

pub fn inform_server(event: events::Event, id: String, admin_id: String, status: String) {
    let message =
        serde_json::to_string(&json!({ "connection": id, "status": status})).unwrap_or_default();
    send(event, admin_id, message);
}

pub fn send(event: events::Event, connection_id: String, message: String) {
    let default_region = Region::default().name().to_owned();
    let client = ApiGatewayManagementApiClient::new(Region::Custom {
        name: default_region,
        endpoint: endpoint(&event.request_context),
    });
    let reply_result = client
        .post_to_connection(PostToConnectionRequest {
            connection_id: connection_id.clone(),
            data: Bytes::from(message),
        })
        .sync();

    if let Err(RusotoError::Service(PostToConnectionError::Gone(_))) = reply_result {
        let connection = models::Connection {
            id: connection_id.clone(),
            role: None,
            que: false,
        };
        log::info!("hanging up on disconnected client {}", connection_id);
        if let Err(err) = DDB.with(|ddb| {
            ddb.delete_item(DeleteItemInput {
                table_name: env::var("tableName").expect("failed to resolve table"),
                key: connection.key(),
                ..DeleteItemInput::default()
            })
            .sync()
        }) {
            error!("Cannot delete connection {} {}", connection_id, err);
        }
    }
}
