# Changelog

All notable changes to this project are documented in this file.

## [0.1.0] - 2026-05-28

### Added

- Native `encode_packet` / `decode_packet` API with secure space escaping (`~` / `~s`).
- Base64URL wire encoding (no padding) and strict ingress validation (`^[A-Za-z0-9_-]+$`).
- Printable ASCII plaintext policy (control bytes rejected).
- Global native rate limiting (500 characters per second).
- `ConcurrentPipeline` for std-thread batch encoding with bounded job channels.
- Initial README and GPL-3.0 license.
