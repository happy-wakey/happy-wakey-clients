from __future__ import annotations

import json
from urllib.parse import quote, urlencode, urljoin, urlparse
from urllib.request import HTTPRedirectHandler, Request, build_opener
from urllib.error import HTTPError


class HappyWakeyError(RuntimeError):
    def __init__(self, status: int, body: object):
        super().__init__(f"Happy Wakey request failed ({status})")
        self.status, self.body = status, body


class HappyWakeyClient:
    def __init__(self, base_url: str, token: str | None = None, logger=None, allow_insecure_loopback: bool = False):
        parsed = urlparse(base_url)
        if parsed.scheme != "https" and not (allow_insecure_loopback and parsed.hostname in {"localhost", "127.0.0.1", "::1"}):
            raise ValueError("HTTPS required")
        self.base_url, self.token, self.logger = base_url.rstrip("/") + "/", token, logger
        self.opener = build_opener(HTTPRedirectHandler())

    def health(self): return self._request("health", "GET", "healthz", auth=False)
    def list_alarms(self): return self._request("list_alarms", "GET", "v1/alarms")
    def create_alarm(self, request): return self._request("create_alarm", "POST", "v1/alarms", request)
    def transition_occurrence(self, occurrence_id, request): return self._request("transition_occurrence", "POST", f"v1/occurrences/{quote(occurrence_id, safe='')}/transitions", request)
    def pull_changes(self, cursor="0", limit=100): return self._request("pull_changes", "GET", "v1/sync/pull?" + urlencode({"cursor": cursor, "limit": limit}))
    def push_changes(self, changes): return self._request("push_changes", "POST", "v1/sync/push", changes)

    def _request(self, operation, method, path, body=None, auth=True):
        if auth and not self.token: raise ValueError("Shared Auth bearer token required")
        headers = {"Accept": "application/json"}
        if auth: headers["Authorization"] = f"Bearer {self.token}"
        data = None if body is None else json.dumps(body).encode()
        if data is not None: headers["Content-Type"] = "application/json"
        try:
            with self.opener.open(Request(urljoin(self.base_url, path), data=data, headers=headers, method=method), timeout=10) as response:
                value = json.loads(response.read())
                if self.logger: self.logger.info("happy_wakey.client.request").add_fields({"operation": operation, "status": response.status}).send()
                return value
        except HTTPError as error:
            if self.logger: self.logger.error("happy_wakey.client.request").add_fields({"operation": operation, "status": error.code}).send()
            try: value = json.loads(error.read())
            except Exception: value = None
            raise HappyWakeyError(error.code, value) from error

