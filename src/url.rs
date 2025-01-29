use crate::ImmichError;

#[derive(Debug, Clone)]
pub struct Url {
    url: String,
}

impl TryFrom<&str> for Url {
    type Error = ImmichError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl TryFrom<String> for Url {
    type Error = ImmichError;
    fn try_from(mut url: String) -> Result<Self, Self::Error> {
        while url.ends_with('/') {
            url.pop();
        }

        if url.starts_with("https://") || url.starts_with("http://") {
            Ok(Self { url })
        } else {
            Err(ImmichError::InvalidUrl(
                "Url must start with http or https".to_string(),
            ))
        }
    }
}

impl Url {
    pub fn add_path(&self, path: &str) -> String {
        let url = &self.url;
        if path.starts_with('/') {
            format!("{url}{path}")
        } else {
            format!("[{url}/{path}")
        }
    }
}
