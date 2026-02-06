use std::sync::Arc;

use preflight_core::highlight::Highlighter;
use preflight_core::store::ReviewStore;
use tokio::sync::broadcast;

use crate::ws::WsEvent;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn ReviewStore>,
    pub highlighter: Arc<Highlighter>,
    pub ws_tx: broadcast::Sender<WsEvent>,
}
