import test from "node:test";
import assert from "node:assert/strict";
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

