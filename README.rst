secure_pipeline
===============

`secure_pipeline` is a pure Rust (std-only) library for strict packet-safe text transport.
It provides:

- Native secure encoding/decoding entrypoints.
- A collision-proof blank-space handling system.
- Base64URL wire format with no padding.
- Strict ingress validation for allowed bytes only.
- Optional thread-based concurrent batch encoding.

No external crates are used.


Why this exists
---------------

Many injection paths rely on delimiter/control characters (spaces, newlines, quotes, shell metacharacters)
being interpreted by downstream systems. This library enforces a narrow, deterministic wire format and
prevents ambiguous space transformations during round-trip encoding.


Security model
--------------

Native behavior is always active in public APIs:

- ``encode_packet()`` and ``decode_packet()`` automatically use secure space handling.
- Incoming encoded payloads are validated against:

  ::

      ^[A-Za-z0-9_-]+$

This excludes control bytes and common interpreter separators from the wire representation.

Important boundary notes
------------------------

The project explicitly documents these constraints:

- There is no hard input-size limit in the library.
- There is native rate limiting: at most 500 characters per second, shared by
  ``encode_packet()`` and ``decode_packet()`` calls.
- Wire safety is not the same thing as sink safety. After decode, callers must
  still use sink-specific protections (parameterized SQL, shell argv arrays,
  proper escaping/encoding per sink).
- The current implementation is ASCII-only and rejects non-ASCII input.
- Printable ASCII is enforced for plaintext payloads (control bytes like
  ``\n``, ``\r``, ``\t``, and ``\0`` are rejected).


Blank-space system (collision-proof)
------------------------------------

The native algorithm uses an escape grammar so literals remain distinguishable:

- ``~`` becomes ``~~``
- space `` `` becomes ``~s``

During decode, only recognized escape sequences are accepted:

- ``~~`` -> ``~``
- ``~s`` -> space

Any malformed escape sequence is rejected.

This guarantees that literal underscores, literal tildes, and real spaces survive round-trip
without collisions.


Installation
------------

Add this crate as a local dependency (or publish and use a versioned dependency):

::

    [dependencies]
    secure_pipeline = { path = "." }


Quick start
-----------

::

    use secure_pipeline::{decode_packet, encode_packet};

    fn main() {
        let encoded = encode_packet("hello safe world").unwrap();
        let decoded = decode_packet(&encoded).unwrap();
        assert_eq!(decoded, "hello safe world");
    }


Concurrent batch encoding
-------------------------

Use ``ConcurrentPipeline`` for thread-based batch work (no async runtime required):

::

    use secure_pipeline::{ConcurrentPipeline, PipelineConfig, decode_packet};

    fn main() {
        let pipeline = ConcurrentPipeline::new(PipelineConfig { workers: 4 });
        let packets = pipeline.encode_all(vec!["one value", "two value"]);

        for packet in packets {
            let packet = packet.unwrap();
            let original = decode_packet(&packet).unwrap();
            println!("{}", original);
        }
    }


Public API
----------

- ``encode_packet(input: &str) -> Result<String, PipelineError>``
- ``decode_packet(input: &str) -> Result<String, PipelineError>``
- ``validate_wire_format(input: &str) -> Result<(), ValidationError>``
- ``ConcurrentPipeline``
- ``PipelineConfig { workers }``

The mode override internals are intentionally not public to keep the secure path native and uniform.


Error handling
--------------

Primary error types:

- ``PipelineError::NonAsciiInput``
- ``PipelineError::ControlByte { index, byte }``
- ``PipelineError::RateLimited { allowed_per_second, attempted_chars }``
- ``PipelineError::InvalidEscapeSequence``
- ``PipelineError::Validation(..)``
- ``PipelineError::Decode(..)``
- ``PipelineError::Utf8``
- ``PipelineError::InternalFailure(..)``


Development
-----------

Run tests:

::

    cargo test

Current test suite covers:

- Base64URL round-trip and invalid length.
- Wire format validation.
- Collision-proof blank-space handling.
- Concurrent batch ordering and decode integrity.
- Rejection of plaintext control characters.
