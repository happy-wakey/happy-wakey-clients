require "json"
require "net/http"
require "uri"

class HappyWakeyClient
  def initialize(base_url:, token: nil, telemetry: nil)
    @base = URI(base_url); raise ArgumentError, "HTTPS required" unless @base.scheme == "https"
    @token, @telemetry = token, telemetry
  end
  def health
    call(:health, :get, "/healthz", nil, false)
  end
  def list_alarms
    call(:list_alarms, :get, "/v1/alarms")
  end
  def create_alarm(request)
    call(:create_alarm, :post, "/v1/alarms", request)
  end
  def transition_occurrence(id, request)
    call(:transition_occurrence, :post, "/v1/occurrences/#{URI.encode_www_form_component(id)}/transitions", request)
  end
  def pull_changes(cursor: "0", limit: 100)
    call(:pull_changes, :get, "/v1/sync/pull?#{URI.encode_www_form(cursor: cursor, limit: limit)}")
  end
  def push_changes(changes)
    call(:push_changes, :post, "/v1/sync/push", changes)
  end
  private
  def call(operation, method, path, body = nil, auth = true)
    raise ArgumentError, "Shared Auth bearer token required" if auth && (@token.nil? || @token.empty?)
    uri = @base + path; request = method == :get ? Net::HTTP::Get.new(uri) : Net::HTTP::Post.new(uri)
    request["Accept"] = "application/json"; request["Authorization"] = "Bearer #{@token}" if auth
    request["Content-Type"], request.body = "application/json", JSON.generate(body) unless body.nil?
    response = Net::HTTP.start(uri.host, uri.port, use_ssl: true, open_timeout: 5, read_timeout: 10) { |http| http.request(request) }
    @telemetry&.info("happy_wakey.client.request", operation: operation, status: response.code.to_i)
    raise "Happy Wakey request failed (#{response.code})" unless response.is_a?(Net::HTTPSuccess)
    JSON.parse(response.body)
  rescue => error
    @telemetry&.error("happy_wakey.client.request", operation: operation)
    raise error
  end
end
