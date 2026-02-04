use std::sync::Arc;

use preflight_core::highlight::Highlighter;
use preflight_core::store::ReviewStore;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn ReviewStore>,
    pub highlighter: Arc<Highlighter>,
}
