import type { Bounty } from "./types";

/** Seconds after the deadline during which the admin can still review. Matches `REVIEW_WINDOW_SECS` in the contract. */
export const REVIEW_WINDOW_SECS = 7 * 24 * 60 * 60;

export interface CancelWindow {
  /** Whether the creator can cancel right now. */
  eligible: boolean;
  /** Unix seconds when cancellation opens, when it isn't open yet. `null` if it can't open at all. */
  availableAt: number | null;
  /** Why cancellation is unavailable, when it is. */
  blocked: "closed" | "paused" | null;
}

/**
 * Mirrors the creator branch of `cancel_bounty` in the contract:
 *
 *   now >= deadline && pending_claims == 0      (no claims to review)
 *   now >= deadline + REVIEW_WINDOW_SECS        (review window has passed)
 *
 * Only Open bounties can be cancelled, and non-admins are refused while the
 * contract is paused.
 */
export function cancelWindow(
  bounty: Pick<Bounty, "status" | "deadline" | "pendingClaims">,
  now: number,
  paused = false,
): CancelWindow {
  if (bounty.status !== "Open") return { eligible: false, availableAt: null, blocked: "closed" };
  if (paused) return { eligible: false, availableAt: null, blocked: "paused" };

  const reviewEnds = bounty.deadline + REVIEW_WINDOW_SECS;
  const deadlinePassed = now >= bounty.deadline;
  const reviewOver = now >= reviewEnds;
  if (reviewOver || (deadlinePassed && bounty.pendingClaims === 0)) {
    return { eligible: true, availableAt: null, blocked: null };
  }

  // Not yet eligible. With no pending claims, the deadline is the only wait;
  // otherwise the review window has to pass first.
  const availableAt = bounty.pendingClaims === 0 ? bounty.deadline : reviewEnds;
  return { eligible: false, availableAt, blocked: null };
}
