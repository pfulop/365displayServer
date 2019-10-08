use dynomite::{
    Item,
    Attribute
};

#[derive(Attribute)]
pub enum Role {
    Observer,
    Player,
    Admin
}

#[derive(Item)]
pub struct Connection {
    #[hash]
    id: String,
    role: Role
}