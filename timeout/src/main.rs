use aws_lambda_events::event::apigw::ApiGatewayWebsocketProxyRequestContext;
use common::{connection_operations::*, error::Error, models::*, send::*};
use futures::future::try_join_all;
use lambda::{lambda, Context};
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone)]
struct DynamoDBEvent {
    records: Vec<DynamoDBEventRecord>,
    request_context: ApiGatewayWebsocketProxyRequestContext,
}

#[derive(Deserialize, Serialize, Clone)]
struct DynamoDBEventRecord {
    #[serde(rename = "dynamodb")]
    dynamodb: DynamoDBStreamRecord,
    #[serde(default)]
    #[serde(rename = "eventName")]
    event_name: String,
}

#[derive(Deserialize, Serialize, Clone)]
struct DynamoDBStreamRecord {
    #[serde(rename = "newImage")]
    new_image: HashMap<String, String>,
    #[serde(rename = "oldImage")]
    old_image: HashMap<String, String>,
}

#[lambda]
#[tokio::main]
async fn main(e: DynamoDBEvent, _: Context) -> Result<(), Error> {
    SimpleLogger::new().init().unwrap();

    let records = e.clone().records;
    let roles = records
        .iter()
        .map(|record| {
            if record.event_name == "REMOVE" {
                let role = record
                    .dynamodb
                    .old_image
                    .get("role")
                    .map(|role| role.parse::<Role>().unwrap())
                    .unwrap();
                Some(role)
            } else {
                None
            }
        })
        .filter(|role| role.is_some())
        .map(|role| role.unwrap())
        .map(|role| next_connection(role, e.clone()));

    try_join_all(roles).await?;

    Ok(())
}

async fn next_connection(role: Role, event: DynamoDBEvent) -> Result<(), Error> {
    if has_player(role).await {
        if let Ok(admin) = find_admin(role).await {
            if let Ok(player) = find_next_in_que(role).await {
                inform_server(
                    event.request_context,
                    player.id.clone(),
                    admin.id,
                    "CONNECTED".to_string(),
                )
                .await;
                mark_player_active(player.id).await;
            }
        }
    }
    Ok(())
}
