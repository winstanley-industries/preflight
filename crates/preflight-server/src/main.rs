use std::sync::Arc;

use preflight_core::json_store::JsonFileStore;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let store = JsonFileStore::new("preflight-state.json").await.unwrap();
    let app = preflight_server::app(Arc::new(store));
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
