import { defer, EMPTY, exhaustMap, fromEvent, take, takeUntil, timer, } from "rxjs";
export class HappyWakeyError extends Error {
    status;
    body;
    constructor(status, body) {
        super(`Happy Wakey request failed (${status})`);
        this.status = status;
        this.body = body;
    }
}
export function validateBaseUrl(options) {
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
export function asObservable(operation) {
    return defer(operation);
}
function aborts(signal) {
    if (signal === undefined)
        return EMPTY;
    if (signal.aborted)
        return defer(() => [undefined]);
    return fromEvent(signal, "abort").pipe(take(1));
}
function pollInterval(value = 30_000) {
    if (!Number.isSafeInteger(value) || value < 10) {
        throw new RangeError("poll interval must be a safe integer of at least 10ms");
    }
    return value;
}
export class HappyWakeyClient {
    #base;
    #token;
    #logger;
    #fetch;
    constructor(options) {
        this.#base = validateBaseUrl(options);
        this.#token = options.token;
        this.#logger = options.logger;
        this.#fetch = options.fetch ?? globalThis.fetch;
    }
    health() {
        return this.#request("health", "GET", "/healthz", undefined, false);
    }
    listAlarms(options = {}) {
        return this.#request("listAlarms", "GET", "/v1/alarms", undefined, true, options.signal);
    }
    createAlarm(request) {
        return this.#request("createAlarm", "POST", "/v1/alarms", request);
    }
    transitionOccurrence(id, request) {
        return this.#request("transitionOccurrence", "POST", `/v1/occurrences/${encodeURIComponent(id)}/transitions`, request);
    }
    pullChanges(cursor = "0", limit = 100) {
        const query = new URLSearchParams({ cursor, limit: String(limit) });
        return this.#request("pullChanges", "GET", `/v1/sync/pull?${query}`);
    }
    pushChanges(changes) {
        return this.#request("pushChanges", "POST", "/v1/sync/push", changes);
    }
    /// Cold stream: no request or credential use occurs before subscription.
    observeAlarms(options = {}) {
        return asObservable(() => this.listAlarms(options));
    }
    /// Non-overlapping polling. Slow requests are never duplicated; aborting the
    /// supplied signal completes the stream and allows `fetch` cancellation to
    /// remain an explicit caller-owned concern.
    watchAlarms(options = {}) {
        const intervalMs = pollInterval(options.intervalMs);
        if (options.signal?.aborted)
            return EMPTY;
        return timer(0, intervalMs).pipe(exhaustMap(() => this.observeAlarms({ signal: options.signal })), takeUntil(aborts(options.signal)));
    }
    async #request(operation, method, path, body, auth = true, signal) {
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
            const value = await response.json().catch(() => null);
            if (!response.ok)
                throw new HappyWakeyError(response.status, value);
            void this.#logger
                ?.info("happy_wakey.client.request")
                .addFields({
                operation,
                status: response.status,
                duration_ms: performance.now() - started,
            })
                .addTags("happy-wakey", "client")
                .send();
            return value;
        }
        catch (error) {
            void this.#logger
                ?.error("happy_wakey.client.request")
                .addFields({ operation, duration_ms: performance.now() - started })
                .addTags("happy-wakey", "client")
                .send();
            throw error;
        }
    }
}
