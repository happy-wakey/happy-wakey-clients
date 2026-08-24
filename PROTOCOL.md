# Client protocol

The normative HTTP surface is
`happy-wakey-interfaces/openapi/happy-wakey.openapi.json`. This repository
mirrors exactly these operation IDs:

1. `health`
2. `listAlarms`
3. `createAlarm`
4. `transitionOccurrence`
5. `pullChanges`
6. `pushChanges`

`listAlarms`, `createAlarm`, `transitionOccurrence`, `pullChanges`, and
`pushChanges` carry `Authorization: Bearer <Shared Auth token>`. The token is an
opaque credential. Only the Rust origin validates issuer, audience, expiry,
subject, and authorization policy.

Every implementation accepts or owns one telemetry adapter. Native Ores SDKs
are used where `ores.otel.log` publishes one. Other languages use the same
structural bridge: an event named `happy_wakey.client.request` with the
`next-loggers/v1` schema. A host may forward that record through its Ores
OpenTelemetry, Supabase, or custom transport. Logging failure never changes an
API result.

