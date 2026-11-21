/**
 * Freighter wallet access. Nothing is signed until a caller asks for it;
 * reading the dashboard never touches the wallet.
 */
import { getAddress, isConnected, requestAccess, signTransaction } from "@stellar/freighter-api";
import { NETWORK_PASSPHRASE } from "../config";

function errorText(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) return String((err as { message: unknown }).message);
  return String(err);
}

/** Address of the connected Freighter account, or `null` if none is connected. */
export async function currentAddress(): Promise<string | null> {
  const connected = await isConnected();
  if (connected.error || !connected.isConnected) return null;
  const res = await getAddress();
  return res.error || !res.address ? null : res.address;
}

/** Asks the user to connect Freighter and returns the chosen address. */
export async function connectWallet(): Promise<string> {
  const connected = await isConnected();
  if (connected.error || !connected.isConnected) {
    throw new Error("Freighter wallet not found. Install the Freighter extension and try again.");
  }
  const res = await requestAccess();
  if (res.error) throw new Error(errorText(res.error));
  if (!res.address) throw new Error("No account was shared by the wallet.");
  return res.address;
}

/** Signs a transaction envelope (XDR) for the configured network. */
export async function signXdr(xdr: string): Promise<string> {
  const res = await signTransaction(xdr, { networkPassphrase: NETWORK_PASSPHRASE });
  if (res.error) throw new Error(errorText(res.error));
  return res.signedTxXdr;
}
