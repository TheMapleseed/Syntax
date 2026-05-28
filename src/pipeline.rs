use crate::base64url;
use crate::validation::{validate_wire_format, ValidationError};

const NATIVE_SPACE_MODE: SpaceMode = SpaceMode::Underscore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpaceMode {
    Underscore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineError {
    NonAsciiInput,
    InvalidEscapeSequence,
    Validation(ValidationError),
    Decode(base64url::DecodeError),
    Utf8,
}

fn ascii_guard(input: &str) -> Result<(), PipelineError> {
    if input.is_ascii() {
        Ok(())
    } else {
        Err(PipelineError::NonAsciiInput)
    }
}

pub(crate) fn tokenize_spaces(input: &str, space_mode: &SpaceMode) -> Result<String, PipelineError> {
    ascii_guard(input)?;
    let escaped = input.replace('~', "~~");
    match space_mode {
        SpaceMode::Underscore => Ok(escaped.replace(' ', "~s")),
    }
}

pub(crate) fn untokenize_spaces(input: &str, space_mode: &SpaceMode) -> Result<String, PipelineError> {
    ascii_guard(input)?;
    match space_mode {
        SpaceMode::Underscore => decode_escaped_underscore_mode(input),
    }
}

fn decode_escaped_underscore_mode(input: &str) -> Result<String, PipelineError> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch != '~' {
            out.push(ch);
            continue;
        }

        let Some(next) = chars.next() else {
            return Err(PipelineError::InvalidEscapeSequence);
        };

        match next {
            '~' => out.push('~'),
            's' => out.push(' '),
            _ => return Err(PipelineError::InvalidEscapeSequence),
        }
    }

    Ok(out)
}

pub fn encode_packet(input: &str) -> Result<String, PipelineError> {
    encode_packet_with_mode(input, &NATIVE_SPACE_MODE)
}

pub(crate) fn encode_packet_with_mode(input: &str, space_mode: &SpaceMode) -> Result<String, PipelineError> {
    let tokenized = tokenize_spaces(input, space_mode)?;
    Ok(base64url::encode(tokenized.as_bytes()))
}

pub fn decode_packet(input: &str) -> Result<String, PipelineError> {
    decode_packet_with_mode(input, &NATIVE_SPACE_MODE)
}

pub(crate) fn decode_packet_with_mode(input: &str, space_mode: &SpaceMode) -> Result<String, PipelineError> {
    validate_wire_format(input).map_err(PipelineError::Validation)?;
    let decoded = base64url::decode(input).map_err(PipelineError::Decode)?;
    let text = String::from_utf8(decoded).map_err(|_| PipelineError::Utf8)?;
    untokenize_spaces(&text, space_mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_with_underscore_mode() {
        let mode = SpaceMode::Underscore;
        let encoded = encode_packet_with_mode("hello world", &mode).unwrap();
        let decoded = decode_packet_with_mode(&encoded, &mode).unwrap();
        assert_eq!(decoded, "hello world");
    }

    #[test]
    fn preserves_literal_underscores_in_underscore_mode() {
        let mode = SpaceMode::Underscore;
        let input = "a_b c";
        let encoded = encode_packet_with_mode(input, &mode).unwrap();
        let decoded = decode_packet_with_mode(&encoded, &mode).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn preserves_literal_tilde_characters() {
        let mode = SpaceMode::Underscore;
        let input = "value~with~tildes and space";
        let encoded = encode_packet_with_mode(input, &mode).unwrap();
        let decoded = decode_packet_with_mode(&encoded, &mode).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn native_calls_apply_security_by_default() {
        let input = "safe text with spaces";
        let encoded = encode_packet(input).unwrap();
        let decoded = decode_packet(&encoded).unwrap();
        assert_eq!(decoded, input);
    }
}
