import { useEffect, useState, type ChangeEvent } from "react";
import { useTranslation, Trans } from "react-i18next";
import { Link, Outlet, useLocation } from "react-router-dom";
import {
  ACTIVE_NETWORK,
  CONTRACT_ID,
  NETWORK,
  availableNetworks,
  explorer,
  saveNetworkChoice,
  type NetworkPreset,
} from "../config";
import { shortAddress } from "../lib/format";
import { WalletButton } from "../wallet/WalletButton";
import { LanguageSelector } from "./LanguageSelector";

const NAV_LINKS: { key: string; to?: string; href?: string; labelKey: string; className?: string }[] = [
  { key: "new", to: "/new", labelKey: "nav.newBounty" },
  { key: "leaderboard", to: "/leaderboard", labelKey: "nav.leaderboard" },
  { key: "about", to: "/about", labelKey: "nav.about" },
  { key: "github", href: "https://github.com/DepRadarOrg/DepRadar", labelKey: "nav.github", className: "btn btn-ghost" },
];

/**
 * Shows the active network, with a picker when more than one is available.
 * Each network has its own RPC client and contract, so a switch saves the
 * choice and reloads the page.
 */
function NetworkSwitcher() {
  const networks = availableNetworks();
  const [saveFailed, setSaveFailed] = useState(false);

  if (networks.length < 2) {
    return (
      <span className="net">
        <span className="live-dot" aria-hidden />
        {NETWORK}
      </span>
    );
  }

  const onChange = (e: ChangeEvent<HTMLSelectElement>) => {
    const id = e.target.value as NetworkPreset["id"];
    if (id === ACTIVE_NETWORK.id) return;
    if (!saveNetworkChoice(id)) {
      // Without storage the choice wouldn't survive the reload, so keep the current network.
      setSaveFailed(true);
      return;
    }
    window.location.reload();
  };

  return (
    <label className="net">
      <span className="live-dot" aria-hidden />
      <select aria-label="Stellar network" value={ACTIVE_NETWORK.id} onChange={onChange} className="net-select">
        {networks.map((n) => (
          <option key={n.id} value={n.id}>
            {n.label}
          </option>
        ))}
      </select>
      {saveFailed && <span className="muted">Could not save network choice</span>}
    </label>
  );
}

export function Layout() {
  const { pathname } = useLocation();
  const { t } = useTranslation();
  useEffect(() => {
    window.scrollTo(0, 0);
  }, [pathname]);

  return (
    <div className="app">
      <div className="bg-grid" aria-hidden />
      <header className="topbar">
        <div className="topbar-inner">
          <Link to="/" className="brand">
            <span className="brand-mark" aria-hidden>
              <span className="brand-sweep" />
            </span>
            DepRadar
          </Link>
          <nav className="topbar-meta">
            {NAV_LINKS.map(({ key, to, href, labelKey, className = "btn btn-ghost" }) =>
              to ? (
                <Link key={key} to={to} className={className}>
                  {t(labelKey)}
                </Link>
              ) : (
                <a key={key} href={href} target="_blank" rel="noreferrer" className={className}>
                  {t(labelKey)}
                </a>
              )
            )}
            <WalletButton />
            <LanguageSelector />
            <NetworkSwitcher />
            <a href={explorer.contract(CONTRACT_ID)} target="_blank" rel="noreferrer" className="mono hide-sm">
              {shortAddress(CONTRACT_ID)}
            </a>
          </nav>
        </div>
      </header>
      <main className="content">
        <Outlet />
      </main>
      <footer className="footer">
        <span>
          {/* Each element in the components array passed to Trans must carry a
              key prop so React can reconcile the list without warnings. */}
          <Trans i18nKey="footer.readOnly" components={[<code key="code" />]} />
        </span>
        <span className="muted">{t("footer.built")}</span>
      </footer>
    </div>
  );
}
