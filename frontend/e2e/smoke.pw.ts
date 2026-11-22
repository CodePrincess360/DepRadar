import { expect, test } from "@playwright/test";

test("dashboard renders against the testnet RPC", async ({ page, request }) => {
  const health = await request.post("https://soroban-testnet.stellar.org", {
    data: { jsonrpc: "2.0", id: 1, method: "getHealth" },
  });
  expect(health.ok()).toBe(true);
  expect((await health.json()).result.status).toBe("healthy");
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Bounties", exact: true })).toBeVisible();
});
