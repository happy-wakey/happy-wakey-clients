#ifndef HAPPY_WAKEY_CLIENT_H
#define HAPPY_WAKEY_CLIENT_H

#include <stddef.h>

typedef int (*hw_transport_fn)(void *context, const char *method, const char *url,
                               const char *authorization, const char *json_body,
                               char **response_json);
typedef void (*hw_telemetry_fn)(void *context, const char *next_loggers_v1_json);

typedef struct {
  const char *base_url;
  const char *bearer_token;
  void *context;
  hw_transport_fn transport;
  hw_telemetry_fn telemetry;
} hw_client;

int hw_health(const hw_client *client, char **response_json);
int hw_list_alarms(const hw_client *client, char **response_json);
int hw_create_alarm(const hw_client *client, const char *request_json, char **response_json);
int hw_transition_occurrence(const hw_client *client, const char *occurrence_id,
                             const char *request_json, char **response_json);
int hw_pull_changes(const hw_client *client, const char *cursor, unsigned limit,
                    char **response_json);
int hw_push_changes(const hw_client *client, const char *request_json, char **response_json);

#endif

