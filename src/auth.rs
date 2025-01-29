use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Deserialize)]
pub(super) struct Login {
    pub accessToken: String,
}

#[derive(Debug, Clone)]
pub(crate) enum Authenticated {
    Cookie(String),
    ApiKey(String),
}

impl Authenticated {
    pub fn header(&self) -> (&str, &str) {
        match self {
            Authenticated::Cookie(cookie) => ("Cookie", cookie),
            Authenticated::ApiKey(key) => ("x-api-key", key),
        }
    }
}
