use reqwest::Url;
use serde::{de::Visitor, Deserialize, Deserializer};

#[derive(Debug, Clone)]
pub struct ApplicationBaseUrl(Url);

impl AsRef<Url> for ApplicationBaseUrl {
    fn as_ref(&self) -> &Url {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ApplicationBaseUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ApplicationBaseUrlVisitor;

        impl<'de> Visitor<'de> for ApplicationBaseUrlVisitor {
            type Value = ApplicationBaseUrl;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid base url string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match ApplicationBaseUrl::parse(value) {
                    Ok(base_url) => Ok(base_url),
                    Err(err) => Err(E::custom(err)),
                }
            }
        }

        deserializer.deserialize_str(ApplicationBaseUrlVisitor)
    }
}

impl ApplicationBaseUrl {
    pub fn parse(url: &str) -> Result<Self, String> {
        let url = Url::parse(url).map_err(|e| e.to_string())?;
        if !url.path().eq("/") {
            return Err(format!("expected base url. found: {url}"));
        }
        Ok(Self(url))
    }

    pub fn join(&self, path: &str) -> Result<Url, String> {
        self.0.join(path).map_err(|e| e.to_string())
    }
}
