import {
  defer,
  EMPTY,
  exhaustMap,
  fromEvent,
  Observable,
  take,
  takeUntil,
  timer,
} from "rxjs";

export type JsonPrimitive = boolean | number | string | null;
export type JsonValue =
  | JsonPrimitive
  | readonly JsonValue[]
  | { readonly [key: string]: JsonValue };

export interface Alarm {
  readonly id: string;
  readonly label: string;
  readonly local_time: string;
  readonly time_zone: string;
  readonly weekdays: readonly number[];
  readonly enabled: boolean;
  readonly sound: string;
  readonly volume: number;
  readonly gradual_seconds: number;
  readonly tags: readonly string[];
  readonly generation: number;
  readonly created_at: string;
  readonly updated_at: string;
}

export interface CreateAlarmRequest extends Omit<Alarm, "id" | "generation" | "created_at" | "updated_at"> {
  readonly transition_id: string;
}

export type AlarmTransitionEvent =
  | "fire"
  | "acknowledge"
  | "snooze"
  | "complete"
  | "mark_missed"
  | "cancel";

export interface TransitionAlarmRequest {
  readonly transition_id: string;
  readonly expected_generation: number;
  readonly event: AlarmTransitionEvent;
  readonly snooze_until: string | null;
  readonly client_time: string;
}

export interface TransitionAlarmResponse {
  readonly disposition: "applied" | "stale" | "rejected";
  readonly occurrence: JsonValue;
  readonly error: JsonValue | null;
}

export interface SyncChange {
  readonly change_id: string;
  readonly scope: string;
  readonly collection: string;
  readonly entity_id: string;
  readonly operation: "upsert" | "delete";
  readonly generation: number;
  readonly actor_id: string;
  readonly document: JsonValue | null;
  readonly occurred_at: string;
}

export interface SyncEnvelope {
  readonly schema: string;
  readonly cursor: string;
  readonly changes: readonly SyncChange[];
  readonly has_more: boolean;
}

export interface OresTelemetryEvent {
  addFields(fields: Readonly<Record<string, unknown>>): OresTelemetryEvent;
  addTags(...tags: readonly string[]): OresTelemetryEvent;
  send(): void | Promise<void>;
}

export interface OresTelemetryLogger {
  info(...values: readonly unknown[]): OresTelemetryEvent;
  error(...values: readonly unknown[]): OresTelemetryEvent;
}

export interface ClientOptions {
  readonly baseUrl: string;
  readonly token?: string;
  readonly logger?: OresTelemetryLogger;
  readonly fetch?: typeof fetch;
  readonly allowInsecureLoopback?: boolean;
}

export interface PollOptions {
  readonly intervalMs?: number;
  readonly signal?: AbortSignal;
}

export interface RequestOptions {
  readonly signal?: AbortSignal;
}

export class HappyWakeyError extends Error {
  constructor(
    readonly status: number,
    readonly body: unknown,
  ) {
    super(`Happy Wakey request failed (${status})`);
  }
}

export function validateBaseUrl(options: ClientOptions): URL {
  const base = new URL(options.baseUrl);
  const hostname = base.hostname.toLowerCase();
  const host = hostname.startsWith("[") && hostname.endsWith("]")
    ? hostname.slice(1, -1)
    : hostname;
  const loopback = ["localhost", "127.0.0.1", "::1"].includes(host);
  const numericIp = /^(?:\d{1,3}\.){3}\d{1,3}$/.test(host) || host.includes(":");
  if (numericIp && !loopback) {
    throw new TypeError("public IP literals are not allowed");
  }
  if (base.protocol !== "https:" && !(options.allowInsecureLoopback && loopback)) {
    throw new TypeError("HTTPS required");
  }
  return base;
}

export function asObservable<T>(operation: () => Promise<T>): Observable<T> {
  return defer(operation);
}

function aborts(signal?: AbortSignal): Observable<Event | undefined> {
  if (signal === undefined) return EMPTY;
  if (signal.aborted) return defer(() => [undefined]);
  return fromEvent(signal, "abort").pipe(take(1));
}

