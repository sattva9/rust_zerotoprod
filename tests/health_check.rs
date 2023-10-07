use std::net::TcpListener;

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().expect("local addr not found").port();
    let server = zerotoprod::run(listener).expect("Failed to bind address");
    tokio::spawn(server);
    format!("127.0.0.1:{port}")
}

#[tokio::test]
async fn health_check_test() {
    let address = spawn_app();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{address}/health_check"))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app_address = spawn_app();
    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(format!("http://{app_address}/subscriptions"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app_address = spawn_app();
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email")
    ];
    let url = format!("http://{app_address}/subscriptions");

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");
        assert_eq!(422, response.status().as_u16(), "The API did not fail with 400 Bad Request when the payload was {}.", error_message);
    }
}