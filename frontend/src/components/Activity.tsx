import { useEffect, useState } from "react";
import { explorer } from "../config";
import { type ActivityItem, fetchActivity } from "../lib/events";
import { relativeTime } from "../lib/format";
import { Empty, ErrorState, Skeleton } from "./ui";

const POLL_MS = 15_000;

/** Recent contract activity, refreshed every 15 seconds. */
export function Activity() {
  const [items, setItems] = useState<ActivityItem[] | null>(null);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        const next = await fetchActivity();
        if (!cancelled) {
          setItems(next);
          setError(null);
        }
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err : new Error(String(err)));
      }
    }
    load();
    const timer = setInterval(load, POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, []);

  return (
    <section className="activity" aria-labelledby="activity-title">
      <h2 id="activity-title">Activity</h2>
      {error && !items && <ErrorState error={error} />}
      {!error && !items && (
        <div aria-hidden>
          <Skeleton className="sk-block" />
        </div>
      )}
      {items && items.length === 0 && <Empty>No contract activity in the last day.</Empty>}
      {items && items.length > 0 && (
        <ol className="activity-list">
          {items.map((item) => (
            <li key={item.id}>
              <a href={explorer.tx(item.txHash)} target="_blank" rel="noreferrer">
                {item.text}
              </a>
              {item.at && <span className="muted"> · {relativeTime(Date.parse(item.at) / 1000)}</span>}
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
