import test from "node:test";
import assert from "node:assert/strict";
import { HappyWakeyClient } from "../dist/client.js";

test("uses bearer auth without logging it", async () => {
  const records = [];
  const event = { addFields(fields) { records.push(fields); return this; }, addTags() { return this; }, send() {} };
  const client = new HappyWakeyClient({ baseUrl: "https://api.test", token: "secret", logger: {info: () => event, error: () => event}, fetch: async (_url, init) => { assert.equal(init.headers.authorization, "Bearer secret"); return new Response("[]", {status: 200, headers: {"content-type": "application/json"}}); } });
  assert.deepEqual(await client.listAlarms(), []);
  assert.equal(JSON.stringify(records).includes("secret"), false);
});

