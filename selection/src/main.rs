use aws_lambda_events::event::apigw::ApiGatewayWebsocketProxyRequest;
use common::{connection_operations, error::Error, models, send};
use lambda::{lambda, Context};
use serde::{Deserialize, Serialize};
use serde_json;
use simple_logger::SimpleLogger;
use std::string::ToString;

#[derive(Debug, Serialize, Deserialize)]
struct SelectionMessage {
    role: models::Role,
    password: Option<String>,
}

#[lambda]
#[tokio::main]
async fn main(e: ApiGatewayWebsocketProxyRequest, _: Context) -> Result<(), Error> {
    SimpleLogger::new().init().unwrap();

    let message = e.body.clone().unwrap();
    let message_content: SelectionMessage = serde_json::from_str(&message)?;

    match message_content.role {
        models::Role::AdminDisplay | models::Role::AdminPong => {
            if message_content.password.unwrap_or_else(|| "_".to_owned())
                == "FikinkoPoznaSvojePrava321"
            {
                let m = SelectionMessage {
                    role: message_content.role,
                    password: None,
                };
                save_role(m, e).await;
            } else {
                return Err("Wrong admin password".into());
            }
        }
        _ => {
            save_role(message_content, e).await;
        }
    }

    Ok(())
}

async fn save_role(
    message_content: SelectionMessage,
    event: ApiGatewayWebsocketProxyRequest,
) -> Result<(), Error> {
    let n_existing = connection_operations::get_player_count_by_role(message_content.role).await?;

    match message_content.role {
        models::Role::AdminDisplay | models::Role::AdminPong => {
            if n_existing == 0 {
                set_role(message_content, event).await;
            }
            Ok(())
        }
        models::Role::PlayerDisplay => {
            if n_existing > 0 {
                put_into_que(message_content, event, n_existing).await;
                Ok(())
            } else {
                set_role(message_content, event).await;
                Ok(())
            }
        }
        models::Role::PlayerPong => {
            if n_existing > 1 {
                put_into_que(message_content, event, n_existing).await;
                Ok(())
            } else {
                let admin = connection_operations::find_admin(models::Role::PlayerPong).await?;
                let connection = set_role(message_content, event.clone()).await;
                send::inform_server(
                    event.request_context,
                    connection.id,
                    admin.id,
                    "CONNECTED".to_string(),
                )
                .await;
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

async fn put_into_que(
    message_content: SelectionMessage,
    event: ApiGatewayWebsocketProxyRequest,
    n_existing: i64,
) {
    let connection_id = event
        .clone()
        .request_context
        .connection_id
        .unwrap_or_default();
    connection_operations::put_into_que(connection_id, message_content.role).await;

    match message_content.role {
        models::Role::PlayerDisplay => {
            connection_operations::time_out_first_in_que(message_content.role).await;
        }
        _ => {}
    }
    send::put_in_que(event.request_context, message_content.role, n_existing).await;
}

async fn set_role(
    message_conent: SelectionMessage,
    event: ApiGatewayWebsocketProxyRequest,
) -> models::Connection {
    let role = message_conent.role;
    let connection_id = event
        .clone()
        .request_context
        .connection_id
        .unwrap_or_default();

    let connection = models::Connection {
        id: connection_id,
        role: Some(message_conent.role),
        que: false,
    };

    let res = connection_operations::save_connection(connection.clone()).await;

    if let Ok(con) = res {
        send::role_accepted(event.request_context.clone(), role).await;
        if let Ok(admin) = connection_operations::find_admin(message_conent.role).await {
            send::inform_server(
                event.request_context,
                con.clone().id,
                admin.id,
                "CONNECTED".to_string(),
            )
            .await;
        }
    }

    connection
}
