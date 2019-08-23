use lambda_runtime::{error::HandlerError, lambda, Context};
use log::info;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CustomOutput {
    status_code: i16,
}

fn main() {
    let _guard = sentry::init("https://fbd344796c774cc9a67a908d0781a9d4@sentry.io/1539573");
    info!("main");
    return lambda!(handler);
}

fn handler(_event: Value, _: Context) -> Result<CustomOutput, HandlerError> {
    info!("handler");
    return Ok(CustomOutput { status_code: 200 });
}
