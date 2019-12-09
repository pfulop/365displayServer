use dynomite::dynamodb::{DeleteItemError, PutItemError};
use quick_error::*;
use rusoto_core::RusotoError;

quick_error! {
    #[derive(Debug)]
    pub enum ConnectionError {
        Connect(err: RusotoError<PutItemError>){from()}
        Disconnect(err: RusotoError<DeleteItemError>){from()}
        Default{from(std::env::VarError)}
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum ConnectionItemError {
        NoConnection{from()}
        WrongDirection{from()}
    }
}
