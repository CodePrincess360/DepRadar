#!/usr/bin/env bash
set -euo pipefail
root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
git init -q "$root/repository"
cp tests/fixtures/repo-scan/positive/postcss.config.js "$root/repository/"
git -C "$root/repository" -c user.name=Fixture -c user.email=fixture@example.invalid add .
git -C "$root/repository" -c user.name=Fixture -c user.email=fixture@example.invalid commit -qm fixture
printf '%s\n' fixture/repository >"$root/repos.txt"
age-keygen -o "$root/identity" 2>/dev/null
export SWEEP_AGE_RECIPIENT
SWEEP_AGE_RECIPIENT=$(age-keygen -y "$root/identity")
export GIT_CONFIG_COUNT=1
export GIT_CONFIG_KEY_0="url.file://$root/repository.insteadOf"
export GIT_CONFIG_VALUE_0=https://github.com/fixture/repository.git
bash scripts/nightly-repo-sweep.sh "$PWD/target/debug/depradar" "$root/repos.txt" "$root/report.tar.age"
age -d -i "$root/identity" "$root/report.tar.age" | tar -xOf - ./fixture_repository.json | python3 -c '
import json,sys
report=json.load(sys.stdin)
assert report["findings"][0]["rule_id"] == "repo-scan/padded-build-config",report
print("Encrypted sweep report decrypted and verified")
'
