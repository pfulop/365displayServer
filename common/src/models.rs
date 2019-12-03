use dynomite::{Attribute, Item};
use serde::{Deserialize, Serialize};

#[derive(Attribute, Debug, Serialize, Deserialize, Copy, Clone)]
pub enum Role {
    Observer,
    PlayerPong,
    PlayerDisplay,
    AdminPong,
    AdminDisplay,
}

#[derive(Serialize, Deserialize, Debug, Item, Clone)]
pub struct Connection {
    #[dynomite(partition_key)]
    pub id: String,
    pub role: Option<Role>,
    pub que: bool,
}
