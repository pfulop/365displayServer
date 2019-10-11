use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponse {
    pub status_code: i16,
}
