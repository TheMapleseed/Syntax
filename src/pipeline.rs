use crate::base64url;
use crate::validation::{validate_wire_format, ValidationError};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const NATIVE_SPACE_MODE: SpaceMode = SpaceMode::Underscore;
const RATE_LIMIT_CHARS_PER_SECOND: usize = 500;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpaceMode {
    Underscore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineError {
    NonAsciiInput,
    ControlByte {
        index: usize,
        byte: u8,
    },
    RateLimited {
        allowed_per_second: usize,
        attempted_chars: usize,
    },
    InvalidEscapeSequence,
    Validation(ValidationError),
    Decode(base64url::DecodeError),
    Utf8,
    InternalFailure(&'static str),
}

#[derive(Debug)]
struct RateLimiterState {
    window_start: Instant,
    used_chars: usize,
}

fn global_rate_limiter() -> &'static Mutex<RateLimiterState> {
    static RATE_LIMITER: OnceLock<Mutex<RateLimiterState>> = OnceLock::new();
    RATE_LIMITER.get_or_init(|| {
        Mutex::new(RateLimiterState {
            window_start: Instant::now(),
            used_chars: 0,
        })
    })
}

fn enforce_rate_limit(input: &str) -> Result<(), PipelineError> {
    let attempted = input.chars().count();
    let limiter = global_rate_limiter();
    let mut guard = limiter.lock().expect("rate limiter mutex poisoned");
    let now = Instant::now();

    if now.duration_since(guard.window_start) >= RATE_LIMIT_WINDOW {
        guard.window_start = now;
        guard.used_chars = 0;
    }

    if guard.used_chars.saturating_add(attempted) > RATE_LIMIT_CHARS_PER_SECOND {
        return Err(PipelineError::RateLimited {
            allowed_per_second: RATE_LIMIT_CHARS_PER_SECOND,
            attempted_chars: attempted,
        });
    }

    guard.used_chars += attempted;
    Ok(())
}

fn ascii_guard(input: &str) -> Result<(), PipelineError> {
    if input.is_ascii() {
        Ok(())
    } else {
        Err(PipelineError::NonAsciiInput)
    }
}

fn printable_ascii_guard(input: &str) -> Result<(), PipelineError> {
    ascii_guard(input)?;
    for (index, byte) in input.bytes().enumerate() {
        if !(0x20..=0x7e).contains(&byte) {
            return Err(PipelineError::ControlByte { index, byte });
        }
    }
    Ok(())
}

pub(crate) fn tokenize_spaces(input: &str, space_mode: &SpaceMode) -> Result<String, PipelineError> {
    printable_ascii_guard(input)?;
    let escaped = input.replace('~', "~~");
    match space_mode {
        SpaceMode::Underscore => Ok(escaped.replace(' ', "~s")),
    }
}

pub(crate) fn untokenize_spaces(input: &str, space_mode: &SpaceMode) -> Result<String, PipelineError> {
    printable_ascii_guard(input)?;
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
    enforce_rate_limit(input)?;
    encode_packet_with_mode(input, &NATIVE_SPACE_MODE)
}

pub(crate) fn encode_packet_with_mode(input: &str, space_mode: &SpaceMode) -> Result<String, PipelineError> {
    let tokenized = tokenize_spaces(input, space_mode)?;
    Ok(base64url::encode(tokenized.as_bytes()))
}

pub fn decode_packet(input: &str) -> Result<String, PipelineError> {
    enforce_rate_limit(input)?;
    decode_packet_with_mode(input, &NATIVE_SPACE_MODE)
}

pub(crate) fn decode_packet_with_mode(input: &str, space_mode: &SpaceMode) -> Result<String, PipelineError> {
    validate_wire_format(input).map_err(PipelineError::Validation)?;
    let decoded = base64url::decode(input).map_err(PipelineError::Decode)?;
    let text = String::from_utf8(decoded).map_err(|_| PipelineError::Utf8)?;
    let untokenized = untokenize_spaces(&text, space_mode)?;
    printable_ascii_guard(&untokenized)?;
    Ok(untokenized)
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

    #[test]
    fn rejects_control_bytes_on_encode() {
        let err = encode_packet("line1\nline2").unwrap_err();
        assert!(matches!(err, PipelineError::ControlByte { .. }));
    }

    #[test]
    fn rejects_control_bytes_after_decode() {
        let wire = base64url::encode(b"hi\nthere");
        let err = decode_packet(&wire).unwrap_err();
        assert!(matches!(err, PipelineError::ControlByte { .. }));
    }
}
