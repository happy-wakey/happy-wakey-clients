const std = @import("std");
pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const module = b.addModule("happy_wakey_client", .{ .root_source_file = b.path("src/root.zig"), .target = target, .optimize = optimize });
    _ = module;
}

