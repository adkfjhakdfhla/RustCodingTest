#![feature(map_try_insert, is_some_with, result_contains_err)]

use logger::StderrLogger;
use runner::{CsvSingleProcessRunner, Runner, RunnerError};
use store::InMemoryStore;

pub mod logger;
pub mod processor;
pub mod runner;
pub mod store;

pub async fn process_events_from_file(input_file: &str) -> Result<(), RunnerError> {
    let mut runner = CsvSingleProcessRunner::<InMemoryStore, StderrLogger>::new(input_file);
    runner.run().await
}
