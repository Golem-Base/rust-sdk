use crate::entity::types::content_type::ContentTypeValidationError;

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum ValidationError {
    #[error(transparent)]
    ContentType(#[from] ContentTypeValidationError),

    #[error("failed to convert string to MIME: {0}")]
    MimeFromStr(String),

    #[error("Missing EntityKey")]
    MissingEntityKey,

    #[error("Missing BlocksToLive")]
    MissingBtl,

    #[error("Missing ContentType")]
    MissingContentType,

    #[error("Missing Payload")]
    MissingPayload,
}
