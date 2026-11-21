import { beforeEach, describe, expect, it, vi } from "vitest";

const queryContract = vi.fn();

vi.mock("@stellar/stellar-sdk", () => ({
  rpc: {
    Server: class {
      queryContract = queryContract;
    },
  },
}));

const SIX_DECIMAL_TOKEN = "CBTESTTOKEN6DECIMALSXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";

describe("getTokenInfo", () => {
  beforeEach(() => {
    queryContract.mockReset();
    vi.resetModules();
  });

  it("reads symbol and decimals from the token contract", async () => {
    queryContract.mockImplementation(async (_id: string, method: string) => ({
      result: method === "symbol" ? "USDC" : 6,
    }));
    const { getTokenInfo } = await import("./contract");
    await expect(getTokenInfo(SIX_DECIMAL_TOKEN)).resolves.toEqual({ symbol: "USDC", decimals: 6 });
    expect(queryContract).toHaveBeenCalledWith(SIX_DECIMAL_TOKEN, "symbol", {});
    expect(queryContract).toHaveBeenCalledWith(SIX_DECIMAL_TOKEN, "decimals", {});
  });

  it("caches successful lookups per token", async () => {
    queryContract.mockImplementation(async (_id: string, method: string) => ({
      result: method === "symbol" ? "USDC" : 6,
    }));
    const { getTokenInfo } = await import("./contract");
    await getTokenInfo(SIX_DECIMAL_TOKEN);
    await getTokenInfo(SIX_DECIMAL_TOKEN);
    expect(queryContract).toHaveBeenCalledTimes(2); // symbol + decimals, once
  });

  it("rejects when the token call fails and does not cache the failure", async () => {
    queryContract.mockRejectedValueOnce(new Error("rpc down")).mockRejectedValueOnce(new Error("rpc down"));
    const { getTokenInfo } = await import("./contract");
    await expect(getTokenInfo(SIX_DECIMAL_TOKEN)).rejects.toThrow("rpc down");
    queryContract.mockImplementation(async (_id: string, method: string) => ({
      result: method === "symbol" ? "USDC" : 6,
    }));
    await expect(getTokenInfo(SIX_DECIMAL_TOKEN)).resolves.toEqual({ symbol: "USDC", decimals: 6 });
  });
});
