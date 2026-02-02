use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let app = preflight_server::app();
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
