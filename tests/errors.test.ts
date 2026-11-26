import { describe, it, expect } from "vitest";
import { errorMessage } from "../src/services/errors";
describe("native error messages", () => {
  it("preserves structured Tauri details instead of object coercion", () => {
    expect(
      errorMessage({
        code: "backend_error",
        message: "Start audio processing before playing a sound",
      }),
    ).toBe("Start audio processing before playing a sound");
    expect(errorMessage(new Error("File missing"))).toBe("File missing");
    expect(errorMessage("Queue full")).toBe("Queue full");
    expect(errorMessage({})).not.toContain("[object Object]");
  });
});
