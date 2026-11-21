/**
 * Read-only access to the deprader_router contract. Every call is a
 * simulated `queryContract` — nothing is signed or submitted, and no wallet
 * is needed.
 */
import { rpc } from "@stellar/stellar-sdk";
import { parseBounty, parseClaim } from "@deprader/contract-types";
import { CONTRACT_ID, NETWORK, RPC_URL } from "../config";
import type { Bounty, Claim } from "./types";

export { parseBounty, parseClaim };

/** Contract address and network, exposed for display (e.g. the About page). */
export { CONTRACT_ID, NETWORK };

const server = new rpc.Server(RPC_URL);

async function query(
  method: string,
  args: Record<string, unknown> = {},
  contractId: string = CONTRACT_ID,
): Promise<unknown> {
  const { result } = await server.queryContract(contractId, method, args);
  return result;
}

/** Display metadata read from a token contract (SAC or SEP-41). */
export interface TokenInfo {
  symbol: string;
  decimals: number;
}

const tokenInfoCache = new Map<string, Promise<TokenInfo>>();

/**
 * `symbol()` and `decimals()` of a token contract, cached per token. Failed
 * lookups are not cached, so a later call can retry. Callers should fall back
 * to the defaults when this rejects.
 */
export function getTokenInfo(token: string): Promise<TokenInfo> {
  const cached = tokenInfoCache.get(token);
  if (cached) return cached;

  const pending = Promise.all([query("symbol", {}, token), query("decimals", {}, token)]).then(
    ([symbol, decimals]) => {
      const places = Number(decimals);
      if (!Number.isInteger(places) || places < 0 || places > 38) {
        throw new Error(`Unexpected decimals for token ${token}: ${String(decimals)}`);
      }
      return { symbol: String(symbol), decimals: places };
    },
  );
  tokenInfoCache.set(token, pending);
  pending.catch(() => tokenInfoCache.delete(token));
  return pending;
}

type Raw = Record<string, unknown>;

/** Address of the current contract admin. */
export async function getAdmin(): Promise<string> {
  return String(await query("get_admin"));
}

/** Address of the pending admin, or `null` when no transfer is in progress. */
export async function getPendingAdmin(): Promise<string | null> {
  const raw = await query("get_pending_admin");
  return raw == null ? null : String(raw);
}

export async function getBountyCount(): Promise<number> {
  return Number(await query("bounty_count"));
}

export async function getBounty(id: number): Promise<Bounty | null> {
  const raw = await query("get_bounty", { bounty_id: id });
  return raw == null ? null : parseBounty(id, raw as Raw);
}

export async function getClaim(bountyId: number, claimId: number): Promise<Claim | null> {
  const raw = await query("get_claim", { bounty_id: bountyId, claim_id: claimId });
  return raw == null ? null : parseClaim(bountyId, claimId, raw as Raw);
}

/** All bounties, newest first. Bounty IDs are `1..=bounty_count`. */
export async function listBounties(): Promise<Bounty[]> {
  const count = await getBountyCount();
  const ids = Array.from({ length: count }, (_, i) => count - i);
  const bounties = await Promise.all(ids.map(getBounty));
  return bounties.filter((b): b is Bounty => b !== null);
}

/** Every claim on a bounty, in submission order. Claim IDs are `0..claim_count`. */
export async function listClaims(bounty: Bounty): Promise<Claim[]> {
  const ids = Array.from({ length: bounty.claimCount }, (_, i) => i);
  const claims = await Promise.all(ids.map((id) => getClaim(bounty.id, id)));
  return claims.filter((c): c is Claim => c !== null);
}
