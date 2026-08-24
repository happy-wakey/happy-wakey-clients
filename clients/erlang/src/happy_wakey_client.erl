-module(happy_wakey_client).
-export([health/1, list_alarms/1, create_alarm/2, transition_occurrence/3, pull_changes/2, push_changes/2]).
call(#{base_url := Base, authorization := Authorization, transport := Transport, telemetry := Telemetry}, Operation, Method, Path, Body, Auth) -> Header = case Auth of true -> Authorization; false -> <<>> end, Result = Transport(Method, <<Base/binary, Path/binary>>, Header, Body), Telemetry(#{schema => <<"next-loggers/v1">>, operation => Operation, succeeded => element(1, Result) =:= ok}), Result.
health(C) -> call(C, health, get, <<"/healthz">>, <<>>, false).
list_alarms(C) -> call(C, list_alarms, get, <<"/v1/alarms">>, <<>>, true).
create_alarm(C, J) -> call(C, create_alarm, post, <<"/v1/alarms">>, J, true).
transition_occurrence(C, Id, J) -> call(C, transition_occurrence, post, <<"/v1/occurrences/", Id/binary, "/transitions">>, J, true).
pull_changes(C, Cursor) -> call(C, pull_changes, get, <<"/v1/sync/pull?cursor=", Cursor/binary>>, <<>>, true).
push_changes(C, J) -> call(C, push_changes, post, <<"/v1/sync/push">>, J, true).

