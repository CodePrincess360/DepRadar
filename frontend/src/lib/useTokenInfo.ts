import { useEffect, useState } from "react";
import { NATIVE_XLM_SAC } from "../config";
import { getTokenInfo, type TokenInfo } from "./contract";

/**
 * Token symbol and decimals for `token`, or `null` while loading or when the
 * lookup fails. Native XLM is known without a call. Callers pass the result's
 * fields to `formatAmount` / `tokenSymbol`, which fall back to their defaults
 * on `undefined`.
 */
export function useTokenInfo(token: string): TokenInfo | null {
  const [loaded, setLoaded] = useState<{ token: string; info: TokenInfo } | null>(null);

  useEffect(() => {
    if (token === NATIVE_XLM_SAC) return;
    let cancelled = false;
    getTokenInfo(token).then(
      (info) => {
        if (!cancelled) setLoaded({ token, info });
      },
      () => {},
    );
    return () => {
      cancelled = true;
    };
  }, [token]);

  return loaded?.token === token ? loaded.info : null;
}
