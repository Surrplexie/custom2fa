use create::account::Account;
use st::fs;
use std::path::Path;

pub fn save_accounts(path: &Path, accounts: &[Account]) {
    let data = serde_json::to_string(accounts).unwrap();
    fs::write(path, data).unwrap();
}

pub fn load_accounts(path: &Path) -> Vec<Account> {
    if !path.exists() {
        return vec![];
    }
    let data = fs::read_to_string(path).unwrap();
    serde_json::from_str(&data).unwrap()
}

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)] // Added derives
pub struct Account {
    pub issuer: String,
    pub label: String,
    pub secret: Vec<u8>,
}