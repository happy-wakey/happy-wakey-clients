import { Observable } from "rxjs";
export type JsonPrimitive = boolean | number | string | null;
export type JsonValue = JsonPrimitive | readonly JsonValue[] | {
    readonly [key: string]: JsonValue;
};
export interface Alarm {
    readonly id: string;
    readonly label: string;
    readonly local_time: string;
    readonly time_zone: string;
    readonly weekdays: readonly number[];
    readonly enabled: boolean;
    readonly sound: string;
    readonly volume: number;
    readonly gradual_seconds: number;
    readonly tags: readonly string[];
    readonly generation: number;
    readonly created_at: string;
    readonly updated_at: string;
}
export interface CreateAlarmRequest extends Omit<Alarm, "id" | "generation" | "created_at" | "updated_at"> {
    readonly transition_id: string;
}
export type AlarmTransitionEvent = "fire" | "acknowledge" | "snooze" | "complete" | "mark_missed" | "cancel";
export interface TransitionAlarmRequest {
    readonly transition_id: string;
    readonly expected_generation: number;
    readonly event: AlarmTransitionEvent;
    readonly snooze_until: string | null;
    readonly client_time: string;
}
export interface TransitionAlarmResponse {
    readonly disposition: "applied" | "stale" | "rejected";
    readonly occurrence: JsonValue;
    readonly error: JsonValue | null;
}
export interface SyncChange {
    readonly change_id: string;
    readonly scope: string;
    readonly collection: string;
    readonly entity_id: string;
    readonly operation: "upsert" | "delete";
    readonly generation: number;
    readonly actor_id: string;
    readonly document: JsonValue | null;
    readonly occurred_at: string;
}
export interface SyncEnvelope {
    readonly schema: string;
    readonly cursor: string;
    readonly changes: readonly SyncChange[];
    readonly has_more: boolean;
}
export interface OresTelemetryEvent {
    addFields(fields: Readonly<Record<string, unknown>>): OresTelemetryEvent;
    addTags(...tags: readonly string[]): OresTelemetryEvent;
    send(): void | Promise<void>;
}
export interface OresTelemetryLogger {
    info(...values: readonly unknown[]): OresTelemetryEvent;
    error(...values: readonly unknown[]): OresTelemetryEvent;
}
export interface ClientOptions {
    readonly baseUrl: string;
    readonly token?: string;
    readonly logger?: OresTelemetryLogger;
    readonly fetch?: typeof fetch;
    readonly allowInsecureLoopback?: boolean;
}
export interface PollOptions {
    readonly intervalMs?: number;
    readonly signal?: AbortSignal;
}
export interface RequestOptions {
    readonly signal?: AbortSignal;
}
export declare class HappyWakeyError extends Error {
    readonly status: number;
    readonly body: unknown;
    constructor(status: number, body: unknown);
}
export declare function validateBaseUrl(options: ClientOptions): URL;
export declare function asObservable<T>(operation: () => Promise<T>): Observable<T>;
export declare class HappyWakeyClient {
    #private;
    constructor(options: ClientOptions);
    health(): Promise<{
        readonly status: "ok";
    }>;
    listAlarms(options?: RequestOptions): Promise<readonly Alarm[]>;
    createAlarm(request: CreateAlarmRequest): Promise<Alarm>;
    transitionOccurrence(id: string, request: TransitionAlarmRequest): Promise<TransitionAlarmResponse>;
    pullChanges(cursor?: string, limit?: number): Promise<SyncEnvelope>;
    pushChanges(changes: readonly SyncChange[]): Promise<SyncEnvelope>;
    observeAlarms(options?: RequestOptions): Observable<readonly Alarm[]>;
    watchAlarms(options?: PollOptions): Observable<readonly Alarm[]>;
}
