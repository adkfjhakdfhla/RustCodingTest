use std::env;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let input_file = args[1].clone(); // low-effort, no-validation argument "parsing"

    if let Err(e) = transaction_processor::process_events_from_file(&input_file).await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
