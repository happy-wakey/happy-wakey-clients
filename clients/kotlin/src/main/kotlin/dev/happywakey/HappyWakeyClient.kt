package dev.happywakey
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse

class HappyWakeyClient(private val base: URI, private val token: String, private val telemetry: (String, Int) -> Unit) {
  private val http = HttpClient.newBuilder().followRedirects(HttpClient.Redirect.NEVER).build()
  init { require(base.scheme == "https") { "HTTPS required" } }
  fun health() = call("health", "GET", "/healthz", null, false)
  fun listAlarms() = call("list_alarms", "GET", "/v1/alarms")
  fun createAlarm(json: String) = call("create_alarm", "POST", "/v1/alarms", json)
  fun transitionOccurrence(id: String, json: String) = call("transition_occurrence", "POST", "/v1/occurrences/$id/transitions", json)
  fun pullChanges(cursor: String = "0", limit: Int = 100) = call("pull_changes", "GET", "/v1/sync/pull?cursor=$cursor&limit=$limit")
  fun pushChanges(json: String) = call("push_changes", "POST", "/v1/sync/push", json)
  private fun call(operation:String,method:String,path:String,body:String?=null,auth:Boolean=true):String { if(auth) require(token.isNotBlank()){"Shared Auth bearer token required"}; val builder=HttpRequest.newBuilder(base.resolve(path)).header("Accept","application/json"); if(auth) builder.header("Authorization","Bearer $token"); builder.method(method,body?.let{HttpRequest.BodyPublishers.ofString(it)}?:HttpRequest.BodyPublishers.noBody()); val response=http.send(builder.build(),HttpResponse.BodyHandlers.ofString()); telemetry("next-loggers/v1:happy_wakey.client.request:$operation",response.statusCode()); check(response.statusCode() in 200..299){"Happy Wakey request failed (${response.statusCode()})"}; return response.body() }
}

