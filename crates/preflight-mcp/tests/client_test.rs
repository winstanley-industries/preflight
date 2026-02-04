use preflight_mcp::client::PreflightClient;

#[test]
fn test_client_builds_with_default_port() {
    let client = PreflightClient::new(3000);
    assert_eq!(client.base_url(), "http://127.0.0.1:3000");
}

#[test]
fn test_client_builds_with_custom_port() {
    let client = PreflightClient::new(3001);
    assert_eq!(client.base_url(), "http://127.0.0.1:3001");
}
