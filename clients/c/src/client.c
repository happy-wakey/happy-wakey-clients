#include "happy_wakey/client.h"

#include <stdio.h>
#include <string.h>

static int request(const hw_client *client, const char *operation, const char *method,
                   const char *path, const char *body, int authenticated,
                   char **response_json) {
  char url[2048];
  char authorization[1024];
  if (!client || !client->base_url || !client->transport ||
      strncmp(client->base_url, "https://", 8) != 0) return -1;
  if (snprintf(url, sizeof(url), "%s%s", client->base_url, path) >= (int)sizeof(url)) return -2;
  authorization[0] = '\0';
  if (authenticated) {
    if (!client->bearer_token || snprintf(authorization, sizeof(authorization),
        "Bearer %s", client->bearer_token) >= (int)sizeof(authorization)) return -3;
  }
  int status = client->transport(client->context, method, url,
      authenticated ? authorization : NULL, body, response_json);
  if (client->telemetry) {
    char event[512];
    snprintf(event, sizeof(event),
      "{\"schema\":\"next-loggers/v1\",\"message\":\"happy_wakey.client.request\",\"fields\":{\"operation\":\"%s\",\"status\":%d}}",
      operation, status);
    client->telemetry(client->context, event);
  }
  return status;
}

int hw_health(const hw_client *c, char **out) { return request(c, "health", "GET", "/healthz", NULL, 0, out); }
int hw_list_alarms(const hw_client *c, char **out) { return request(c, "list_alarms", "GET", "/v1/alarms", NULL, 1, out); }
int hw_create_alarm(const hw_client *c, const char *body, char **out) { return request(c, "create_alarm", "POST", "/v1/alarms", body, 1, out); }
int hw_transition_occurrence(const hw_client *c, const char *id, const char *body, char **out) {
  char path[512];
  if (!id || snprintf(path, sizeof(path), "/v1/occurrences/%s/transitions", id) >= (int)sizeof(path)) return -2;
  return request(c, "transition_occurrence", "POST", path, body, 1, out);
}
int hw_pull_changes(const hw_client *c, const char *cursor, unsigned limit, char **out) {
  char path[512];
  if (snprintf(path, sizeof(path), "/v1/sync/pull?cursor=%s&limit=%u", cursor ? cursor : "0", limit) >= (int)sizeof(path)) return -2;
  return request(c, "pull_changes", "GET", path, NULL, 1, out);
}
int hw_push_changes(const hw_client *c, const char *body, char **out) { return request(c, "push_changes", "POST", "/v1/sync/push", body, 1, out); }

