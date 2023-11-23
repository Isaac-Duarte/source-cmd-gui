use chrono::Local;
use log::{Level, LevelFilter, Metadata, Record};
use serde::Serialize;
use std::sync::Mutex;
use tokio::sync::mpsc;

#[derive(Debug, Serialize)]
pub struct Log {
    pub time_stamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

struct StdoutLogger {
    sender: Mutex<mpsc::Sender<Log>>,
}

impl log::Log for StdoutLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let now = Local::now().format("%Y-%m-%dT%H:%M:%S%.3fZ"); // Format the timestamp
            let level = record.level();
            let target = record.target();
            let message = format!("{} {} {} > {}", now, level, target, record.args());

            let log = Log {
                time_stamp: now.to_string(),
                level: level.to_string(),
                target: target.to_string(),
                message: record.args().to_string(),
            };

            println!("{}", message); // Print to console
            let _ = self.sender.lock().unwrap().try_send(log); // Send to channel
        }
    }

    fn flush(&self) {}
}
pub fn setup_logger(sender: mpsc::Sender<Log>) {
    let logger = StdoutLogger {
        sender: Mutex::new(sender),
    };
    log::set_boxed_logger(Box::new(logger))
        .map(|()| log::set_max_level(LevelFilter::Info))
        .unwrap();
}
