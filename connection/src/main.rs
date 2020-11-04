use aws_lambda_events::event::apigw::ApiGatewayWebsocketProxyRequest;
use common::{connection_operations, error::Error, models, send};
use lambda::{lambda, Context};
use simple_logger::SimpleLogger;

#[lambda]
#[tokio::main]
async fn main(e: ApiGatewayWebsocketProxyRequest, _: Context) -> Result<(), Error> {
    SimpleLogger::new().init().unwrap();

    let event = e
        .clone()
        .request_context
        .event_type
        .ok_or("Missing Event Type")?;

    match event.as_ref() {
        "CONNECT" => {
            connection_operations::save_player(
                e.request_context
                    .connection_id
                    .ok_or("Missing connection id")?,
            )
            .await;
        }
        "DISCONNECT" => {
            let connection_id = e
                .clone()
                .request_context
                .connection_id
                .ok_or("Missing Connection ID")?;
            let unresolved_connection = models::UnresolvedConnection { id: connection_id };
            let connection =
                connection_operations::find_connection_in_db(unresolved_connection.clone()).await?;
            if !connection.que {
                match connection.role {
                    Some(models::Role::PlayerPong) | Some(models::Role::PlayerDisplay) => {
                        let admin =
                            connection_operations::find_admin(connection.role.unwrap()).await?;
                        send::inform_server(
                            e.request_context.clone(),
                            connection.id.clone(),
                            admin.id.clone(),
                            "DISCONNECTED".to_string(),
                        )
                        .await;
                        if let Ok(player) =
                            connection_operations::find_next_in_que(connection.role.unwrap()).await
                        {
                            send::inform_server(
                                e.request_context.clone(),
                                player.id.clone(),
                                admin.id,
                                "CONNECTED".to_string(),
                            )
                            .await;
                            connection_operations::mark_player_active(player.id).await;
                        }
                    }
                    _ => {}
                }
            }
            connection_operations::delete_player(unresolved_connection.id).await;
        }
        _ => {
            log::warn!("UNKNOWN EVENT {}", event);
        }
    };
    Ok(())
}
