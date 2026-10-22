import { describe, expect, it } from "vitest";
import type { AppStatus } from "../src/types/app";

describe("AppStatus contract", () => {
  it("represents the Task 01 backend state", () => {
    const status: AppStatus = {
      appName: "VoiceMorph Studio",
      version: "0.1.0",
      platform: "windows-x86_64",
      audioEngineReady: false,
    };

    expect(status.appName).toBe("VoiceMorph Studio");
    expect(status.audioEngineReady).toBe(false);
  });
});
