//FROM CLIENT TO SERVER

use common::{connection_operations::*, events::*, models::*, responses::*, send::*};
use dynomite::dynamodb::DynamoDbClient;
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::Level;
use simple_logger;

thread_local!(
    static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
);

fn main() {
    simple_logger::init_with_level(Level::Debug).unwrap();
    lambda!(handler)
}

fn handler(event: Event, _: Context) -> Result<HttpResponse, HandlerError> {
    let message = event.message().clone();
    let admin = find_user(event.request_context.connection_id.clone())?;

    match admin.role {
        Some(Role::AdminPong) | Some(Role::AdminDisplay) => {
            let players = find_players(admin.role.unwrap())?;
            for player in players {
                send(event.to_owned(), player.id, message.clone());
            }
        }
        _ => {}
    };

    return Ok(HttpResponse { status_code: 200 });
}
