use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub message: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestContext {
    pub event_type: String,
    pub connection_id: String,
    pub domain_name: String,
    pub stage: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub request_context: RequestContext,
    body: Option<String>,
}

impl Event {
    pub fn message(&self) -> Option<Message> {
        let body = &self.body.clone().unwrap();
        serde_json::from_str::<Message>(body).ok()
    }
}
