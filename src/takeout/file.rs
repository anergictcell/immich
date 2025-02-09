use std::{borrow::Cow, fs::File, path::Path};

use flate2::read::GzDecoder;
use tar::Entry;

use crate::takeout::ParseError;

#[derive(Eq, Hash, PartialEq)]
pub(crate) struct Filename {
    name: String,
    album: String,
    filetype: FileType,
}

impl Filename {
    const MEDIA_EXTENSIONS: [&'static str; 10] = [
        "jpg", "jpeg", "png", "webp", "heic", "mp4", "m4v", "webm", "3gp", "gif",
    ];

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn album(&self) -> &str {
        &self.album
    }

    pub fn filetype(&self) -> &FileType {
        &self.filetype
    }

    pub fn normalize_duplicates(name: &mut String) {
        // this happens only for metadata
        // IMG_20131023_123651(1).jpg
        // IMG_20131023_123651.jpg(1).json ==> The .json extension is already removed
        // IMG_20131023_123651.jpg(1)

        if name.ends_with(')') {
            if let Some(idx) = name.rfind('(') {
                let number = name.split_off(idx);

                if let Some(idx) = name.rfind('.') {
                    name.insert_str(idx, &number);
                }
            }
        }
    }
}

impl TryFrom<&Entry<'_, GzDecoder<File>>> for Filename {
    type Error = ParseError;
    fn try_from(entry: &Entry<GzDecoder<File>>) -> Result<Self, Self::Error> {
        let path = entry.path()?;

        let filetype = FileType::try_from(&path)?;

        let album = path
            .parent()
            .ok_or(ParseError::FilePathError(
                "Asset path must contain an album name".to_string(),
            ))?
            .file_name()
            .ok_or(ParseError::FilePathError(
                "Asset path must contain an album name".to_string(),
            ))?
            .to_string_lossy()
            .to_string();

        let mut name = path
            .file_name()
            .ok_or(ParseError::FilePathError(
                "Asset path must contain a filename".to_string(),
            ))?
            .to_string_lossy()
            .replace("-edited", "")
            .replace(".supplemental-metadata", "")
            .replace(".json", "");

        Self::normalize_duplicates(&mut name);

        Ok(Self {
            album,
            name,
            filetype,
        })
    }
}

#[derive(Eq, Hash, PartialEq)]
pub(crate) enum FileType {
    Metadata,
    Original,
    Edited,
    Unknown,
}

impl TryFrom<&Cow<'_, Path>> for FileType {
    type Error = ParseError;

    fn try_from(path: &Cow<'_, Path>) -> Result<Self, Self::Error> {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            if &ext == "json" {
                Ok(Self::Metadata)
            } else if Filename::MEDIA_EXTENSIONS.contains(&ext.as_str()) {
                let filename = path
                    .file_name()
                    .ok_or(ParseError::FilePathError(
                        "Asset path must contain a filename".to_string(),
                    ))?
                    .to_string_lossy();
                if filename.contains("edited") {
                    Ok(Self::Edited)
                } else {
                    Ok(Self::Original)
                }
            } else {
                Ok(Self::Unknown)
            }
        } else {
            Err(ParseError::FileNameError(
                "File does not have an extension".to_string(),
            ))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_duplicates() {
        let mut s = "IMG_20131023_123651.jpg(1)".to_string();
        Filename::normalize_duplicates(&mut s);
        assert_eq!(&s, "IMG_20131023_123651(1).jpg");

        let mut s = "IMG_20131023_123627.jpg(1)".to_string();
        Filename::normalize_duplicates(&mut s);
        assert_eq!(&s, "IMG_20131023_123627(1).jpg");

        let mut s = "IMG_20131023_123651.jpg(9)".to_string();
        Filename::normalize_duplicates(&mut s);
        assert_eq!(&s, "IMG_20131023_123651(9).jpg");

        let mut s = "IMG_20131023_123627.jpg(9)".to_string();
        Filename::normalize_duplicates(&mut s);
        assert_eq!(&s, "IMG_20131023_123627(9).jpg");

        let mut s = "IMG_20131023_123651.jpg(11)".to_string();
        Filename::normalize_duplicates(&mut s);
        assert_eq!(&s, "IMG_20131023_123651(11).jpg");

        let mut s = "IMG_20131023_123627.jpg(12)".to_string();
        Filename::normalize_duplicates(&mut s);
        assert_eq!(&s, "IMG_20131023_123627(12).jpg");

        let mut s = "IMG_20131023_123627.jpg".to_string();
        Filename::normalize_duplicates(&mut s);
        assert_eq!(&s, "IMG_20131023_123627.jpg");

        let mut s = "IMG_20131023_123627(1).jpg".to_string();
        Filename::normalize_duplicates(&mut s);
        assert_eq!(&s, "IMG_20131023_123627(1).jpg");
    }
}
