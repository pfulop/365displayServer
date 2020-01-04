use common::{connection_operations::*, events::*, models::*, responses::*, send::*};
use dynomite::{
    attr_map,
    dynamodb::{DynamoDb, DynamoDbClient, PutItemInput, ScanInput, UpdateItemInput},
};
use dynomite::{Attributes, FromAttributes, Item};
use failure::Error;
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::{debug, error, Level};
use serde::{Deserialize, Serialize};

thread_local!(
    static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
);

fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();
    lambda!(handler)
}

fn handler(event: Event, _: Context) -> Result<HttpResponse, HandlerError> {
    let records = event.records.to_owned();
    records.map(|records| {
        for record in records.iter() {
            if record.event_name == Some("REMOVE".to_string()) {
                record
                    .dynamodb
                    .to_owned()
                    .and_then(|dynamodb| dynamodb.old_image)
                    .and_then(|item| item.get("role").and_then(|role| Some(role.clone())))
                    .and_then(|role| role.s.clone())
                    .and_then(|role_string| role_string.parse::<Role>().ok())
                    .and_then(|role| {
                        if !has_player(role) {
                            if let Ok(admin) = find_admin(role) {
                                if let Ok(player) = find_next_in_que(role) {
                                    inform_server(
                                        event.clone(),
                                        player.id.clone(),
                                        admin.id,
                                        "CONNECTED".to_string(),
                                    );
                                    mark_player_active(player.id)
                                }
                            }
                        }
                        Some("1")
                    });
            }
        }
    });

    return Ok(HttpResponse { status_code: 200 });
}
