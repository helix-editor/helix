use std::sync::{Arc, Mutex};

use helix_view::theme::Loader;
use log;

use crate::{path, DynError};
use once_cell::sync::Lazy;

static LOGGER: Lazy<MockLog> = Lazy::new(|| MockLog::new());

pub fn theme_check() -> Result<(), DynError> {
    log::set_logger(&*LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Warn);

    let theme_names = Loader::read_names(&path::themes());
    let loader = Loader::new(&vec![path::project_root().join("runtime")]);

    let mut issues_found = false;
    for name in theme_names {
        let _ = loader.load(&name).unwrap();

        {
            let warnings = LOGGER.warnings.lock().unwrap();
            if !warnings.is_empty() {
                issues_found = true;

                println!("Theme '{name}' loaded with warnings:");
                for warning in warnings.iter() {
                    println!("{warning}");
                }
            }
        }

        LOGGER.clear();
    }
    match issues_found {
        true => Err("Issues found in bundled themes".to_string().into()),
        false => Ok(()),
    }
}

struct MockLog {
    warnings: Arc<Mutex<Vec<String>>>,
}

impl MockLog {
    pub fn new() -> Self {
        MockLog {
            warnings: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn clear(&self) {
        let mut warnings = self.warnings.lock().unwrap();
        warnings.clear();
    }
}

impl log::Log for MockLog {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut warnings = self.warnings.lock().unwrap();
        warnings.push(record.args().to_string());
    }

    fn flush(&self) { // Do nothing
    }
}
