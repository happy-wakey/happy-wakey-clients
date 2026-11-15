# AttoLang client metadata

`client.atto` is the initial AttoLang (`.atto`) **pure request-description**
surface for the six Happy Wakey operations. It exposes canonical paths,
a dynamic-route **template**, allowlisted operation IDs, Shared Auth bearer
policy, and the credential-free telemetry event name.

This is deliberately **not yet a runnable HTTP client**. An Atto-capable host
adapter must provide and test: HTTPS enforcement (with explicit opt-in only
for development loopback), bearer in headers rather than URLs, same-origin
redirect refusal, path-segment percent encoding, cursor query encoding,
JSON Schema validation, idempotency/occurrence generation handling, safe
error wrapping, and Ores `next-loggers/v1` telemetry without secrets or
request bodies. Unknown operations must be rejected **before** querying
`requires_bearer`. The dynamic occurrence path is not ready to send until
its `{occurrence_id}` placeholder is encoded and replaced.

Run `python3 scripts/validate_matrix.py` and
`cargo test --manifest-path clients/rust/Cargo.toml --test atto_contract` from the repository
root. They compare the Atto source against the pinned operation contract and
exercise negative drift cases. These are **host-side static conformance tests**,
not guest compilation or execution. No end-to-end Atto runtime support is
claimed until a tested transport adapter and independent compiler evidence
exist.

## Rust host-contract prototype

The standalone Rust crate at `rust-host/` provides a pinned six-operation
request planner, an immutable validated origin, and a pluggable transport
interface. It encodes occurrence IDs and cursors safely, checks the required
request shape for alarms and transitions, caps buffered request/response
sizes, refuses redirects, rejects duplicate JSON response keys and reports
credential-free telemetry.

Run the **Rust-only Atto test suite** with:

```sh
cargo test --manifest-path clients/rust/Cargo.toml --test atto_contract
cargo test --manifest-path clients/atto/rust-host/Cargo.toml
```

The host crate does **not implement an HTTP transport**, does **not compile
or execute .atto code**, and does not replace the full authoritative JSON
Schema validator. The production Transport must independently enforce TLS,
DNS/egress restrictions, redirects, per-request timeouts, cancellation and
streaming byte caps *before* returning a buffered response. Do not admit
production use until compiler and wire-conformance evidence exists.
