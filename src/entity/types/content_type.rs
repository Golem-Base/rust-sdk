// TODO: Move validation of mime into a separate type, only validate length of `ContentType`.
// TODO: Test with content type from hypr/reqwest, add these examples to docs
use crate::entity::error::ValidationError;

/// `ContentType` in this context refers to what is now called `MediaType`
/// but is more commonly referred to as `MIME`. This type is intended to be
/// as minimal as possible, only offering a layer of validation to the underlying
/// string itself. It's done this way to avoid unnecessary dependencies, but
/// also to allow flexibility of existing `MIME` crates, like `mime` or `mediatype`,
/// or any type that meets the trait constraints of `S`. Validation adheres
/// to RFC 2045 and RFC 7231 `MIME` type standards, as well as the requirements for the Arkiv network.
#[derive(Debug, Clone)]
pub struct ContentType<Mime: Into<String> + AsRef<str>>(pub(crate) Mime);

impl TryFrom<&str> for ContentType<String> {
    type Error = ValidationError;
    /// Create a new `ContentType<S>` at runtime. This will return an error
    /// if validation fails. If the content type is known at compile time, it's
    /// recommended to use `ContentType::new_static` to get compile time validation.
    fn try_from(source: &str) -> Result<Self, Self::Error> {
        ContentType(source.into()).validate()
    }
}
impl TryFrom<String> for ContentType<String> {
    type Error = ValidationError;
    /// Create a new `ContentType<S>` at runtime. This will return an error
    /// if validation fails. If the content type is known at compile time, it's
    /// recommended to use `ContentType::new_static` to get compile time validation.
    fn try_from(source: String) -> Result<Self, Self::Error> {
        ContentType(source).validate()
    }
}
/// The blanket implementation of `TryFrom` is not enough to cover conversion of
/// `ContentType<&str>` to `ContentType<String>` for compile time validated content types.
impl TryFrom<ContentType<&str>> for ContentType<String> {
    type Error = ValidationError;
    fn try_from(value: ContentType<&str>) -> Result<Self, Self::Error> {
        Ok(ContentType(value.0.into()))
    }
}

// TODO: Provide links to the RFCs
impl ContentType<&'static str> {
    /// Used for compile time validation of content types as `const` values.
    /// For a runtime validated `ContentType` use `TryFrom`.
    ///
    /// # Panics
    ///
    /// Panics if the source is not a valid `MIME` according to RFC 2045 and 7231.
    ///
    /// # Example
    ///
    /// ```rs,ignore
    /// use arkiv_sdk::entity::ContentType;
    ///
    /// pub const CUSTOM_CONTENT_TYPE: ContentType<&str> = ContentType::new("application/vnd.example.long-format+json;version=42;mode=fast;debug=true;region=us-west-2;retry=5");
    /// ```
    pub const fn new(source: &'static str) -> ContentType<&'static str> {
        match Self::validate_source(source) {
            Ok(()) => ContentType(source),
            Err(err) => panic!("{}", err.as_static_str()),
        }
    }
}
impl<Mime: Into<String> + AsRef<str>> ContentType<Mime> {
    /// A reference to the underlying source string `S`.
    pub fn source(&self) -> &str {
        &self.0.as_ref()
    }

