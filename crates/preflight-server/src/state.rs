use std::collections::HashMap;
use std::sync::Arc;

use preflight_core::highlight::Highlighter;
use preflight_core::review::AgentStatus;
use preflight_core::store::ReviewStore;
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

use crate::ws::WsEvent;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn ReviewStore>,
    pub highlighter: Arc<Highlighter>,
    pub ws_tx: broadcast::Sender<WsEvent>,
    pub agent_status: Arc<Mutex<HashMap<Uuid, AgentStatus>>>,
}
