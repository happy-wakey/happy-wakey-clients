import test from "node:test";
import assert from "node:assert/strict";
import { firstValueFrom } from "rxjs";
import { HappyWakeyClient } from "../dist/client.js";

test("rejects public IP literals", () => {
  assert.throws(
    () => new HappyWakeyClient({ baseUrl: "https://98.90.186.114" }),
    /public IP literals are not allowed/,
  );
});

test("loopback IPv6 is allowed and is not treated as a public IP", () => {
  assert.doesNotThrow(() => new HappyWakeyClient({ baseUrl: "https://[::1]" }));
});

test("uses bearer auth without logging it", async () => {
  const records = [];
  const event = { addFields(fields) { records.push(fields); return this; }, addTags() { return this; }, send() {} };
  const client = new HappyWakeyClient({ baseUrl: "https://api.test", token: "secret", logger: {info: () => event, error: () => event}, fetch: async (_url, init) => { assert.equal(init.headers.authorization, "Bearer secret"); return new Response("[]", {status: 200, headers: {"content-type": "application/json"}}); } });
  assert.deepEqual(await client.listAlarms(), []);
  assert.equal(JSON.stringify(records).includes("secret"), false);
});

test("RxJS alarm requests are cold until subscription", async () => {
  let calls = 0;
  const client = new HappyWakeyClient({
    baseUrl: "https://api.test",
    token: "secret",
    fetch: async () => {
      calls += 1;
      return new Response("[]", {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  });

  const alarms$ = client.observeAlarms();
  assert.equal(calls, 0);
  assert.deepEqual(await firstValueFrom(alarms$), []);
  assert.equal(calls, 1);
});

test("RxJS polling exhausts overlaps and aborts explicitly", async () => {
  let active = 0;
  let maximumActive = 0;
  let calls = 0;
  const controller = new AbortController();
  const client = new HappyWakeyClient({
    baseUrl: "https://api.test",
    token: "secret",
    fetch: async (_url, init) => {
      calls += 1;
      active += 1;
      maximumActive = Math.max(maximumActive, active);
      try {
        await new Promise((resolve, reject) => {
          const timeout = setTimeout(resolve, 25);
          init.signal?.addEventListener(
            "abort",
            () => {
              clearTimeout(timeout);
              reject(new DOMException("aborted", "AbortError"));
            },
            { once: true },
          );
        });
        return new Response("[]", { status: 200 });
      } finally {
        active -= 1;
      }
    },
  });
  const values = [];
  const subscription = client
    .watchAlarms({ intervalMs: 10, signal: controller.signal })
    .subscribe({ next: (value) => values.push(value), error: () => {} });

  await new Promise((resolve) => setTimeout(resolve, 65));
  controller.abort();
  await new Promise((resolve) => setTimeout(resolve, 5));
  subscription.unsubscribe();

  assert.equal(maximumActive, 1);
  assert.ok(calls >= 2);
  assert.ok(values.length >= 1);
});
