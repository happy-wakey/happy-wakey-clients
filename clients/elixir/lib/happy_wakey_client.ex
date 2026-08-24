defmodule HappyWakeyClient do
  defstruct [:base_url, :authorization, :transport, :telemetry]
  defp call(c, operation, method, path, body \\ nil, auth \\ true) do
    header = if auth, do: c.authorization, else: nil
    result = c.transport.(method, c.base_url <> path, header, body)
    c.telemetry.(%{schema: "next-loggers/v1", message: "happy_wakey.client.request", operation: operation, succeeded: match?({:ok, _}, result)})
    result
  end
  def health(c), do: call(c, :health, :get, "/healthz", nil, false)
  def list_alarms(c), do: call(c, :list_alarms, :get, "/v1/alarms")
  def create_alarm(c, json), do: call(c, :create_alarm, :post, "/v1/alarms", json)
  def transition_occurrence(c, id, json), do: call(c, :transition_occurrence, :post, "/v1/occurrences/#{URI.encode(id)}/transitions", json)
  def pull_changes(c, cursor \\ "0"), do: call(c, :pull_changes, :get, "/v1/sync/pull?cursor=#{URI.encode_www_form(cursor)}")
  def push_changes(c, json), do: call(c, :push_changes, :post, "/v1/sync/push", json)
end

