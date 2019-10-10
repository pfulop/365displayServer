use dynomite::{Attribute, Item};
use serde::{Deserialize, Serialize};

#[derive(Attribute, Debug, Serialize, Deserialize)]
pub enum Role {
    Observer,
    Player,
    Admin,
}

#[derive(Serialize, Deserialize, Debug, Item)]
pub struct Connection {
    #[dynomite(partition_key)]
    pub id: String,
    pub role: Option<Role>,
}
