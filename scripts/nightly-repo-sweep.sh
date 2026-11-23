#!/usr/bin/env bash
set -euo pipefail

scanner=${1:?path to depradar}
repos=${2:?repository list}
output=${3:?encrypted output file}
: "${SWEEP_AGE_RECIPIENT:?configure an age recipient for the disclosure team}"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/reports"
failed=0

while IFS= read -r repo || [[ -n "$repo" ]]; do
  [[ -z "$repo" || "$repo" == \#* ]] && continue
  [[ "$repo" =~ ^[a-zA-Z0-9_-]+/[a-zA-Z0-9_.-]+$ ]] || exit 2
  report="$tmp/reports/${repo//\//_}.json"
  if timeout 180 git -c core.hooksPath=/dev/null clone --quiet --depth 1 \
      "https://github.com/$repo.git" "$tmp/checkout" 2>"$tmp/git-error" &&
      timeout 180 "$scanner" repo-scan "$tmp/checkout" >"$report" 2>"$tmp/scan-error"; then
    :
  else
    rm -f "$report"
    printf '%s\n' '{"error":"repository clone or scan failed"}' >"$report"
    failed=1
  fi
  rm -rf "$tmp/checkout"
  sleep 2
done <"$repos"

tar -C "$tmp/reports" -cf - . | age -r "$SWEEP_AGE_RECIPIENT" -o "$output"
exit "$failed"
