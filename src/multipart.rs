use std::io;
/// adapted from https://crates.io/crates/ureq_multipart
use std::io::Write;

#[derive(Debug)]
pub struct MultipartBuilder {
    boundary: String,
    inner: Vec<u8>,
    data_written: bool,
}
impl Default for MultipartBuilder {
    fn default() -> Self {
        Self::new()
    }
}
#[allow(dead_code)]
impl MultipartBuilder {
    pub fn new() -> Self {
        Self {
            boundary: "IMMICHCLIENTMULTIPARTUPLOADBOUND".to_string(),
            inner: Vec::new(),
            data_written: false,
        }
    }
    /// add text field
    ///
    /// * name field name
    /// * text field text value
    pub fn add_text(mut self, name: &str, text: &str) -> io::Result<Self> {
        self.write_field_headers(name, None, None)?;
        self.inner.write_all(text.as_bytes())?;
        Ok(self)
    }

    pub fn add_bytes(
        mut self,
        bytes: &[u8],
        name: &str,
        filename: Option<&str>,
    ) -> io::Result<Self> {
        // This is necessary to make sure it is interpreted as a file on the server end.
        // let content_type = Some("application/octet-stream");
        self.write_field_headers(name, filename, None)?;
        self.inner.write_all(bytes)?;
        Ok(self)
    }

    fn write_boundary(&mut self) -> io::Result<()> {
        if self.data_written {
            self.inner.write_all(b"\r\n")?;
        }

        write!(self.inner, "--{}\r\n", self.boundary)
    }
    fn write_field_headers(
        &mut self,
        name: &str,
        filename: Option<&str>,
        content_type: Option<&str>,
    ) -> io::Result<()> {
        self.write_boundary()?;
        if !self.data_written {
            self.data_written = true;
        }
        write!(
            self.inner,
            "Content-Disposition: form-data; name=\"{name}\""
        )?;
        if let Some(filename) = filename {
            write!(self.inner, "; filename=\"{filename}\"")?;
        }
        if let Some(content_type) = content_type {
            write!(self.inner, "\r\nContent-Type: {content_type}")?;
        }
        self.inner.write_all(b"\r\n\r\n")
    }
    /// general multipart data
    ///
    /// # Return
    /// * (content_type,post_data)
    ///    * content_type http header content type
    ///    * post_data ureq.req.send_send_bytes(&post_data)
    ///
    pub fn finish(mut self) -> io::Result<(String, Vec<u8>)> {
        if self.data_written {
            self.inner.write_all(b"\r\n")?;
        }

        // always write the closing boundary, even for empty bodies
        write!(self.inner, "--{}--\r\n", self.boundary)?;
        Ok((
            format!("multipart/form-data; boundary={}", self.boundary),
            self.inner,
        ))
    }
}
