#pragma once

#include <functional>
#include <stdexcept>
#include <string>

namespace happy_wakey {
using Transport = std::function<std::string(const std::string &, const std::string &,
                                             const std::string &, const std::string &)>;
using Telemetry = std::function<void(const std::string &, int)>;

class Client {
 public:
  Client(std::string base_url, std::string token, Transport transport, Telemetry telemetry)
      : base_(std::move(base_url)), token_(std::move(token)), transport_(std::move(transport)),
        telemetry_(std::move(telemetry)) {
    if (!base_.starts_with("https://")) throw std::invalid_argument("HTTPS required");
  }
  std::string health() { return call("health", "GET", "/healthz", "", false); }
  std::string list_alarms() { return call("list_alarms", "GET", "/v1/alarms", "", true); }
  std::string create_alarm(const std::string &json) { return call("create_alarm", "POST", "/v1/alarms", json, true); }
  std::string transition_occurrence(const std::string &id, const std::string &json) { return call("transition_occurrence", "POST", "/v1/occurrences/" + id + "/transitions", json, true); }
  std::string pull_changes(const std::string &cursor, unsigned limit) { return call("pull_changes", "GET", "/v1/sync/pull?cursor=" + cursor + "&limit=" + std::to_string(limit), "", true); }
  std::string push_changes(const std::string &json) { return call("push_changes", "POST", "/v1/sync/push", json, true); }

 private:
  std::string call(const std::string &operation, const std::string &method,
                   const std::string &path, const std::string &body, bool auth) {
    try {
      auto result = transport_(method, base_ + path, auth ? "Bearer " + token_ : "", body);
      if (telemetry_) telemetry_("next-loggers/v1:happy_wakey.client.request:" + operation, 200);
      return result;
    } catch (...) {
      if (telemetry_) telemetry_("next-loggers/v1:happy_wakey.client.request:" + operation, 0);
      throw;
    }
  }
  std::string base_, token_;
  Transport transport_;
  Telemetry telemetry_;
};
}  // namespace happy_wakey

