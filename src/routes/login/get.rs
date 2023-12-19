use axum::response::{Html, IntoResponse, Response};
use axum_extra::extract::CookieJar;

pub async fn login_form(cookie_jar: CookieJar) -> Response {
    let error_html = match cookie_jar.get("_flash") {
        None => "".into(),
        Some(cookie) => {
            format!("<p><i>{}</i></p>", cookie.value())
        }
    };

    let body = format!(
        r#"<!doctype html>
    <html lang="en">
        <head>
            <meta http-equiv="content-type" content="text/html; charset=utf-8" />
            <title>Login</title>
        </head>
        <body>
            {error_html}
            <form action="/login" method="post">
                <label>
                    Username
                    <input
                        type="text"
                        placeholder="Enter Username"
                        name="username"
                    />
                </label>
                <label>
                    Password
                    <input
                        type="password"
                        placeholder="Enter Password"
                        name="password"
                    />
                </label>
                <button type="submit">Login</button>
            </form>
        </body>
    </html>
"#
    );
    Html(body).into_response()
}
