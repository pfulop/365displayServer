//FROM CLIENT TO SERVER

use aws_lambda_events::event::apigw::ApiGatewayWebsocketProxyRequest;
use common::{connection_operations::*, error::Error, models, send};
use lambda::{lambda, Context};
use simple_logger::SimpleLogger;

#[lambda]
#[tokio::main]
async fn main(e: ApiGatewayWebsocketProxyRequest, _: Context) -> Result<(), Error> {
    SimpleLogger::new().init().unwrap();

    let message = e.body.clone().unwrap();

    let connection_id = e
        .clone()
        .request_context
        .connection_id
        .ok_or("Missing Connection ID")?;
    let unresolved_connection = models::UnresolvedConnection { id: connection_id };

    let player = find_connection_in_db(unresolved_connection).await?;
    let admin = find_admin(player.role.unwrap()).await?;
    if player.id != admin.id {
        send::send(e.request_context, admin.id, message);
    }

    Ok(())
}
