pub trait Logger {
    fn error(&self, message: String) -> ();
}

#[derive(Default)]
pub struct StderrLogger;
impl Logger for StderrLogger {
    fn error(&self, message: String) -> () {
        eprintln!("{}", message);
    }
}

#[derive(Default)]
pub struct NoopLogger;
impl Logger for NoopLogger {
    fn error(&self, _message: String) -> () {
        ();
    }
}
