use common::*;
use dynomite::dynamodb::{DeleteItemInput, DynamoDb, DynamoDbClient, PutItemInput};
use dynomite::Item;
use failure::Error;
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::{error, Level};
use simple_logger;
use std::env;

thread_local! {
    static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
}

fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();
    lambda!(handler)
}

fn handler(event: events::Event, _: Context) -> Result<responses::HttpResponse, HandlerError> {
    let table_name = env::var("connectionsTable")?;
    let result = match event.request_context.event_type.as_ref() {
        "CONNECT" => {
            let connection = models::Connection {
                id: event.request_context.connection_id,
                role: Some(models::Role::Observer),
                que: false,
            };
            DDB.with(|ddb| {
                ddb.put_item(PutItemInput {
                    table_name,
                    item: connection.into(),
                    ..PutItemInput::default()
                })
                .sync()
                .map(drop)
                .map_err(Error::from)
            })
        }
        "DISCONNECT" => connection_operations::find_user(
            event.request_context.connection_id.clone(),
        )
        .and_then(|connection| {
            if !connection.que {
                match connection.role {
                    Some(models::Role::PlayerPong) | Some(models::Role::PlayerDisplay) => {
                        if let Ok(admin) =
                            connection_operations::find_admin(connection.role.unwrap())
                        {
                            send::inform_server(
                                event.clone(),
                                connection.id.clone(),
                                admin.id.clone(),
                                "DISCONNECTED".to_string(),
                            );
                            if let Ok(player) =
                                connection_operations::find_next_in_que(connection.role.unwrap())
                            {
                                send::inform_server(
                                    event.clone(),
                                    player.id.clone(),
                                    admin.id,
                                    "CONNECTED".to_string(),
                                );
                                connection_operations::mark_player_active(player.id)
                            }
                        }
                    }
                    _ => {}
                }
            }
            DDB.with(|ddb| {
                ddb.delete_item(DeleteItemInput {
                    table_name,
                    key: connection.key(),
                    ..DeleteItemInput::default()
                })
                .sync()
                .map(drop)
                .map_err(Error::from)
            })
        }),
        _ => send::pong(event),
    };

    if let Err(err) = result {
        error!("Failed to work with connection: {:?}", err);
        return Ok(responses::HttpResponse { status_code: 500 });
    }

    Ok(responses::HttpResponse { status_code: 200 })
}
