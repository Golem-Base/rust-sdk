use std::str::FromStr;

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum ValidationError {
    #[error(transparent)]
    ContentType(#[from] ContentTypeValidationError),

    #[error("failed to convert string to MIME: {0}")]
    MimeFromStr(String),

    #[error("`BTL` must be a non-zero `u64`.")]
    InvalidBtl,
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

#[derive(Debug)]
pub struct ContentType {
    source: String,
    media_type: mime::Mime,
}
impl ContentType {
    /// A reference to the underlying `String` parsed into a `ContentType`.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// A reference to the parsed source.
    pub fn media_type(&self) -> &mime::Mime {
        &self.media_type
    }

    /// Used for compile time validation of custom content types as `const` values.
    ///
    /// In 99% of cases, using `ContentType::try_from` is enough, but this method guarantees that the
    /// source is valid prior to runtime and will not be rejected by the network. Validation adheres
    /// to RFC 2045 and RFC 7231 MIME type standards, as well as the requirements for the Arkiv network.
    ///
    /// # Example
    ///
    /// ```rs,ignore
    /// use golem_base_sdk::entity::ContentType;
    ///
    /// pub const CUSTOM_CONTENT_TYPE: &str = ContentType::custom("application/vnd.example.long-format+json;version=42;mode=fast;debug=true;region=us-west-2;retry=5");
    /// ```
    pub const fn custom(source: &'static str) -> &'static str {
        match Self::validate_source(source) {
            Ok(()) => source,
            Err(err) => panic!("{}", err.as_static_str()),
        }
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
    pub(crate) fn validate(self) -> Result<Self, ValidationError> {
        Self::validate_source(self.source()).map_err(ValidationError::from)?;

        Ok(self)
    }
}
impl TryFrom<mime::Mime> for ContentType {
    type Error = ValidationError;

    // `mime::Mime` implements `std::fmt::Display`, automatically implementing
    // `ToString`, which is writing the original source into the `Formatter`.
    fn try_from(media_type: mime::Mime) -> Result<Self, Self::Error> {
        let content_type = Self {
            source: media_type.to_string(),
            media_type,
        };

        content_type.validate()
    }
}
/// Attempts to parse the content type string as `MediaType::Mime` otherwise
/// defaults to `MediaType::Custom`. Only returns an error if validation fails.
impl TryFrom<&str> for ContentType {
    type Error = ValidationError;

    fn try_from(source: &str) -> Result<Self, Self::Error> {
        let source = source.to_string();
        let content_type = Self {
            source: source.clone(),
            media_type: mime::Mime::from_str(&source)
                .map_err(|err| ValidationError::MimeFromStr(err.to_string()))?,
        };

        content_type.validate()
    }
}
/// Attempts to parse the content type string as `MediaType::Mime` otherwise
/// defaults to `MediaType::Custom`. Only returns an error if validation fails.
impl TryFrom<String> for ContentType {
    type Error = ValidationError;

    fn try_from(source: String) -> Result<Self, Self::Error> {
        let content_type = Self {
            source: source.clone(),
            media_type: mime::Mime::from_str(&source)
                .map_err(|err| ValidationError::MimeFromStr(err.to_string()))?,
        };

        content_type.validate()
    }
}

#[cfg(test)]
mod mime_tests {
    use super::*;

    #[test]
    fn control() {
        const CONTROL_CONTENT_TYPE: &str = r#"application/json; version="1";mode=debug"#;

        // compile time checks
        const _CONTROL_COMPILE_TIME: &str = ContentType::custom(CONTROL_CONTENT_TYPE);

        // runtime checks
        assert!(ContentType::try_from(CONTROL_CONTENT_TYPE).is_ok());
    }

    #[test]
    fn length_exceeded() {
        assert_eq!(
            ContentType::validate_source(
                "application/vnd.example.super-long-custom-format+json;version=42;mode=fast;region=us-west-2;retry=5;debug=true;feature=experimental",
            ).err(),
            Some(ContentTypeValidationError::LengthExceeded)
        );
    }

    #[test]
    fn missing_type_subtype_separator() {
        assert_eq!(
            ContentType::validate_source("applicationjson;version=1").err(),
            Some(ContentTypeValidationError::MissingTypeSeparator)
        );
    }

    #[test]
    fn missing_type() {
        assert_eq!(
            ContentType::validate_source("/json;version=1").err(),
            Some(ContentTypeValidationError::MissingType)
        );
    }

    #[test]
    fn invalid_type_char() {
        assert_eq!(
            ContentType::validate_source("applic@tion/json;version=1").err(),
            Some(ContentTypeValidationError::InvalidTypeChar)
        );
    }

    #[test]
    fn missing_subtype() {
        assert_eq!(
            ContentType::validate_source("application/;version=1").err(),
            Some(ContentTypeValidationError::MissingSubtype)
        );
    }

    #[test]
    fn invalid_subtype_char() {
        assert_eq!(
            ContentType::validate_source("application/custom@json;version=1").err(),
            Some(ContentTypeValidationError::InvalidSubtypeChar)
        );
        assert_eq!(
            ContentType::validate_source("application/json ;version=1").err(),
            Some(ContentTypeValidationError::InvalidSubtypeChar)
        );
    }

    #[test]
    fn missing_parameter_separator() {
        assert_eq!(
            ContentType::validate_source("application/jsonversion=1").err(),
            Some(ContentTypeValidationError::MissingParameterSeparator)
        );
        assert_eq!(
            ContentType::validate_source("application/jsonversion=1").err(),
            Some(ContentTypeValidationError::MissingParameterSeparator)
        );
        assert_eq!(
            ContentType::validate_source("application/jsonversion=1;mode=debug").err(),
            Some(ContentTypeValidationError::MissingParameterSeparator)
        );
    }

    #[test]
    fn invalid_parameter_key() {
        assert_eq!(
            ContentType::validate_source("application/json;versi@n=1").err(),
            Some(ContentTypeValidationError::InvalidParameterKey)
        );
    }

    #[test]
    fn missing_parameter_assignment() {
        assert_eq!(
            ContentType::validate_source("application/json;version1").err(),
            Some(ContentTypeValidationError::MissingParameterAssignment)
        );
    }

    #[test]
    fn invalid_parameter_value() {
        assert_eq!(
            ContentType::validate_source("application/json;version=1@").err(),
            Some(ContentTypeValidationError::InvalidParameterValue)
        );
    }
}
