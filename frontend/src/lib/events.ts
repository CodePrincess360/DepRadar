/**
 * Recent activity from the router's contract events, decoded into readable
 * lines. Event shapes follow the `env.events().publish(...)` calls in
 * `contracts/deprader_router/src/lib.rs`.
 */
import { scValToNative, type rpc } from "@stellar/stellar-sdk";
import { CONTRACT_ID } from "../config";
import { getBounty, server } from "./contract";
import { formatAmount, tokenSymbol } from "./format";

/** About one day of ledgers (~5s each). Older events may be outside RPC retention. */
const LOOKBACK_LEDGERS = 17_280;
/** Upper bound on events pulled per poll. */
const EVENT_LIMIT = 1_000;

export interface ActivityItem {
  id: string;
  txHash: string;
  /** Ledger close time as an ISO string, when the RPC reports it. */
  at: string | null;
  text: string;
}

/** Decoded event before it becomes a line of text. */
export interface RawEvent {
  name: string;
  /** Second topic (the bounty ID) for bounty-scoped events. */
  bountyId: number | null;
  /** Native value of the event payload. */
  data: unknown;
}

type Tuple = unknown[];

const asTuple = (v: unknown): Tuple => (Array.isArray(v) ? v : [v]);
const asNumber = (v: unknown): number => Number(v);
const asBigInt = (v: unknown): bigint => (typeof v === "bigint" ? v : BigInt(v as string | number));

/** Formats a token amount, adding the symbol when the token is known. */
function amountText(raw: unknown, token: string | undefined): string {
  const value = formatAmount(asBigInt(raw));
  return token ? `${value} ${tokenSymbol(token)}` : value;
}

/**
 * Turns one decoded contract event into a sentence. `token` is the bounty's
 * token when the caller knows it; amounts are shown without a symbol otherwise.
 */
export function describeEvent(event: RawEvent, token?: string): string {
  const { name, bountyId, data } = event;
  const bounty = bountyId !== null ? `bounty #${bountyId}` : "";
  const d = asTuple(data);

  switch (name) {
    case "created":
      // (creator, token, amount, deadline)
      return `Bounty #${bountyId} funded with ${amountText(d[2], asString(d[1]) ?? token)}`;
    case "pool_bty":
      // (admin, token, amount, deadline)
      return `Bounty #${bountyId} opened from the pool with ${amountText(d[2], asString(d[1]) ?? token)}`;
    case "funded":
      // (funder, amount)
      return `${shortAccount(asString(d[0]))} added ${amountText(d[1], token)} to ${bounty}`;
    case "extended":
      // (creator, new_deadline)
      return `Deadline extended on ${bounty}`;
    case "claimed":
      // (claim_id, claimant)
      return `Claim #${asNumber(d[0])} submitted on ${bounty}`;
    case "withdrawn":
      // (claim_id, claimant)
      return `Claim #${asNumber(d[0])} withdrawn on ${bounty}`;
    case "rejected":
      // (claim_id, caller)
      return `Claim #${asNumber(d[0])} rejected on ${bounty}`;
    case "released":
      // single release: (claim_id, claimant, payout, caller, drips_share)
      // split release:  (claim_id, claimant, payout, admin)
      return `Claim #${asNumber(d[0])} accepted on ${bounty} — ${amountText(d[2], token)}`;
    case "cancelled":
      // (caller, amount)
      return `Bounty #${bountyId} cancelled — ${amountText(d[1], token)} refunded`;
    case "pause":
      // (paused, admin)
      return d[0] ? "Contract paused" : "Contract resumed";
    case "admin_prp":
      return "Admin transfer proposed";
    case "admin_set":
      return "Admin changed";
    case "tkn_allow":
      return `Token allowed: ${shortAccount(asString(d[0]))}`;
    case "tkn_deny":
      return `Token disallowed: ${shortAccount(asString(d[0]))}`;
    case "min_set":
      return `Minimum amount set for ${shortAccount(asString(d[0]))}`;
    case "pool_set":
      return "Funding pool address updated";
    case "pool_dep":
      return `Deposit of ${amountText(d[2], asString(d[1]))} to the pool`;
    default:
      return `Contract event: ${name}`;
  }
}

function asString(v: unknown): string | undefined {
  return typeof v === "string" ? v : undefined;
}

function shortAccount(address: string | undefined): string {
  if (!address) return "An account";
  return address.length > 12 ? `${address.slice(0, 6)}…${address.slice(-4)}` : address;
}

/** Decodes a raw RPC event into the name, bounty ID and payload used above. */
export function decodeRawEvent(topic: rpc.Api.EventResponse["topic"], value: unknown): RawEvent {
  const name = String(scValToNative(topic[0]));
  const bountyId = topic[1] ? asNumber(scValToNative(topic[1])) : null;
  return { name, bountyId, data: value };
}

/**
 * The most recent contract events, newest first. Bounty tokens are looked up
 * so amounts can show their symbol.
 */
export async function fetchActivity(limit = 30): Promise<ActivityItem[]> {
  const latest = await server.getLatestLedger();
  const res = await server.getEvents({
    startLedger: Math.max(1, latest.sequence - LOOKBACK_LEDGERS),
    filters: [{ type: "contract", contractIds: [CONTRACT_ID] }],
    limit: EVENT_LIMIT,
  });

  const recent = res.events.slice(-limit).reverse();
  const decoded = recent.map((e) => ({
    event: e,
    raw: decodeRawEvent(e.topic, scValToNative(e.value)),
  }));

  const ids = [...new Set(decoded.map((d) => d.raw.bountyId).filter((id): id is number => id !== null))];
  const tokens = new Map<number, string>();
  await Promise.all(
    ids.map(async (id) => {
      try {
        const bounty = await getBounty(id);
        if (bounty) tokens.set(id, bounty.token);
      } catch {
        // Missing token just means the amount is shown without a symbol.
      }
    }),
  );

  return decoded.map(({ event, raw }) => ({
    id: event.id,
    txHash: event.txHash,
    at: event.ledgerClosedAt ?? null,
    text: describeEvent(raw, raw.bountyId !== null ? tokens.get(raw.bountyId) : undefined),
  }));
}
