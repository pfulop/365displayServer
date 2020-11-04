//FROM SERVER TO CLIENTS
use aws_lambda_events::event::apigw::ApiGatewayWebsocketProxyRequest;
use common::{connection_operations, error::Error, models, send};
use lambda::{lambda, Context};
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;

#[derive(Debug, Serialize, Deserialize)]
struct AdminMessage {
    connection_id: Option<String>,
}

#[lambda]
#[tokio::main]
async fn main(e: ApiGatewayWebsocketProxyRequest, _: Context) -> Result<(), Error> {
    SimpleLogger::new().init().unwrap();

    let connection_id = e
        .clone()
        .request_context
        .connection_id
        .ok_or("Missing Connection ID")?;
    let unresolved_connection = models::UnresolvedConnection { id: connection_id };

    let message = e.body.clone().unwrap();
    let admin = connection_operations::find_connection_in_db(unresolved_connection).await?;
    let message_content: AdminMessage = serde_json::from_str(&message)?;

    match admin.role {
        Some(models::Role::AdminPong) | Some(models::Role::AdminDisplay) => {
            if let Some(connection_id) = message_content.connection_id {
                send::send(e.request_context.to_owned(), connection_id, message.clone()).await;
            } else {
                let players = connection_operations::find_players(admin.role.unwrap()).await?;
                for player in players {
                    send::send(e.request_context.to_owned(), player.id, message.clone()).await;
                }
            }
        }
        _ => {}
    };

    Ok(())
}
