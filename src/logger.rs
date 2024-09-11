
pub struct Logger;
pub static LOGGER: Logger = Logger;

impl log::Log for Logger {
  fn enabled(&self, metadata: &log::Metadata) -> bool {
    metadata.level() <= log::Level::Info
  }

  fn log(&self, record: &log::Record) {
    if self.enabled(record.metadata()) {
      let filename = record.file().unwrap_or("unknown").split("/").last().unwrap_or("unknown");
      println!("[{}] [{}] - {}", record.level(), filename, record.args());
    }
  }

  fn flush(&self) {}
}

