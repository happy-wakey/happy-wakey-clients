# happy-wakey-clients

Official contract-first clients for the Happy Wakey JSON API. The repository
ships 17 package slices across 16 programming languages plus four TypeScript
runtimes. Every slice exposes the same six OpenAPI operations and carries
telemetry through the [`ores-otel/ores.otel.log`](https://github.com/ores-otel/ores.otel.log)
`next-loggers/v1` contract.

| Language | Package path | Transport |
| --- | --- | --- |
| C | `clients/c` | injected native HTTP callback |
| C++ | `clients/cpp` | injected native HTTP callback |
| Zig | `clients/zig` | injected native HTTP callback |
| TypeScript | `clients/typescript` | Fetch; Node, Deno, Bun, Edge |
| Python | `clients/python` | standard-library HTTP |
| Go | `clients/go` | `net/http` |
| Ruby | `clients/ruby` | `Net::HTTP` |
| PHP | `clients/php` | injected PSR-style transport |
| Rust | `clients/rust` | `reqwest` |
| Rust/WASM | `clients/wasm` | host Fetch callback |
| Dart | `clients/dart` | injected transport for Flutter/web |
| Gleam | `clients/gleam` | injected transport |
| Erlang | `clients/erlang` | injected transport |
| Elixir | `clients/elixir` | injected transport |
| Java | `clients/java` | `java.net.http.HttpClient` |
| Kotlin | `clients/kotlin` | `java.net.http.HttpClient` |
| Swift | `clients/swift` | `URLSession` |

The contract lock pins the exact `happy-wakey-interfaces` commit used to
author the clients. `scripts/validate_matrix.py` checks all package sentinels,
all operation names, TLS policy, bearer handling, and an Ores telemetry bridge
for every target.

## Uniform behavior

- Production base URLs must use HTTPS. Plain HTTP is accepted only for a
  loopback host with an explicit development opt-out.
- Bearer tokens are carried but never parsed, logged, or placed in an error.
- Non-success responses are controlled errors; redirects are not followed
  automatically because they could forward authorization to another origin.
- Mutations require caller-supplied idempotency/transition IDs and occurrence
  generations from `happy-wakey-interfaces`.
- Telemetry records operation, outcome, status, duration, and trace ID only.
  Request bodies, alarm labels, and authorization values are excluded.
- Async runtimes must discard stale completions in their native app machine;
  an SDK response does not bypass app or occurrence state authority.

## Validate

```sh
python3 scripts/validate_matrix.py
RUSTUP_TOOLCHAIN=stable cargo test --manifest-path clients/rust/Cargo.toml
(cd clients/go && go test ./...)
npm --prefix clients/typescript test
```
