/**
 * Access to the deprader_router contract. Reads are simulated `queryContract`
 * calls that need no wallet. The admin and creator actions at the bottom are
 * the only calls that sign and submit, and they run only on user request.
 */
import { Address, BASE_FEE, Contract, TransactionBuilder, nativeToScVal, rpc, xdr } from "@stellar/stellar-sdk";
import { parseBounty, parseClaim } from "@deprader/contract-types";
import { CONTRACT_ID, NETWORK, NETWORK_PASSPHRASE, RPC_URL } from "../config";
import { signXdr } from "./wallet";
import type { Bounty, Claim } from "./types";

export { parseBounty, parseClaim };

/** Contract address and network, exposed for display (e.g. the About page). */
export { CONTRACT_ID, NETWORK };

const server = new rpc.Server(RPC_URL);

async function query(method: string, args: Record<string, unknown> = {}): Promise<unknown> {
  const { result } = await server.queryContract(CONTRACT_ID, method, args);
  return result;
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

/** Whether the contract is paused. `cancel_bounty` refuses non-admin callers while it is. */
export async function isPaused(): Promise<boolean> {
  return Boolean(await query("is_paused"));
}

/**
 * Submits a state-changing contract call signed by the connected wallet and
 * waits for it to land. Resolves with the transaction hash; rejects with a
 * readable message if simulation, signing, submission or execution fails.
 */
export async function invokeSigned(source: string, method: string, args: xdr.ScVal[]): Promise<string> {
  const account = await server.getAccount(source);
  const tx = new TransactionBuilder(account, { fee: BASE_FEE, networkPassphrase: NETWORK_PASSPHRASE })
    .addOperation(new Contract(CONTRACT_ID).call(method, ...args))
    .setTimeout(60)
    .build();

  const prepared = await server.prepareTransaction(tx);
  const signed = TransactionBuilder.fromXDR(await signXdr(prepared.toXDR()), NETWORK_PASSPHRASE);

  const sent = await server.sendTransaction(signed);
  if (sent.status === "ERROR") throw new Error(`Transaction rejected by the network (${sent.status}).`);

  const result = await server.pollTransaction(sent.hash, { attempts: 30 });
  if (result.status !== "SUCCESS") throw new Error(`Transaction failed on-chain (${result.status}).`);
  return sent.hash;
}

/** Wallet-signed call to `release(caller, bounty_id, claim_id)`. */
export function releaseClaim(admin: string, bountyId: number, claimId: number): Promise<string> {
  return invokeSigned(admin, "release", [
    new Address(admin).toScVal(),
    nativeToScVal(bountyId, { type: "u64" }),
    nativeToScVal(claimId, { type: "u32" }),
  ]);
}

/** Wallet-signed call to `reject_claim(caller, bounty_id, claim_id)`. */
export function rejectClaim(admin: string, bountyId: number, claimId: number): Promise<string> {
  return invokeSigned(admin, "reject_claim", [
    new Address(admin).toScVal(),
    nativeToScVal(bountyId, { type: "u64" }),
    nativeToScVal(claimId, { type: "u32" }),
  ]);
}

/** Wallet-signed call to `cancel_bounty(caller, bounty_id)`. */
export function cancelBounty(caller: string, bountyId: number): Promise<string> {
  return invokeSigned(caller, "cancel_bounty", [
    new Address(caller).toScVal(),
    nativeToScVal(bountyId, { type: "u64" }),
  ]);
}
