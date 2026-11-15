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
`python3 -m unittest discover -s tests -p '*_test.py'` from the repository
root. They compare the Atto source against the pinned operation contract and
exercise negative drift cases. These are **host-side static conformance tests**,
not guest compilation or execution. No end-to-end Atto runtime support is
claimed until a tested transport adapter and independent compiler evidence
exist.
