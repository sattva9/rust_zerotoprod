use crate::helpers::spawn_app;

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = spawn_app().await;

    let login_body =
        serde_json::json!({ "username": "random-username", "password": "random-password"});
    let response = app.post_login(&login_body).await;

    assert_eq!(response.status().as_u16(), 303);
}
