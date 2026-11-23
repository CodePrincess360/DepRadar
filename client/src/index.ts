/**
 * Minimal read-only client for the deprader_router Soroban contract on
 * Stellar testnet. Fetches a bounty and every claim submitted against it
 * and prints the result — no submitted transactions, no UI.
 *
 * Usage:
 *   npm start -- --bounty-id 1
 *   npm start -- --bounty-id 1 --contract C... --rpc https://...
 *
 * --bounty-id defaults to 1, the bounty used in the on-chain
 * create_bounty -> submit_claim -> reject_claim -> release cycle described
 * in PROGRESS.md, so running with no flags exercises real data.
 *
 * --contract / --rpc override the target deployment. When omitted, the
 * DEPRADAR_CONTRACT_ID / DEPRADAR_RPC_URL env vars are used, falling back
 * to the default testnet deployment below.
 *
 * --all fetches every bounty (using bounty_count) and its claims instead
 * of a single bounty. --json prints a single JSON document instead of the
 * human-readable output; combined with --all it includes every bounty.
 */
import { rpc } from "@stellar/stellar-sdk";

export const DEFAULT_CONTRACT_ID = "CCPRQANY467KKE4ROKCNW6TH3LY3EPVFRXIXJ5V4VCYHLJKWZPOZYQJQ";
export const DEFAULT_RPC_URL = "https://soroban-testnet.stellar.org";

const BOOLEAN_FLAGS = new Set(["all", "json", "proof"]);

export interface ParsedArgs {
  bountyId: number;
  contractId: string;
  rpcUrl: string;
  all: boolean;
  json: boolean;
  proof: boolean;
}

export type Env = Record<string, string | undefined>;

export function parseArgs(argv: string[], env: Env = process.env): ParsedArgs {
  const flags = new Map<string, string>();
  const booleans = new Set<string>();
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg.startsWith("--")) {
      const name = arg.slice(2);
      if (BOOLEAN_FLAGS.has(name)) {
        booleans.add(name);
      } else {
        flags.set(name, argv[i + 1]);
        i += 1;
      }
    }
  }

  const rawBountyId = flags.get("bounty-id") ?? "1";
  const bountyId = Number(rawBountyId);
  if (!Number.isInteger(bountyId) || bountyId < 0) {
    throw new Error(`--bounty-id must be a non-negative integer, got: ${rawBountyId}`);
  }

  const contractId = flags.get("contract") ?? env.DEPRADAR_CONTRACT_ID ?? DEFAULT_CONTRACT_ID;
  const rpcUrl = flags.get("rpc") ?? env.DEPRADAR_RPC_URL ?? DEFAULT_RPC_URL;

  return { bountyId, contractId, rpcUrl, all: booleans.has("all"), json: booleans.has("json"), proof: booleans.has("proof") };
}

export function bigIntReplacer(_key: string, value: unknown) {
  return typeof value === "bigint" ? value.toString() : value;
}

export interface FormatterInput {
  bountyId: number;
  contractId: string;
  rpcUrl: string;
  health: { status: string; latestLedger: number };
  bountyCount: unknown;
  bounty: unknown;
  claims: unknown[];
}

/** Renders the full read-client report as a list of output lines. */
export function formatOutput(input: FormatterInput): string[] {
  const { bountyId, contractId, rpcUrl, health, bountyCount, bounty, claims } = input;
  const lines: string[] = [];

  lines.push(`Connected to ${rpcUrl} (status: ${health.status}, latest ledger: ${health.latestLedger})`);
  lines.push(`Contract: ${contractId}`);
  lines.push(`Bounties created: ${bountyCount}\n`);

  lines.push(`get_bounty(${bountyId}) =>`);
  if (bounty === null) {
    lines.push("  (no bounty with this ID)");
    return lines;
  }
  lines.push(JSON.stringify(bounty, bigIntReplacer, 2));

  claims.forEach((claim, claimId) => {
    lines.push(`\nget_claim(${bountyId}, ${claimId}) =>`);
    lines.push(claim === null ? "  (no claim with this ID)" : JSON.stringify(claim, bigIntReplacer, 2));
  });

  return lines;
}

interface BountyResult {
  bounty_id: number;
  bounty: unknown;
  claims: unknown[];
  proof_ref?: string | null;
}

async function fetchBounty(server: rpc.Server, contractId: string, bountyId: number, proof: boolean): Promise<BountyResult> {
  const { result: bounty } = await server.queryContract(contractId, "get_bounty", { bounty_id: bountyId });
  const claims: unknown[] = [];
  if (bounty !== null) {
    const claimCount = Number((bounty as { claim_count: number }).claim_count);
    for (let claimId = 0; claimId < claimCount; claimId += 1) {
      const { result: claim } = await server.queryContract(contractId, "get_claim", {
        bounty_id: bountyId,
        claim_id: claimId,
      });
      claims.push(claim);
    }
  }
  const result: BountyResult = { bounty_id: bountyId, bounty, claims };
  if (proof && bounty !== null) {
    const response = await server.queryContract(contractId, "bounty_proof", { bounty_id: bountyId });
    result.proof_ref = proofHex(response.result);
  }
  return result;
}

export function proofHex(value: unknown): string | null {
  if (value === null || value === undefined) return null;
  if (!(value instanceof Uint8Array) || value.length !== 32) {
    throw new Error("Invalid bounty proof digest");
  }
  return Buffer.from(value).toString("hex");
}

function printBounty(result: BountyResult) {
  const { bounty_id: bountyId, bounty: rawBounty, claims: rawClaims } = result;
  console.log(`get_bounty(${bountyId}) =>`);
  if (rawBounty === null) {
    console.log("  (no bounty with this ID)");
    return;
  }
  console.log(JSON.stringify(rawBounty, bigIntReplacer, 2));
  if (result.proof_ref !== undefined) {
    console.log(`bounty_proof(${bountyId}) => ${result.proof_ref ?? "(no proof)"}`);
  }

  rawClaims.forEach((rawClaim, claimId) => {
    console.log(`\nget_claim(${bountyId}, ${claimId}) =>`);
    if (rawClaim === null) {
      console.log("  (no claim with this ID)");
      return;
    }
    console.log(JSON.stringify(rawClaim, bigIntReplacer, 2));
  });
}

async function main() {
  const { bountyId, contractId, rpcUrl, all, json, proof } = parseArgs(process.argv.slice(2));
  const server = new rpc.Server(rpcUrl);

  const health = await server.getHealth();
  const { result: count } = await server.queryContract(contractId, "bounty_count", {});

  if (!json) {
    console.log(`Connected to ${rpcUrl} (status: ${health.status}, latest ledger: ${health.latestLedger})`);
    console.log(`Contract: ${contractId}`);
    console.log(`Bounties created: ${count}\n`);
  }

  const bountyIds = all
    ? Array.from({ length: Number(count) }, (_unused, index) => index + 1)
    : [bountyId];

  const results: BountyResult[] = [];
  for (const id of bountyIds) {
    results.push(await fetchBounty(server, contractId, id, proof));
  }

  if (json) {
    const document = all
      ? { contract_id: contractId, bounty_count: count, bounties: results }
      : { contract_id: contractId, bounty_count: count, bounty: results[0] };
    console.log(JSON.stringify(document, bigIntReplacer, 2));
    return;
  }

  for (const result of results) {
    printBounty(result);
  }
}

if (require.main === module) {
  main().catch((err) => {
    console.error("Read failed:", err instanceof Error ? err.message : err);
    process.exitCode = 1;
  });
}
