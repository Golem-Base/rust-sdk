use crate::entity::types::content_type::ContentTypeValidationError;

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum ValidationError {
    #[error(transparent)]
    ContentType(#[from] ContentTypeValidationError),

    #[error("failed to convert string to MIME: {0}")]
    MimeFromStr(String),

    #[error("`BTL` must be a non-zero `u64`.")]
    InvalidBtl,
}
