use crate::runner::Event;
use crate::store::{Client, StoreError, Transaction};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum ProcessorError {
    #[error("{0}")]
    StoreError(#[from] StoreError),
    #[error("Attempted processing of transaction that has already been processed")]
    TransactionExists,
    #[error("Amount not specified for transaction")]
    NoAmount,
    #[error("Attempted to deposit or withdraw on locked client account")]
    ClientLocked,
    #[error("Withdrawal exceeds client withdrawable (free) balance")]
    WithdrawalAboveBalance,
    #[error("Dispute or withdrawal refers to nonexistent client")]
    ClientMissing,
    #[error("Client and transaction do not match in alleged dispute")]
    ClientTransactionMismatch,
    #[error("Dispute refers to nonexistent transaction")]
    TransactionMissing,
    #[error("Attempted to open duplicate dispute on transaction")]
    TransactionDisputed,
    #[error("Attempted to open dispute on withdrawal transaction")]
    WithdrawalNotDisputable,
    #[error("Attempted to close a dispute on a non-disputed transaction")]
    TransactionNotDisputed,
}

#[async_trait]
pub trait Processor {
    async fn process_event(
        maybe_tx: Option<Transaction>,
        maybe_client: Option<Client>,
        event: &Event,
    ) -> Result<(Client, Transaction), ProcessorError>;
}

pub struct DepositProcessor;
pub struct WithdrawalProcessor;
pub struct DisputeProcessor;
pub struct ResolveProcessor;
pub struct ChargebackProcessor;

#[async_trait]
impl Processor for DepositProcessor {
    async fn process_event(
        maybe_tx: Option<Transaction>,
        maybe_client: Option<Client>,
        event: &Event,
    ) -> Result<(Client, Transaction), ProcessorError> {
        // PRECONDITION: transaction must be unique
        if let Some(_) = maybe_tx {
            return Err(ProcessorError::TransactionExists);
        }
        // PRECONDITION: client must not be locked
        if maybe_client.is_some_with(|client| client.locked) {
            return Err(ProcessorError::ClientLocked);
        }
        // PRECONDITION: event must have an amount
        let amount = match event.amount {
            None => return Err(ProcessorError::NoAmount),
            Some(amount) => amount,
        };
        // OK
        // POSTCONDITION: client saved with new value (or inserted if did not exist)
        let mut client = maybe_client.unwrap_or(Client::default());
        client.id = event.client; // in case it was a new client
        client.available += amount;
        // POSTCONDITION: new transaction created
        let tx = Transaction {
            id: event.tx,
            client: event.client,
            amount,
            disputed: false,
        };
        Ok((client, tx))
    }
}

#[async_trait]
impl Processor for WithdrawalProcessor {
    async fn process_event(
        maybe_tx: Option<Transaction>,
        maybe_client: Option<Client>,
        event: &Event,
    ) -> Result<(Client, Transaction), ProcessorError> {
        // PRECONDITION: transaction must be unique
        if let Some(_) = maybe_tx {
            return Err(ProcessorError::TransactionExists);
        }
        // PRECONDITION: event must have an amount
        let amount = match event.amount {
            None => return Err(ProcessorError::NoAmount),
            Some(amount) => amount,
        };
        // PRECONDITION: client must exist
        let mut client = match maybe_client {
            None => return Err(ProcessorError::ClientMissing),
            Some(client) => client,
        };
        // PRECONDITION: client must not be locked
        if client.locked {
            return Err(ProcessorError::ClientLocked);
        }
        // PRECONDITION: withdrawal must not exceed client balance
        if amount > client.available {
            return Err(ProcessorError::WithdrawalAboveBalance);
        }
        // OK
        // POSTCONDITION: client available balanced reduced
        client.available -= amount;
        // POSTCONDITION: new transaction created
        let tx = Transaction {
            id: event.tx,
            client: event.client,
            amount: amount * -1f64,
            disputed: false,
        };
        Ok((client, tx))
    }
}

#[async_trait]
impl Processor for DisputeProcessor {
    async fn process_event(
        maybe_tx: Option<Transaction>,
        maybe_client: Option<Client>,
        _event: &Event,
    ) -> Result<(Client, Transaction), ProcessorError> {
        // PRECONDITION: transaction must exist
        let mut tx = match maybe_tx {
            None => return Err(ProcessorError::TransactionMissing),
            Some(tx) => tx,
        };
        // PRECONDITION: transaction must not be under dispute
        if tx.disputed {
            return Err(ProcessorError::TransactionDisputed);
        }
        // PRECONDITION: transaction must not have been a withdrawal
        if tx.amount < 0f64 {
            return Err(ProcessorError::WithdrawalNotDisputable);
        }
        // PRECONDITION: client must exist
        let mut client = match maybe_client {
            None => return Err(ProcessorError::ClientMissing),
            Some(client) => client,
        };
        // PRECONDITION: client must match tx
        if tx.client != client.id {
            return Err(ProcessorError::ClientTransactionMismatch);
        }
        // OK
        // POSTCONDITION: client funds are held, to maximum extent
        client.available -= tx.amount;
        client.held += tx.amount;
        // POSTCONDITION: transaction is marked as currently disputed
        tx.disputed = true;
        Ok((client, tx))
    }
}

#[async_trait]
impl Processor for ResolveProcessor {
    async fn process_event(
        maybe_tx: Option<Transaction>,
        maybe_client: Option<Client>,
        _event: &Event,
    ) -> Result<(Client, Transaction), ProcessorError> {
        // PRECONDITION: transaction must exist
        let mut tx = match maybe_tx {
            None => return Err(ProcessorError::TransactionMissing),
            Some(tx) => tx,
        };
        // PRECONDITION: transaction must be under dispute
        if !tx.disputed {
            return Err(ProcessorError::TransactionNotDisputed);
        }
        // PRECONDITION: client must exist
        let mut client = match maybe_client {
            None => return Err(ProcessorError::ClientMissing),
            Some(client) => client,
        };
        // PRECONDITION: client must match tx
        if tx.client != client.id {
            return Err(ProcessorError::ClientTransactionMismatch);
        }
        // OK
        // POSTCONDITION: client held funds from the dispute are released
        client.available += tx.amount;
        client.held -= tx.amount;
        // POSTCONDITION: transaction is no longer under dispute
        tx.disputed = false;
        Ok((client, tx))
    }
}

#[async_trait]
impl Processor for ChargebackProcessor {
    async fn process_event(
        maybe_tx: Option<Transaction>,
        maybe_client: Option<Client>,
        _event: &Event,
    ) -> Result<(Client, Transaction), ProcessorError> {
        // PRECONDITION: transaction must exist
        let mut tx = match maybe_tx {
            None => return Err(ProcessorError::TransactionMissing),
            Some(tx) => tx,
        };
        // PRECONDITION: transaction must be under dispute
        if !tx.disputed {
            return Err(ProcessorError::TransactionNotDisputed);
        }
        // PRECONDITION: client must exist
        let mut client = match maybe_client {
            None => return Err(ProcessorError::ClientMissing),
            Some(client) => client,
        };
        // PRECONDITION: client must match tx
        if tx.client != client.id {
            return Err(ProcessorError::ClientTransactionMismatch);
        }
        // OK
        // POSTCONDITION: client held funds are removed from the client
        client.held -= tx.amount;
        // POSTCONDITION: client account is frozen
        client.locked = true;
        // POSTCONDITION: transaction is no longer under dispute
        tx.disputed = false;
        Ok((client, tx))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod deposit_test {
        use crate::runner::EventType;

        use super::*;

        fn default_event(event_type: EventType) -> Event {
            Event {
                event_type,
                client: 0,
                tx: 0,
                amount: Some(0f64),
            }
        }

        #[tokio::test]
        async fn deposit_fails_if_transaction_exists() {
            let result = DepositProcessor::process_event(
                Some(Transaction::default()),
                None,
                &default_event(EventType::Deposit),
            )
            .await;
            assert!(result.contains_err(&ProcessorError::TransactionExists));
        }

        #[tokio::test]
        async fn deposit_fails_for_locked_client() {
            let mut client = Client::default();
            client.locked = true;
            let result = DepositProcessor::process_event(
                None,
                Some(client),
                &default_event(EventType::Deposit),
            )
            .await;
            assert!(result.contains_err(&ProcessorError::ClientLocked));
        }

        #[tokio::test]
        async fn deposit_fails_without_amount() {
            let mut event = default_event(EventType::Deposit);
            event.amount = None;
            let result = DepositProcessor::process_event(None, None, &event).await;
            assert!(result.contains_err(&ProcessorError::NoAmount));
        }

        #[tokio::test]
        async fn deposit_succeeds_with_no_client() {
            let mut event = default_event(EventType::Deposit);
            let amount = 1f64;
            event.amount = Some(amount);
            let result = DepositProcessor::process_event(None, None, &event).await;
            assert!(result.is_ok());
            let (client, tx) = result.unwrap();
            let expected_client = Client {
                id: 0,
                available: amount,
                held: 0f64,
                locked: false,
            };
            let expected_tx = Transaction {
                id: 0,
                amount,
                client: 0,
                disputed: false,
            };
            assert_eq!(client, expected_client);
            assert_eq!(tx, expected_tx);
        }

        #[tokio::test]
        async fn deposit_succeeds_with_existing_client() {
            let mut event = default_event(EventType::Deposit);
            let amount = 1f64;
            let initial_client = Client {
                id: 1,
                available: 2f64,
                held: 0f64,
                locked: false,
            };

            event.client = 1;
            event.amount = Some(amount);
            let result =
                DepositProcessor::process_event(None, Some(initial_client.clone()), &event).await;
            assert!(result.is_ok());
            let (client, tx) = result.unwrap();
            let expected_client = Client {
                id: 1,
                available: initial_client.available + amount,
                held: 0f64,
                locked: false,
            };
            let expected_tx = Transaction {
                id: 0,
                amount,
                client: 1,
                disputed: false,
            };
            assert_eq!(client, expected_client);
            assert_eq!(tx, expected_tx);
        }
    }

    mod withdrawal_test {
        // TODO
    }
    mod dispute_test {
        // TODO
    }
    mod resolve_test {
        // TODO
    }
    mod chargeback_test {
        // TODO
    }
}
