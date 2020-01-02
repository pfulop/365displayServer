use dynomite::{Attribute, Item};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Attribute, Debug, Serialize, Deserialize, Copy, Clone)]
pub enum Role {
    Observer,
    PlayerPong,
    PlayerDisplay,
    AdminPong,
    AdminDisplay,
}

impl FromStr for Role {
    type Err = ();

    fn from_str(s: &str) -> Result<Role, ()> {
        match s {
            "PlayerPong" => Ok(Role::PlayerPong),
            "PlayerDisplay" => Ok(Role::PlayerDisplay),
            "AdminPong" => Ok(Role::AdminPong),
            "AdminDisplay" => Ok(Role::AdminDisplay),
            _ => Ok(Role::Observer),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Item, Clone)]
pub struct Connection {
    #[dynomite(partition_key)]
    pub id: String,
    pub role: Option<Role>,
    pub que: bool,
}

#[derive(Serialize, Deserialize, Debug, Item, Clone)]
pub struct UnresolvedConnection {
    #[dynomite(partition_key)]
    pub id: String,
}
