export interface OresTelemetryEvent {
  addFields(fields: Record<string, unknown>): OresTelemetryEvent;
  addTags(...tags: string[]): OresTelemetryEvent;
  send(): void | Promise<void>;
}
export interface OresTelemetryLogger {
  info(...values: unknown[]): OresTelemetryEvent;
  error(...values: unknown[]): OresTelemetryEvent;
}
export interface ClientOptions {
  baseUrl: string;
  token?: string;
  logger?: OresTelemetryLogger;
  fetch?: typeof fetch;
  allowInsecureLoopback?: boolean;
}
export class HappyWakeyError extends Error {
  constructor(readonly status: number, readonly body: unknown) { super(`Happy Wakey request failed (${status})`); }
}
export class HappyWakeyClient {
  readonly #base: URL;
  readonly #token?: string;
  readonly #logger?: OresTelemetryLogger;
  readonly #fetch: typeof fetch;
  constructor(options: ClientOptions) {
    this.#base = new URL(options.baseUrl);
    let host = this.#base.hostname.toLowerCase();
    if (host.startsWith("[") && host.endsWith("]")) host = host.slice(1, -1);
    const loopback = ["localhost", "127.0.0.1", "::1"].includes(host);
    const numericIp = /^(?:\d{1,3}\.){3}\d{1,3}$/.test(host) || host.includes(":");
    if (numericIp && !loopback) throw new TypeError("public IP literals are not allowed");
    if (this.#base.protocol !== "https:" && !(options.allowInsecureLoopback && loopback)) throw new TypeError("HTTPS required");
    this.#token = options.token;
    this.#logger = options.logger;
    this.#fetch = options.fetch ?? globalThis.fetch;
  }
  health() { return this.#request("health", "GET", "/healthz", undefined, false); }
  listAlarms() { return this.#request("listAlarms", "GET", "/v1/alarms"); }
  createAlarm(request: unknown) { return this.#request("createAlarm", "POST", "/v1/alarms", request); }
  transitionOccurrence(id: string, request: unknown) { return this.#request("transitionOccurrence", "POST", `/v1/occurrences/${encodeURIComponent(id)}/transitions`, request); }
  pullChanges(cursor = "0", limit = 100) { const query = new URLSearchParams({cursor, limit: String(limit)}); return this.#request("pullChanges", "GET", `/v1/sync/pull?${query}`); }
  pushChanges(changes: unknown[]) { return this.#request("pushChanges", "POST", "/v1/sync/push", changes); }
  async #request(operation: string, method: string, path: string, body?: unknown, auth = true): Promise<unknown> {
    if (auth && !this.#token) throw new TypeError("Shared Auth bearer token required");
    const started = performance.now();
    try {
      const response = await this.#fetch(new URL(path, this.#base), { method, redirect: "error", headers: {accept: "application/json", ...(auth ? {authorization: `Bearer ${this.#token}`} : {}), ...(body === undefined ? {} : {"content-type": "application/json"})}, body: body === undefined ? undefined : JSON.stringify(body) });
      const value = await response.json().catch(() => null);
      if (!response.ok) throw new HappyWakeyError(response.status, value);
      void this.#logger?.info("happy_wakey.client.request").addFields({operation, status: response.status, duration_ms: performance.now() - started}).addTags("happy-wakey", "client").send();
      return value;
    } catch (error) {
      void this.#logger?.error("happy_wakey.client.request").addFields({operation, duration_ms: performance.now() - started}).addTags("happy-wakey", "client").send();
      throw error;
    }
  }
}

