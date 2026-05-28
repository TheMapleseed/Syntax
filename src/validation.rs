#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    EmptyInput,
    InvalidByte { index: usize, byte: u8 },
}

pub fn validate_wire_format(input: &str) -> Result<(), ValidationError> {
    if input.is_empty() {
        return Err(ValidationError::EmptyInput);
    }

    for (index, byte) in input.bytes().enumerate() {
        let is_valid = byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-';
        if !is_valid {
            return Err(ValidationError::InvalidByte { index, byte });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_base64url_alphabet() {
        assert!(validate_wire_format("AbC_012-xyz").is_ok());
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(validate_wire_format(""), Err(ValidationError::EmptyInput));
    }

    #[test]
    fn rejects_illegal_symbols() {
        let err = validate_wire_format("abc+def").unwrap_err();
        assert_eq!(
            err,
            ValidationError::InvalidByte {
                index: 3,
                byte: b'+'
            }
        );
    }
}
