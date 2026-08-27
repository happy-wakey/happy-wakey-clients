export class HappyWakeyError extends Error {
    status;
    body;
    constructor(status, body) {
        super(`Happy Wakey request failed (${status})`);
        this.status = status;
        this.body = body;
    }
}
export class HappyWakeyClient {
    #base;
    #token;
    #logger;
    #fetch;
    constructor(options) {
        this.#base = new URL(options.baseUrl);
        let host = this.#base.hostname.toLowerCase();
        if (host.startsWith("[") && host.endsWith("]"))
            host = host.slice(1, -1);
        const loopback = ["localhost", "127.0.0.1", "::1"].includes(host);
        const numericIp = /^(?:\d{1,3}\.){3}\d{1,3}$/.test(host) || host.includes(":");
        if (numericIp && !loopback)
            throw new TypeError("public IP literals are not allowed");
        if (this.#base.protocol !== "https:" && !(options.allowInsecureLoopback && loopback))
            throw new TypeError("HTTPS required");
        this.#token = options.token;
        this.#logger = options.logger;
        this.#fetch = options.fetch ?? globalThis.fetch;
    }
    health() { return this.#request("health", "GET", "/healthz", undefined, false); }
    listAlarms() { return this.#request("listAlarms", "GET", "/v1/alarms"); }
    createAlarm(request) { return this.#request("createAlarm", "POST", "/v1/alarms", request); }
    transitionOccurrence(id, request) { return this.#request("transitionOccurrence", "POST", `/v1/occurrences/${encodeURIComponent(id)}/transitions`, request); }
    pullChanges(cursor = "0", limit = 100) { const query = new URLSearchParams({ cursor, limit: String(limit) }); return this.#request("pullChanges", "GET", `/v1/sync/pull?${query}`); }
    pushChanges(changes) { return this.#request("pushChanges", "POST", "/v1/sync/push", changes); }
    async #request(operation, method, path, body, auth = true) {
        if (auth && !this.#token)
            throw new TypeError("Shared Auth bearer token required");
        const started = performance.now();
        try {
            const response = await this.#fetch(new URL(path, this.#base), { method, redirect: "error", headers: { accept: "application/json", ...(auth ? { authorization: `Bearer ${this.#token}` } : {}), ...(body === undefined ? {} : { "content-type": "application/json" }) }, body: body === undefined ? undefined : JSON.stringify(body) });
            const value = await response.json().catch(() => null);
            if (!response.ok)
                throw new HappyWakeyError(response.status, value);
            void this.#logger?.info("happy_wakey.client.request").addFields({ operation, status: response.status, duration_ms: performance.now() - started }).addTags("happy-wakey", "client").send();
            return value;
        }
        catch (error) {
            void this.#logger?.error("happy_wakey.client.request").addFields({ operation, duration_ms: performance.now() - started }).addTags("happy-wakey", "client").send();
            throw error;
        }
    }
}
