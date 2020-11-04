use crate::connection_operations::delete_player;
use crate::error::Error;
use crate::models;
use aws_lambda_events::event::apigw::{
    ApiGatewayWebsocketProxyRequest, ApiGatewayWebsocketProxyRequestContext,
};
use bytes::Bytes;
use dynomite::dynamodb::{DynamoDb, DynamoDbClient, GetItemInput};
use dynomite::{FromAttributes, Item};
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

fn endpoint(ctx: &ApiGatewayWebsocketProxyRequestContext) -> String {
    format!(
        "https://{}/{}",
        ctx.domain_name.clone().unwrap_or_default(),
        ctx.stage.clone().unwrap_or_default()
    )
}

pub async fn pong(
    request_context: ApiGatewayWebsocketProxyRequestContext,
    client: DynamoDbClient,
) -> Result<(), Error> {
    let table_name = env::var("connectionsTable")?;
    let connection_id = request_context
        .clone()
        .connection_id
        .ok_or("Missing Connection ID")?;

    let connection = models::Connection {
        id: connection_id.clone(),
        role: None,
        que: false,
    };

    let res = client
        .get_item(GetItemInput {
            table_name,
            key: connection.key(),
            ..GetItemInput::default()
        })
        .await?;

    res.item
        .map(models::Connection::from_attrs)
        .ok_or("No connection found")?
        .map(|connection| serde_json::to_string(&connection))
        .map(|message_string| async {
            let message = message_string.expect("There should be a message");
            send(request_context, connection_id.clone(), message).await;
            Ok(())
        })
        .expect("Error sending message")
        .await
}

pub async fn role_accepted(
    request_context: ApiGatewayWebsocketProxyRequestContext,
    role: models::Role,
) {
    match request_context.clone().connection_id {
        Some(connection_id) => {
            let message = serde_json::to_string(&json!({ "role": role, "status": "accepted" }))
                .unwrap_or_default();
            send(request_context, connection_id, message).await;
        }
        None => {}
    }
}

pub async fn put_in_que(
    request_context: ApiGatewayWebsocketProxyRequestContext,
    role: models::Role,
    order: i64,
) {
    match request_context.clone().connection_id {
        Some(connection_id) => {
            let message =
                serde_json::to_string(&json!({ "role": role, "status": "que", "order": order }))
                    .unwrap_or_default();
            send(request_context, connection_id, message).await;
        }
        None => {}
    }
}

pub async fn inform_server(
    request_context: ApiGatewayWebsocketProxyRequestContext,
    id: String,
    admin_id: String,
    status: String,
) {
    let message =
        serde_json::to_string(&json!({ "connection": id, "status": status})).unwrap_or_default();
    send(request_context, admin_id, message).await;
}

pub async fn send(
    request_context: ApiGatewayWebsocketProxyRequestContext,
    connection_id: String,
    message: String,
) {
    let default_region = Region::default().name().to_owned();
    let client = ApiGatewayManagementApiClient::new(Region::Custom {
        name: default_region,
        endpoint: endpoint(&request_context),
    });
    let reply_result = client
        .post_to_connection(PostToConnectionRequest {
            connection_id: connection_id.clone(),
            data: Bytes::from(message),
        })
        .await;

    if let Err(RusotoError::Service(PostToConnectionError::Gone(_))) = reply_result {
        delete_player(connection_id).await;
    }
}
