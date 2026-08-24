const std = @import("std");

pub const Transport = *const fn (method: []const u8, url: []const u8, authorization: ?[]const u8, body: ?[]const u8) anyerror![]const u8;
pub const Telemetry = *const fn (operation: []const u8, ok: bool) void;

pub const Client = struct {
    allocator: std.mem.Allocator,
    base_url: []const u8,
    token: []const u8,
    transport: Transport,
    telemetry: Telemetry,

    fn call(self: Client, operation: []const u8, method: []const u8, path: []const u8, body: ?[]const u8, auth: bool) ![]const u8 {
        if (!std.mem.startsWith(u8, self.base_url, "https://")) return error.HttpsRequired;
        const url = try std.fmt.allocPrint(self.allocator, "{s}{s}", .{ self.base_url, path });
        defer self.allocator.free(url);
        const authorization = if (auth) try std.fmt.allocPrint(self.allocator, "Bearer {s}", .{self.token}) else null;
        defer if (authorization) |value| self.allocator.free(value);
        const result = self.transport(method, url, authorization, body) catch |err| { self.telemetry(operation, false); return err; };
        self.telemetry(operation, true); // Ores next-loggers/v1 telemetry adapter
        return result;
    }
    pub fn health(self: Client) ![]const u8 { return self.call("health", "GET", "/healthz", null, false); }
    pub fn list_alarms(self: Client) ![]const u8 { return self.call("list_alarms", "GET", "/v1/alarms", null, true); }
    pub fn create_alarm(self: Client, json: []const u8) ![]const u8 { return self.call("create_alarm", "POST", "/v1/alarms", json, true); }
    pub fn transition_occurrence(self: Client, id: []const u8, json: []const u8) ![]const u8 { const path = try std.fmt.allocPrint(self.allocator, "/v1/occurrences/{s}/transitions", .{id}); defer self.allocator.free(path); return self.call("transition_occurrence", "POST", path, json, true); }
    pub fn pull_changes(self: Client, cursor: []const u8, limit: u16) ![]const u8 { const path = try std.fmt.allocPrint(self.allocator, "/v1/sync/pull?cursor={s}&limit={d}", .{cursor, limit}); defer self.allocator.free(path); return self.call("pull_changes", "GET", path, null, true); }
    pub fn push_changes(self: Client, json: []const u8) ![]const u8 { return self.call("push_changes", "POST", "/v1/sync/push", json, true); }
};

