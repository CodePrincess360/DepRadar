import { useCallback, useEffect, useState } from "react";
import { connectWallet, currentAddress } from "./wallet";

/** The connected Freighter address (or `null`), plus a connect action. */
export function useWallet() {
  const [address, setAddress] = useState<string | null>(null);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let cancelled = false;
    currentAddress()
      .then((a) => !cancelled && setAddress(a))
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  const connect = useCallback(async () => {
    setError(null);
    try {
      setAddress(await connectWallet());
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    }
  }, []);

  return { address, connect, error };
}
