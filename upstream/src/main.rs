//FROM CLIENT TO SERVER

use common::{connection_operations::*, events::*, responses::*, send::*};
use dynomite::dynamodb::DynamoDbClient;
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::Level;
use simple_logger;

thread_local!(
    static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
);

fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();
    lambda!(handler)
}

fn handler(event: Event, _: Context) -> Result<HttpResponse, HandlerError> {
    let message = event.message().clone();
    let player = find_user(event.request_context.connection_id.clone())?;
    let admin = find_admin(player.role.unwrap())?;
    if player.id != admin.id {
        send(event, admin.id, message);
    }
    return Ok(HttpResponse { status_code: 200 });
}
