use std::time::Duration;

use reqwest::{header, Client, ClientBuilder, Url};
use secrecy::ExposeSecret;
use serde::Serialize;

use crate::configuration::EmailSettings;

use crate::domain::Subscriber;

pub struct EmailClient {
    http_client: Client,
    url: Url,
    sender: Subscriber,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendEmailRequest<'a> {
    sender: &'a Subscriber,
    to: Vec<&'a Subscriber>,
    subject: &'a str,
    html_content: &'a str,
}

impl EmailClient {
    pub fn new(config: EmailSettings) -> anyhow::Result<Self> {
        let url = Url::parse(&config.endpoint).map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mut headers = header::HeaderMap::new();
        headers.insert(
            "api-key",
            config
                .api_key
                .expose_secret()
                .parse()
                .map_err(|e| anyhow::anyhow!("Failed tp parse api_key: {e}"))?,
        );
        headers.insert(
            header::ACCEPT,
            "application/json"
                .parse()
                .map_err(|e| anyhow::anyhow!("Failed tp parse api_key: {e}"))?,
        );

        let http_client = ClientBuilder::new()
            .default_headers(headers)
            .timeout(Duration::from_millis(config.timeout_millis))
            .build()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        Ok(Self {
            http_client,
            url,
            sender: config.sender,
        })
    }

    pub async fn send_email(
        &self,
        recipent: &Subscriber,
        subject: &str,
        html_content: &str,
    ) -> anyhow::Result<()> {
        let url = self
            .url
            .join("/v3/smtp/email")
            .map_err(|e| anyhow::anyhow!(e))?;
        let body = SendEmailRequest {
            sender: &self.sender,
            to: vec![recipent],
            subject,
            html_content,
        };

        let _ = self
            .http_client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to send a confirmation email for {}. {e}",
                    recipent.email
                )
            })?
            .error_for_status()
            .map_err(|e| {
                anyhow::anyhow!(
                    "Error while sending a confirmation email for {}. {e}",
                    recipent.email
                )
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::EmailClient;
    use crate::configuration::EmailSettings;
    use crate::domain::{Subscriber, SubscriberEmail, SubscriberName};
    use claims::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::faker::name::en::FirstName;
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use serde_json::Value;
    use wiremock::matchers::{any, header, header_exists, method};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result = serde_json::from_slice::<Value>(&request.body);
            if let Ok(body) = result {
                body.get("sender").is_some()
                    && body.get("to").is_some()
                    && body.get("subject").is_some()
                    && body.get("htmlContent").is_some()
            } else {
                false
            }
        }
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }
    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn name() -> SubscriberName {
        SubscriberName::parse(FirstName().fake()).unwrap()
    }

    fn email_settings(base_url: String) -> EmailSettings {
        EmailSettings {
            endpoint: base_url,
            api_key: Secret::new(Faker.fake()),
            sender: Subscriber {
                name: name(),
                email: email(),
            },
            timeout_millis: 10,
        }
    }

    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(email_settings(base_url)).unwrap()
    }

    fn subscriber() -> Subscriber {
        Subscriber {
            name: SubscriberName::parse(FirstName().fake()).unwrap(),
            email: SubscriberEmail::parse(SafeEmail().fake()).unwrap(),
        }
    }

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("api-key"))
            .and(header("Content-Type", "application/json"))
            .and(header("accept", "application/json"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = email_client
            .send_email(&subscriber(), &subject(), &content())
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&subscriber(), &subject(), &content())
            .await;
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&subscriber(), &subject(), &content())
            .await;
        assert_err!(outcome);
    }
}
