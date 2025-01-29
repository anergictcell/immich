use crate::client::ImmichClient;
use ureq::json;

use crate::auth::{Authenticated, Login};
use crate::{url::Url, Client};
use crate::{ImmichError, ImmichResult};

pub(crate) struct Host {
    url: Url,
}

impl Host {
    pub fn new<T: TryInto<Url>>(url: T) -> ImmichResult<Self>
    where
        ImmichError: From<<T>::Error>,
    {
        Ok(Self {
            url: url.try_into()?,
        })
    }

    pub fn email(self, username: &str, password: &str) -> ImmichResult<Client> {
        let response = ureq::post(&self.url.add_path("/auth/login"))
            .add_default_header()
            .send_json(json!({
                "email": username,
                "password": password,
            }))?;

        #[allow(clippy::to_string_in_format_args)]
        if response.status() == 201 {
            let login: Login = response.into_json()?;
            let auth = Authenticated::Cookie(format!("immich_access_token={}", login.accessToken));
            Ok(Client::new(self.url, auth))
        } else {
            println!(
                "Response code: {}: {}\n\n{}",
                response.status(),
                response.status_text().to_string(),
                response
                    .into_string()
                    .unwrap_or("Unreadable error message".to_string())
            );
            Err(ImmichError::Auth)
        }
    }

    pub fn key(self, key: &str) -> ImmichResult<Client> {
        let auth = Authenticated::ApiKey(key.to_string());
        let client = Client::new(self.url, auth);
        if client.check_auth() {
            Ok(client)
        } else {
            Err(ImmichError::Auth)
        }
    }
}
