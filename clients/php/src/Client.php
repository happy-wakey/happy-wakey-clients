<?php
declare(strict_types=1);
namespace HappyWakey;

final class Client {
  public function __construct(private string $baseUrl, private string $token, private \Closure $transport, private \Closure $telemetry) {
    if (!str_starts_with($baseUrl, 'https://')) throw new \InvalidArgumentException('HTTPS required');
  }
  public function health(): mixed { return $this->call('health', 'GET', '/healthz', null, false); }
  public function listAlarms(): mixed { return $this->call('list_alarms', 'GET', '/v1/alarms'); }
  public function createAlarm(array $request): mixed { return $this->call('create_alarm', 'POST', '/v1/alarms', $request); }
  public function transitionOccurrence(string $id, array $request): mixed { return $this->call('transition_occurrence', 'POST', '/v1/occurrences/'.rawurlencode($id).'/transitions', $request); }
  public function pullChanges(string $cursor = '0', int $limit = 100): mixed { return $this->call('pull_changes', 'GET', '/v1/sync/pull?'.http_build_query(['cursor'=>$cursor,'limit'=>$limit])); }
  public function pushChanges(array $changes): mixed { return $this->call('push_changes', 'POST', '/v1/sync/push', $changes); }
  private function call(string $operation, string $method, string $path, ?array $body = null, bool $auth = true): mixed {
    if ($auth && $this->token === '') throw new \LogicException('Shared Auth bearer token required');
    try { $result = ($this->transport)($method, $this->baseUrl.$path, ['accept'=>'application/json'] + ($auth ? ['authorization'=>'Bearer '.$this->token] : []), $body); ($this->telemetry)(['schema'=>'next-loggers/v1','message'=>'happy_wakey.client.request','fields'=>['operation'=>$operation,'status'=>200]]); return $result; }
    catch (\Throwable $error) { ($this->telemetry)(['schema'=>'next-loggers/v1','message'=>'happy_wakey.client.request','fields'=>['operation'=>$operation,'status'=>0]]); throw $error; }
  }
}

