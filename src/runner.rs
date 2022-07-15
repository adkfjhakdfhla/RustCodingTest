use async_trait::async_trait;
use serde::Deserialize;
use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::io;
use thiserror::Error;

use crate::logger::Logger;
use crate::processor::{
    ChargebackProcessor, DepositProcessor, DisputeProcessor, Processor, ResolveProcessor,
    WithdrawalProcessor,
};
use crate::store::{Client, Store, StoreError};

#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("{0}")]
    StoreError(#[from] StoreError),
    #[error("Input file could not be opened")]
    FileError,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum EventType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Deserialize, Debug)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<f64>,
}

#[async_trait]
pub trait Runner {
    async fn run(&mut self) -> Result<(), RunnerError>;
}

pub struct CsvSingleProcessRunner<S: Store + Send + Sync, L: Logger> {
    input_file: String,
    store: S,
    logger: L,
}

impl<S: Store + Default + Send + Sync, L: Logger + Default> CsvSingleProcessRunner<S, L> {
    pub fn new(input_file: &str) -> Self {
        Self {
            input_file: input_file.to_owned(),
            store: S::default(),
            logger: L::default(),
        }
    }
}

#[async_trait]
impl<S: Store + Send + Sync, L: Logger + Send + Sync> Runner for CsvSingleProcessRunner<S, L> {
    async fn run(&mut self) -> Result<(), RunnerError> {
        let mut rdr = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_path(&self.input_file)
            .or(Err(RunnerError::FileError))?;
        for result in rdr.deserialize::<Event>() {
            if let Some(event) = result.as_ref().ok() {
                let maybe_tx = self.store.get_transaction(event.tx);
                let maybe_client = self.store.get_client(event.client);
                let result = match event.event_type {
                    EventType::Deposit => DepositProcessor::process_event,
                    EventType::Withdrawal => WithdrawalProcessor::process_event,
                    EventType::Dispute => DisputeProcessor::process_event,
                    EventType::Resolve => ResolveProcessor::process_event,
                    EventType::Chargeback => ChargebackProcessor::process_event,
                }(maybe_tx, maybe_client, event)
                .await;
                match result {
                    Err(e) => self.logger.error(e.to_string()),
                    Ok((client, transaction)) => {
                        self.store.set_transaction(transaction)?;
                        self.store.set_client(client)?;
                    }
                }
            } else if let Some(e) = result.err() {
                self.logger.error(e.to_string())
            }
        }

        let mut wtr = csv::Writer::from_writer(io::stdout());
        for client in self.store.dump_clients() {
            if let Err(e) = wtr.serialize(client) {
                self.logger.error(e.to_string());
            }
        }
        Ok(())
    }
}

impl Serialize for Client {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Client", 5)?;
        state.serialize_field("client", &self.id)?;
        state.serialize_field("available", &self.available)?;
        state.serialize_field("held", &self.held)?;
        state.serialize_field("total", &(&self.available + &self.held))?; // WARN: assumes no overflow
        state.serialize_field("locked", &self.locked)?;
        state.end()
    }
}
