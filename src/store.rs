use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum StoreError {}

#[derive(Default, PartialEq, Debug, Clone)]
pub struct Transaction {
    pub id: u32,
    pub client: u16,
    pub amount: f64,
    pub disputed: bool,
}

#[derive(Default, PartialEq, Debug, Clone)]
pub struct Client {
    pub id: u16,
    pub available: f64,
    pub held: f64,
    pub locked: bool,
}

pub trait Store {
    fn get_client(&self, id: u16) -> Option<Client>;
    fn set_client(&mut self, client: Client) -> Result<(), StoreError>;

    fn get_transaction(&self, id: u32) -> Option<Transaction>;
    fn set_transaction(&mut self, transaction: Transaction) -> Result<(), StoreError>;

    fn dump_clients(&self) -> Vec<Client>;
    fn dump_transactions(&self) -> Vec<Transaction>;
}

#[derive(Default)]
pub struct InMemoryStore {
    clients: HashMap<u16, Client>,
    transactions: HashMap<u32, Transaction>,
}

impl Store for InMemoryStore {
    fn get_client(&self, id: u16) -> Option<Client> {
        self.clients.get(&id).cloned()
    }

    fn set_client(&mut self, client: Client) -> Result<(), StoreError> {
        let _ = &self.clients.insert(client.id, client);
        Ok(())
    }

    fn get_transaction(&self, id: u32) -> Option<Transaction> {
        self.transactions.get(&id).cloned()
    }

    fn set_transaction(&mut self, transaction: Transaction) -> Result<(), StoreError> {
        let _ = &self.transactions.insert(transaction.id, transaction);
        Ok(())
    }

    fn dump_clients(&self) -> Vec<Client> {
        self.clients.values().map(|client| client.clone()).collect()
    }

    fn dump_transactions(&self) -> Vec<Transaction> {
        self.transactions.values().map(|tx| tx.clone()).collect()
    }
}