    /// First checks the length, then iterates over the bytes of the string
    /// to determine the following in order:
    /// 1. Contains a type-subtype separator '/'
    /// 2. Contains a valid type
    /// 3. Contains a valid subtype
    ///
    /// Optionally validate parameters:
    /// 1. Contains parameter separator ';'
    /// 2. Contains a valid key
    /// 3. Contains key-value separator '='
    /// 4. Contains a valid value, optionally wrapped in double quotes
    const fn validate_source(source: &str) -> Result<(), ContentTypeValidationError> {
        if source.len() > 128 {
            return Err(ContentTypeValidationError::LengthExceeded);
        }

        let bytes = source.as_bytes();

        // We must first find the type-subtype separator prior to validating the type
        let mut type_sep_index = 0;
        let mut contains_invalid_char = false;
        while type_sep_index < bytes.len() && bytes[type_sep_index] != b'/' {
            if !Self::is_type_token(bytes[type_sep_index]) {
                contains_invalid_char = true;
            }
            type_sep_index += 1;
        }
        if type_sep_index == bytes.len() {
            return Err(ContentTypeValidationError::MissingTypeSeparator);
        }
        if type_sep_index == 0 {
            return Err(ContentTypeValidationError::MissingType);
        }
        if contains_invalid_char {
            return Err(ContentTypeValidationError::InvalidTypeChar);
        }

        // Parse the subtype
        let mut subtype_start = type_sep_index + 1;
        let mut err = Ok(());
        while subtype_start < bytes.len() && bytes[subtype_start] != b';' {
            let byte = bytes[subtype_start];
            if !Self::is_subtype_token(byte) && err.is_ok() {
                if byte == b'=' {
                    // More than likely this is a typo or missing semicolon
                    err = Err(ContentTypeValidationError::MissingParameterSeparator);
                } else {
                    err = Err(ContentTypeValidationError::InvalidSubtypeChar);
                }
            }
            subtype_start += 1;
        }
        if subtype_start == type_sep_index + 1 {
            return Err(ContentTypeValidationError::MissingSubtype);
        }
        if err.is_err() {
            return err;
        }

        // Validate params
        let mut param_index = subtype_start;
        while param_index < bytes.len() {
            // Find param separator
            if bytes[param_index] != b';' {
                return Err(ContentTypeValidationError::MissingParameterSeparator);
            }
            param_index += 1;
            while bytes[param_index] == b' ' {
                param_index += 1;
            }

            // Parse param key
            let key_start = param_index;
            while param_index < bytes.len() && bytes[param_index] != b'=' {
                if !Self::is_parameter_token(bytes[param_index]) {
                    return Err(ContentTypeValidationError::InvalidParameterKey);
                }
                param_index += 1;
            }

            if param_index == key_start || param_index >= bytes.len() {
                return Err(ContentTypeValidationError::MissingParameterAssignment);
            }

            param_index += 1; // Skip '='

            let mut quoted_value_pair = 0;
            if bytes[param_index] == b'"' {
                quoted_value_pair = 1;
                param_index += 1;
            }

            // Parse param value
            let val_start = param_index;
            while param_index < bytes.len() && bytes[param_index] != b';' {
                let byte = bytes[param_index];
                if byte == b'"' {
                    quoted_value_pair += 1;
                }
                if !Self::is_parameter_token(byte) && byte != b'"'
                    || (byte == b'"' && quoted_value_pair > 2)
                {
                    return Err(ContentTypeValidationError::InvalidParameterValue);
                }
                param_index += 1;
            }

            if param_index == val_start {
                return Err(ContentTypeValidationError::InvalidParameterValue);
            }
        }

        Ok(())
    }

    const fn is_type_token(byte: u8) -> bool {
        matches!(byte,
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'!' | b'#' | b'$' | b'&' |
            b'-' | b'^' | b'_' | b'.' | b'+'
        )
    }

    const fn is_subtype_token(byte: u8) -> bool {
        Self::is_type_token(byte)
    }

    const fn is_parameter_token(byte: u8) -> bool {
        matches!(byte,
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' |
            b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' |
            b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
        )
    }

    /// Return an error if validation fails.
    fn validate(self) -> Result<Self, ValidationError> {
        Self::validate_source(self.source()).map_err(ValidationError::from)?;

        Ok(self)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone, Copy)]
pub enum ContentTypeValidationError {
    #[error("`ContentType` exceeds maximum length of `[char; 128]`.")]
    LengthExceeded,

    #[error("`ContentType` missing type-subtype separator: '/'.")]
    MissingTypeSeparator,

    #[error("`ContentType` missing type before type-subtype separator: '/'.")]
    MissingType,

    #[error("`ContentType` type contains an invalid character")]
    InvalidTypeChar,

    #[error("`ContentType` missing subtype after type-subtype separator: '/'.")]
    MissingSubtype,

    #[error("`ContentType` subtype contains an invalid character")]
    InvalidSubtypeChar,

    #[error("`ContentType` missing parameter separator, expected ';'.")]
    MissingParameterSeparator,

    #[error("`ContentType` invalid character in parameter key.")]
    InvalidParameterKey,

    #[error("`ContentType` invalid parameter key or missing '='.")]
    MissingParameterAssignment,

