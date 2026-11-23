# Repository scanning and coordinated disclosure

Scan a local checkout. The CLI writes JSON containing file paths, line
numbers, and rule IDs. It skips ignored files, symlinks, generated
directories, and files larger than 2 MiB. The report counts oversized files.
A successful scan exits zero even when it detects indicators. Investigate
matches before treating them as confirmed findings.

```sh
umask 077
depradar repo-scan . > private-report.json
```

## Nightly sweep

The [nightly workflow](../../.github/workflows/repo-sweep.yml) processes
the [checked-in Wave repository list](../../.github/drips-wave-repos.txt)
sequentially, with a pause between attempts. It clones public repositories
without credentials or submodules and never executes repository code.
Configure the repository Actions variable `SWEEP_AGE_RECIPIENT` with the
disclosure team's age public recipient. Only encrypted results are uploaded.
Keep the age identity outside the repository and decrypt in a private
workspace.

## Private notification and publication

Confirm the finding, identify the affected project's private security contact
from its security policy, and send them the report through that channel.
For DepRadar itself, follow [DepRadar's security policy](../../SECURITY.md).
Agree on a public release date with maintainers and record it only after they
receive the report. Treat the report and receipt as private until that date.
The CLI records your notification, requires at least seven days of embargo,
and binds the receipt to the exact report bytes.

```sh
# Set EMBARGO_UNTIL to the agreed Unix timestamp before running this.
depradar repo-disclose private-report.json receipt.json --embargo-until "$EMBARGO_UNTIL"
depradar repo-publish private-report.json receipt.json > published-report.json
```

The publish command returns an error and emits no report during the embargo
or if the receipt belongs to different bytes. The receipt is a local
operator record. Restrict write access to it and keep the machine clock
accurate.

## Badge

```sh
depradar repo-badge published-report.json --public --receipt receipt.json > repo-scan.svg
```

A current clean scan produces `clean`. A current released report with
matches produces `findings`. A missing or future-dated report, an otherwise
clean report that skipped oversized files, or a scan at least 14 days old
produces `stale`. For a clean report, omit `--receipt`. Generate an SVG
without `--public` for private use.

The [Pages workflow](../../.github/workflows/pages.yml) serves
`repo-scan.svg` alongside the dashboard and recomputes freshness daily.
To update it, place the latest cleared JSON and, for a report with
findings, its receipt next to this guide as `published-report.json` and
`published-receipt.json`. Keep private reports in the disclosure channel.

## Testnet bounty

Use a router deployed from this version of the contract and an identity
funded in an allowed testnet token. For setup, see the
[Stellar tutorial](../tutorial.md). The selected finding index is zero based.
The CLI invokes the proof-bearing creation entry point only after the
report's embargo expires. The router retains the SHA-256 hash for
`bounty_proof` lookups.

```sh
depradar repo-bounty private-report.json --finding 0 --receipt receipt.json \
  --issue-ref "$PUBLIC_ADVISORY" --router "$ROUTER" --source "$IDENTITY" \
  --token "$TOKEN" --amount "$STROOPS" --deadline "$DEADLINE"
```

## Signed community indicators

Host `rules.json` and `rules.json.sig` at direct HTTPS URLs in a separate
rules repository. The JSON is an array of objects with unique `id` values
beginning with `ioc/` and literal single-line UTF-8 `marker` values. The
signature file contains a base64-encoded Ed25519 signature of the exact JSON
bytes. Obtain the base64-encoded raw Ed25519 public key through a trusted
channel before configuring the scanner.

```sh
depradar repo-scan . --update-rules --rules-url "$RULES_URL" \
  --rules-public-key "$PUBLIC_KEY" > private-report.json
```

An invalid signature, response, or rule file aborts the scan. Signed rules
are loaded for that invocation and supplement the built-in checks.
