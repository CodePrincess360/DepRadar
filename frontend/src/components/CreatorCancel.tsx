import { useState, type ReactNode } from "react";
import { cancelBounty, isPaused } from "../lib/contract";
import { formatAmount, formatDate, tokenSymbol } from "../lib/format";
import { cancelWindow, type CancelWindow } from "../lib/cancel";
import type { Bounty } from "../lib/types";
import { useAsync } from "../lib/useAsync";
import { useWallet } from "../lib/useWallet";

/**
 * Cancel & refund for the bounty's creator. Shown only to the connected
 * creator wallet; the button appears once `cancel_bounty` would accept it.
 */
export function CreatorCancel({ bounty, onAction }: { bounty: Bounty; onAction: (txHash: string) => void }) {
  const wallet = useWallet();
  const paused = useAsync(isPaused, "paused");

  if (bounty.status !== "Open") return null;

  if (!wallet.address) {
    return (
      <div className="creator-cancel">
        <button type="button" className="btn btn-ghost" onClick={wallet.connect}>
          Connect wallet to manage this bounty
        </button>
        {wallet.error && <p className="error">{wallet.error.message}</p>}
      </div>
    );
  }

  if (wallet.address !== bounty.creator) return null;
  if (paused.status === "loading") return null;

  const cw = cancelWindow(bounty, Date.now() / 1000, paused.status === "success" && paused.data);
  return <CreatorCancelAction bounty={bounty} creator={wallet.address} cw={cw} onAction={onAction} />;
}

function CreatorCancelAction({
  bounty,
  creator,
  cw,
  onAction,
}: {
  bounty: Bounty;
  creator: string;
  cw: CancelWindow;
  onAction: (txHash: string) => void;
}) {
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function cancel() {
    setBusy(true);
    setError(null);
    try {
      onAction(await cancelBounty(creator, bounty.id));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
      setConfirming(false);
    }
  }

  let body: ReactNode;
  if (cw.blocked === "paused") {
    body = <p className="muted">Cancellation is unavailable while the contract is paused.</p>;
  } else if (cw.eligible) {
    body = confirming ? (
      <div className="confirm-release">
        <p>
          Cancel this bounty and refund <strong>{formatAmount(bounty.amount)} {tokenSymbol(bounty.token)}</strong> to
          its funders?
        </p>
        <button type="button" className="btn" disabled={busy} onClick={cancel}>
          {busy ? "Cancelling…" : "Confirm cancel & refund"}
        </button>{" "}
        <button type="button" className="btn btn-ghost" disabled={busy} onClick={() => setConfirming(false)}>
          Back
        </button>
      </div>
    ) : (
      <button type="button" className="btn" onClick={() => setConfirming(true)}>
        Cancel &amp; refund
      </button>
    );
  } else if (cw.availableAt !== null) {
    body = (
      <p className="muted">
        Cancel &amp; refund becomes available on {formatDate(cw.availableAt)}.
      </p>
    );
  } else {
    body = null;
  }

  return (
    <div className="creator-cancel">
      {body}
      {error && <p className="error">{error}</p>}
    </div>
  );
}
