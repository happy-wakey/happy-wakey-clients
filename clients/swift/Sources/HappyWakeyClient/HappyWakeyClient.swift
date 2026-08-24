import Foundation
public struct HappyWakeyClient: Sendable {
  public typealias Telemetry = @Sendable (String, Int) -> Void
  let base: URL; let token: String; let session: URLSession; let telemetry: Telemetry
  public init(base: URL, token: String, telemetry: @escaping Telemetry) throws { guard base.scheme == "https" else { throw URLError(.secureConnectionFailed) }; self.base=base; self.token=token; self.telemetry=telemetry; let config=URLSessionConfiguration.ephemeral; self.session=URLSession(configuration: config) }
  public func health() async throws -> Data { try await call("health","GET","/healthz",nil,false) }
  public func listAlarms() async throws -> Data { try await call("list_alarms","GET","/v1/alarms") }
  public func createAlarm(_ json: Data) async throws -> Data { try await call("create_alarm","POST","/v1/alarms",json) }
  public func transitionOccurrence(_ id:String,_ json:Data) async throws -> Data { try await call("transition_occurrence","POST","/v1/occurrences/\(id)/transitions",json) }
  public func pullChanges(cursor:String="0",limit:Int=100) async throws -> Data { try await call("pull_changes","GET","/v1/sync/pull?cursor=\(cursor)&limit=\(limit)") }
  public func pushChanges(_ json:Data) async throws -> Data { try await call("push_changes","POST","/v1/sync/push",json) }
  private func call(_ operation:String,_ method:String,_ path:String,_ body:Data?=nil,_ auth:Bool=true) async throws -> Data { guard !auth || !token.isEmpty else { throw URLError(.userAuthenticationRequired) }; var request=URLRequest(url:URL(string:path,relativeTo:base)!); request.httpMethod=method; request.httpBody=body; request.setValue("application/json",forHTTPHeaderField:"Accept"); if auth { request.setValue("Bearer \(token)",forHTTPHeaderField:"Authorization") }; let (data,response)=try await session.data(for:request); let status=(response as? HTTPURLResponse)?.statusCode ?? 0; telemetry("next-loggers/v1:happy_wakey.client.request:\(operation)",status); guard (200...299).contains(status) else { throw URLError(.badServerResponse) }; return data }
}

