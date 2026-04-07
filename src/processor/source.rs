/// Image source type — either a remote HTTP URL or an S3 object key.
#[derive(Debug)]
pub enum ImageSource {
    Url(String),
    S3(String),
}
