import { describe, expect, it } from "vitest";
import { parseArgs, proofHex } from "./index";

describe("bounty report proofs", () => {
  it("looks up the proof only when requested", () => {
    expect(parseArgs([], {}).proof).toBe(false);
    expect(parseArgs(["--proof", "--bounty-id", "2"], {}).proof).toBe(true);
  });

  it("formats a 32-byte digest and rejects malformed values", () => {
    expect(proofHex(Uint8Array.from({ length: 32 }, () => 42))).toBe("2a".repeat(32));
    expect(proofHex(null)).toBeNull();
    expect(() => proofHex(new Uint8Array(31))).toThrow("Invalid bounty proof digest");
  });
});
