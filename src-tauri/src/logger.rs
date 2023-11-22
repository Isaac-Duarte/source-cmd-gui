use log::{Record, Level, Metadata, LevelFilter};
use std::sync::Mutex;
use tokio::sync::mpsc;

struct StdoutLogger {
    sender: Mutex<mpsc::Sender<String>>,
}

impl log::Log for StdoutLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let message = format!("{}", record.args());
            println!("{}", message); // Also print to console
            let _ = self.sender.lock().unwrap().try_send(message);
        }
    }

    fn flush(&self) {}
}

pub fn setup_logger(sender: mpsc::Sender<String>) {
    let logger = StdoutLogger { sender: Mutex::new(sender) };
    log::set_boxed_logger(Box::new(logger))
        .map(|()| log::set_max_level(LevelFilter::Info))
        .unwrap();
}