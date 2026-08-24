export interface OresTelemetryEvent {
    addFields(fields: Record<string, unknown>): OresTelemetryEvent;
    addTags(...tags: string[]): OresTelemetryEvent;
    send(): void | Promise<void>;
}
export interface OresTelemetryLogger {
    info(...values: unknown[]): OresTelemetryEvent;
    error(...values: unknown[]): OresTelemetryEvent;
}
export interface ClientOptions {
    baseUrl: string;
    token?: string;
    logger?: OresTelemetryLogger;
    fetch?: typeof fetch;
    allowInsecureLoopback?: boolean;
}
export declare class HappyWakeyError extends Error {
    readonly status: number;
    readonly body: unknown;
    constructor(status: number, body: unknown);
}
export declare class HappyWakeyClient {
    #private;
    constructor(options: ClientOptions);
    health(): Promise<unknown>;
    listAlarms(): Promise<unknown>;
    createAlarm(request: unknown): Promise<unknown>;
    transitionOccurrence(id: string, request: unknown): Promise<unknown>;
    pullChanges(cursor?: string, limit?: number): Promise<unknown>;
    pushChanges(changes: unknown[]): Promise<unknown>;
}
