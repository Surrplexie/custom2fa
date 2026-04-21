use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    pub issuer: String,
    pub label: String,
    pub secret: Vec<u8>,
}