package dev.happywakey;
import java.net.URI;
import java.net.http.*;
import java.time.Duration;
import java.util.function.Consumer;

public final class HappyWakeyClient {
  private final URI base; private final String token; private final HttpClient http; private final Consumer<String> telemetry;
  public HappyWakeyClient(URI base, String token, Consumer<String> telemetry) { if (!"https".equals(base.getScheme())) throw new IllegalArgumentException("HTTPS required"); this.base=base; this.token=token; this.telemetry=telemetry; this.http=HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(5)).followRedirects(HttpClient.Redirect.NEVER).build(); }
  public String health() throws Exception { return call("health","GET","/healthz",null,false); }
  public String listAlarms() throws Exception { return call("list_alarms","GET","/v1/alarms",null,true); }
  public String createAlarm(String json) throws Exception { return call("create_alarm","POST","/v1/alarms",json,true); }
  public String transitionOccurrence(String id,String json) throws Exception { return call("transition_occurrence","POST","/v1/occurrences/"+id+"/transitions",json,true); }
  public String pullChanges(String cursor,int limit) throws Exception { return call("pull_changes","GET","/v1/sync/pull?cursor="+cursor+"&limit="+limit,null,true); }
  public String pushChanges(String json) throws Exception { return call("push_changes","POST","/v1/sync/push",json,true); }
  private String call(String operation,String method,String path,String body,boolean auth) throws Exception { if(auth && token.isBlank()) throw new IllegalStateException("Shared Auth bearer token required"); var builder=HttpRequest.newBuilder(base.resolve(path)).timeout(Duration.ofSeconds(10)).header("Accept","application/json"); if(auth) builder.header("Authorization","Bearer "+token); builder.method(method,body==null?HttpRequest.BodyPublishers.noBody():HttpRequest.BodyPublishers.ofString(body)); var response=http.send(builder.build(),HttpResponse.BodyHandlers.ofString()); telemetry.accept("{\"schema\":\"next-loggers/v1\",\"operation\":\""+operation+"\",\"status\":"+response.statusCode()+"}"); if(response.statusCode()<200||response.statusCode()>299) throw new IllegalStateException("Happy Wakey request failed ("+response.statusCode()+")"); return response.body(); }
}

