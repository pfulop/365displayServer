//FROM SERVER TO CLIENTS

use common::{connection_operations::*, events::*, models::*, responses::*, send::*};
use dynomite::dynamodb::DynamoDbClient;
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::Level;
use serde::{Deserialize, Serialize};
use simple_logger;

#[derive(Debug, Serialize, Deserialize)]
struct AdminMessage {
    connection_id: Option<String>,
}

thread_local!(
    static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
);

fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();
    lambda!(handler)
}

fn handler(event: Event, _: Context) -> Result<HttpResponse, HandlerError> {
    let message = event.message().clone();
    let admin = find_user(event.request_context.connection_id.clone())?;
    let message_content: AdminMessage = serde_json::from_str(&message)?;

    match admin.role {
        Some(Role::AdminPong) | Some(Role::AdminDisplay) => {
            if let Some(connection_id) = message_content.connection_id {
                send(event.to_owned(), connection_id, message.clone());
            } else {
                let players = find_players(admin.role.unwrap())?;
                for player in players {
                    send(event.to_owned(), player.id, message.clone());
                }
            }
        }
        _ => {}
    };

    return Ok(HttpResponse { status_code: 200 });
}