function pollInterval(value = 30_000): number {
  if (!Number.isSafeInteger(value) || value < 10) {
    throw new RangeError("poll interval must be a safe integer of at least 10ms");
  }
  return value;
}

export class HappyWakeyClient {
  readonly #base: URL;
  readonly #token?: string;
  readonly #logger?: OresTelemetryLogger;
  readonly #fetch: typeof fetch;

  constructor(options: ClientOptions) {
    this.#base = validateBaseUrl(options);
    this.#token = options.token;
    this.#logger = options.logger;
    this.#fetch = options.fetch ?? globalThis.fetch;
  }

  health(): Promise<{ readonly status: "ok" }> {
    return this.#request("health", "GET", "/healthz", undefined, false);
  }

  listAlarms(options: RequestOptions = {}): Promise<readonly Alarm[]> {
    return this.#request("listAlarms", "GET", "/v1/alarms", undefined, true, options.signal);
  }

  createAlarm(request: CreateAlarmRequest): Promise<Alarm> {
    return this.#request("createAlarm", "POST", "/v1/alarms", request);
  }

  transitionOccurrence(
    id: string,
    request: TransitionAlarmRequest,
  ): Promise<TransitionAlarmResponse> {
    return this.#request(
      "transitionOccurrence",
      "POST",
      `/v1/occurrences/${encodeURIComponent(id)}/transitions`,
      request,
    );
  }

  pullChanges(cursor = "0", limit = 100): Promise<SyncEnvelope> {
    const query = new URLSearchParams({ cursor, limit: String(limit) });
    return this.#request("pullChanges", "GET", `/v1/sync/pull?${query}`);
  }

  pushChanges(changes: readonly SyncChange[]): Promise<SyncEnvelope> {
    return this.#request("pushChanges", "POST", "/v1/sync/push", changes);
  }

  /// Cold stream: no request or credential use occurs before subscription.
  observeAlarms(options: RequestOptions = {}): Observable<readonly Alarm[]> {
    return asObservable(() => this.listAlarms(options));
  }

  /// Non-overlapping polling. Slow requests are never duplicated; aborting the
  /// supplied signal completes the stream and allows `fetch` cancellation to
  /// remain an explicit caller-owned concern.
  watchAlarms(options: PollOptions = {}): Observable<readonly Alarm[]> {
    const intervalMs = pollInterval(options.intervalMs);
    if (options.signal?.aborted) return EMPTY;
    return timer(0, intervalMs).pipe(
      exhaustMap(() => this.observeAlarms({ signal: options.signal })),
      takeUntil(aborts(options.signal)),
    );
  }

  async #request<T>(
    operation: string,
    method: "GET" | "POST",
    path: string,
    body?: unknown,
    auth = true,
    signal?: AbortSignal,
  ): Promise<T> {
    if (auth && !this.#token) {
      throw new TypeError("Shared Auth bearer token required");
    }
    const started = performance.now();
    try {
      const response = await this.#fetch(new URL(path, this.#base), {
        method,
        redirect: "error",
        signal,
        headers: {
          accept: "application/json",
          ...(auth ? { authorization: `Bearer ${this.#token}` } : {}),
          ...(body === undefined ? {} : { "content-type": "application/json" }),
        },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
      const value: unknown = await response.json().catch(() => null);
      if (!response.ok) throw new HappyWakeyError(response.status, value);
      void this.#logger
        ?.info("happy_wakey.client.request")
        .addFields({
          operation,
          status: response.status,
          duration_ms: performance.now() - started,
        })
        .addTags("happy-wakey", "client")
        .send();
      return value as T;
    } catch (error) {
      void this.#logger
        ?.error("happy_wakey.client.request")
        .addFields({ operation, duration_ms: performance.now() - started })
        .addTags("happy-wakey", "client")
        .send();
      throw error;
    }
  }
}
