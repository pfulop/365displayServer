use rusoto_dynamodbstreams::*;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Clone)]
pub struct RequestContext {
    pub event_type: String,
    pub connection_id: String,
    pub domain_name: String,
    pub stage: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Clone)]
pub struct Event {
    pub request_context: RequestContext,
    pub records: Option<Vec<Record>>,
    pub body: Option<String>,
}

impl Event {
    pub fn message(&self) -> String {
        let result = &self.body.clone().unwrap();
        result.clone()
    }
}
