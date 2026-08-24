defmodule HappyWakeyClient.MixProject do
  use Mix.Project
  def project, do: [app: :happy_wakey_client, version: "0.1.0", elixir: "~> 1.17"]
  def application, do: [extra_applications: [:logger]]
end

