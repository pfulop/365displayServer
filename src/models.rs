use dynomite::{Attribute, Item};

#[derive(Attribute)]
pub enum Role {
    Observer,
    Player,
    Admin,
}

#[derive(Item)]
pub struct Connection {
    #[dynomite(partition_key)]
    pub id: String,
    pub role: Role,
}
