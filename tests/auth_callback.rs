#[test]
fn parse_token_query_sets_token() {
    // This test only validates that no panics occur and the setter path runs.
    // We don't assert on token content because global state is not resettable here.
    nearx::auth::handle_auth_callback_query("token=abc123&foo=bar");
    assert!(nearx::auth::has_token());
}

#[test]
fn parse_code_query_does_not_panic() {
    // In native tests this is a no-op; on wasm it would attempt an exchange via JS.
    nearx::auth::handle_auth_callback_query("code=xyz&state=s1");
    // No assertion; just ensure handler is resilient.
}
