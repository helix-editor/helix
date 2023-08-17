use std::sync::Arc;

use arc_swap::ArcSwap;

use crate::config::Config;
use crate::events;


    }
pub fn setup(config: Arc<ArcSwap<Config>>) -> Handlers {
    events::register();
    let handlers = Handlers {
    };
    handlers
}