    #[error("`ContentType` invalid character or empty parameter value.")]
    InvalidParameterValue,
}
impl ContentTypeValidationError {
    const fn as_static_str(&self) -> &'static str {
        match self {
            Self::LengthExceeded => "`ContentType` exceeds maximum length of `[char; 128]`.",
            Self::MissingTypeSeparator => "`ContentType` missing type-subtype separator: '/'.",
            Self::MissingType => "`ContentType` missing type before type-subtype separator: '/'.",
            Self::InvalidTypeChar => "`ContentType` type contains an invalid character",
            Self::MissingSubtype => {
                "`ContentType` missing subtype after type-subtype separator: '/'."
            }
            Self::InvalidSubtypeChar => "`ContentType` subtype contains an invalid character",
            Self::MissingParameterSeparator => {
                "`ContentType` invalid parameter separator, expected ';'."
            }
            Self::InvalidParameterKey => "`ContentType` invalid character in parameter key.",
            Self::MissingParameterAssignment => {
                "`ContentType` invalid parameter key or missing '='."
            }
            Self::InvalidParameterValue => {
                "`ContentType` invalid character or empty parameter value."
            }
        }
    }
}

#[cfg(test)]
mod mime_tests {
    use super::*;

    #[test]
    fn control() {
        const CONTROL_CONTENT_TYPE: &str = r#"application/json; version="1";mode=debug"#;

        // compile time checks
        const _CONTROL_COMPILE_TIME: ContentType<&str> = ContentType::new(CONTROL_CONTENT_TYPE);

        // runtime checks
        assert!(ContentType::try_from(CONTROL_CONTENT_TYPE).is_ok());

        // parsed mime compatibility
        let mime: mime::Mime = "application/json".parse().unwrap();
        assert!(ContentType::try_from(mime.to_string()).is_ok());
    }

    #[test]
    fn length_exceeded() {
        assert_eq!(
            ContentType::<&str>::validate_source(
                "application/vnd.example.super-long-custom-format+json;version=42;mode=fast;region=us-west-2;retry=5;debug=true;feature=experimental",
            ).err(),
            Some(ContentTypeValidationError::LengthExceeded)
        );
    }

    #[test]
    fn missing_type_subtype_separator() {
        assert_eq!(
            ContentType::<&str>::validate_source("applicationjson;version=1").err(),
            Some(ContentTypeValidationError::MissingTypeSeparator)
        );
    }

    #[test]
    fn missing_type() {
        assert_eq!(
            ContentType::<&str>::validate_source("/json;version=1").err(),
            Some(ContentTypeValidationError::MissingType)
        );
    }

    #[test]
    fn invalid_type_char() {
        assert_eq!(
            ContentType::<&str>::validate_source("applic@tion/json;version=1").err(),
            Some(ContentTypeValidationError::InvalidTypeChar)
        );
    }

    #[test]
    fn missing_subtype() {
        assert_eq!(
            ContentType::<&str>::validate_source("application/;version=1").err(),
            Some(ContentTypeValidationError::MissingSubtype)
        );
    }

    #[test]
    fn invalid_subtype_char() {
        assert_eq!(
            ContentType::<&str>::validate_source("application/custom@json;version=1").err(),
            Some(ContentTypeValidationError::InvalidSubtypeChar)
        );
        assert_eq!(
            ContentType::<&str>::validate_source("application/json ;version=1").err(),
            Some(ContentTypeValidationError::InvalidSubtypeChar)
        );
    }

    #[test]
    fn missing_parameter_separator() {
        assert_eq!(
            ContentType::<&str>::validate_source("application/jsonversion=1").err(),
            Some(ContentTypeValidationError::MissingParameterSeparator)
        );
        assert_eq!(
            ContentType::<&str>::validate_source("application/jsonversion=1").err(),
            Some(ContentTypeValidationError::MissingParameterSeparator)
        );
        assert_eq!(
            ContentType::<&str>::validate_source("application/jsonversion=1;mode=debug").err(),
            Some(ContentTypeValidationError::MissingParameterSeparator)
        );
    }

    #[test]
    fn invalid_parameter_key() {
        assert_eq!(
            ContentType::<&str>::validate_source("application/json;versi@n=1").err(),
            Some(ContentTypeValidationError::InvalidParameterKey)
        );
    }

    #[test]
    fn missing_parameter_assignment() {
        assert_eq!(
            ContentType::<&str>::validate_source("application/json;version1").err(),
            Some(ContentTypeValidationError::MissingParameterAssignment)
        );
    }

    #[test]
    fn invalid_parameter_value() {
        assert_eq!(
            ContentType::<&str>::validate_source("application/json;version=1@").err(),
            Some(ContentTypeValidationError::InvalidParameterValue)
        );
    }
}
