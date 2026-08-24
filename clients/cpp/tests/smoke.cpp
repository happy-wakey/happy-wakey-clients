#include "happy_wakey/client.hpp"

int main() {
  happy_wakey::Client client(
      "https://api.happy-wakey.dev", "test",
      [](const auto &, const auto &, const auto &, const auto &) { return "{}"; },
      [](const auto &, int) {});
  return client.health() == "{}" ? 0 : 1;
}

