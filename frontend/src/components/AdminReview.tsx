import { useState } from "react";
import { explorer } from "../config";
import { getAdmin, rejectClaim, releaseClaim } from "../lib/contract";
import { formatAmount, shortAddress, tokenSymbol } from "../lib/format";
import type { Bounty, Claim } from "../lib/types";
import { useAsync } from "../lib/useAsync";
import { useWallet } from "../lib/useWallet";
import { AddressLink } from "./ui";

interface Props {
  bounty: Bounty;
  claims: Claim[];
  /** Called with the transaction hash after a successful action. */
  onAction: (txHash: string) => void;
}

/**
 * Reject / Release controls for pending claims. Shown only when the connected
 * wallet is the contract admin; everyone else sees nothing here.
 */
export function AdminReview({ bounty, claims, onAction }: Props) {
  const wallet = useWallet();
  const admin = useAsync(getAdmin, "admin");
  const pending = claims.filter((c) => c.status === "Pending");

  if (pending.length === 0) return null;

  if (!wallet.address) {
    return (
      <div className="admin-review">
        <button type="button" className="btn btn-ghost" onClick={wallet.connect}>
          Connect wallet to review claims
        </button>
        {wallet.error && <p className="error">{wallet.error.message}</p>}
      </div>
    );
  }

  if (admin.status !== "success" || admin.data !== wallet.address) return null;
  const adminAddress = wallet.address;

  return (
    <div className="admin-review">
      <h3>Review pending claims</h3>
      {pending.map((claim) => (
        <ClaimReviewRow key={claim.id} bounty={bounty} claim={claim} admin={adminAddress} onAction={onAction} />
      ))}
    </div>
  );
}

function ClaimReviewRow({
  bounty,
  claim,
  admin,
  onAction,
}: {
  bounty: Bounty;
  claim: Claim;
  admin: string;
  onAction: (txHash: string) => void;
}) {
  const [busy, setBusy] = useState<"reject" | "release" | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function run(kind: "reject" | "release", send: () => Promise<string>) {
    setBusy(kind);
    setError(null);
    try {
      onAction(await send());
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(null);
      setConfirming(false);
    }
  }

  return (
    <div className="claim-review">
      <div>
        Claim #{claim.id} by <AddressLink address={claim.claimant} />
      </div>
      {confirming ? (
        <div className="confirm-release">
          <p>
            Release <strong>{formatAmount(bounty.amount)} {tokenSymbol(bounty.token)}</strong> to{" "}
            <code title={claim.claimant}>{shortAddress(claim.claimant)}</code>? This sends funds and closes the bounty.
          </p>
          <button
            type="button"
            className="btn"
            disabled={busy !== null}
            onClick={() => run("release", () => releaseClaim(admin, bounty.id, claim.id))}
          >
            {busy === "release" ? "Releasing…" : "Confirm release"}
          </button>{" "}
          <button type="button" className="btn btn-ghost" disabled={busy !== null} onClick={() => setConfirming(false)}>
            Cancel
          </button>
        </div>
      ) : (
        <div className="claim-review-actions">
          <button
            type="button"
            className="btn btn-ghost"
            disabled={busy !== null}
            onClick={() => run("reject", () => rejectClaim(admin, bounty.id, claim.id))}
          >
            {busy === "reject" ? "Rejecting…" : "Reject"}
          </button>{" "}
          <button type="button" className="btn" disabled={busy !== null} onClick={() => setConfirming(true)}>
            Release
          </button>
        </div>
      )}
      {error && <p className="error">{error}</p>}
    </div>
  );
}

/** Link to the transaction explorer for a hash returned by an action. */
export function TxLink({ hash }: { hash: string }) {
  return (
    <a href={explorer.tx(hash)} target="_blank" rel="noreferrer">
      View transaction
    </a>
  );
}
