# Contributing to DepRadar

Thanks for helping improve DepRadar! This guide covers the basics for getting a change merged.

## Before you start

- Read the [ROADMAP.md](ROADMAP.md) to see where the project is headed.
- For larger changes, open or comment on an issue first so we can align on scope.
- Keep pull requests focused: one issue per PR where possible.

## Development setup

- Install the Rust toolchain (stable) via [rustup](https://rustup.rs/).
- Build and test locally:

  ```sh
  cargo build
  cargo test
  ```

- Run the CLI against a sample project to sanity-check behavior.

## Adding a new detection rule (no Rust required)

New campaigns are contributed as **data files plus fixtures** — you do not need to
write or modify any Rust code to add a rule. The test harness picks up new rules
automatically.

### 1. Add the rule data file

Create a new file under `data/ioc/` named after the campaign, e.g.
`data/ioc/my-campaign.json`. Follow the rule format documented in
[`docs/rule-format.md`](docs/rule-format.md). At minimum a rule declares:

- `id` — a stable, unique, kebab-case identifier (e.g. `my-campaign`).
- `name` — a human-readable campaign name.
- `description` — what the rule detects and why it matters.
- `severity` — one of `low`, `medium`, `high`, `critical`.
- `indicators` — the list of indicators (package names, versions, hashes, etc.)
  that identify the campaign.

### 2. Add fixtures

Fixtures live next to the rule under `data/ioc/fixtures/<rule-id>/`. Copy the
template at [`data/ioc/fixtures/TEMPLATE/`](data/ioc/fixtures/TEMPLATE/) and
provide:

- `positive/` — inputs that **must** trigger the rule.
- `negative/` — inputs that **must not** trigger the rule.

Each fixture is a small manifest/lockfile snippet. Keep them minimal and
self-contained so they are easy to review.

### 3. Validate locally

The test harness validates every rule data file against its fixtures, so you can
check your contribution without touching Rust internals:

```sh
cargo test --test rule_harness
```

This asserts that:

- every rule file parses and matches the documented schema,
- every rule has a matching fixture directory,
- all `positive/` fixtures trigger the rule, and
- all `negative/` fixtures do not.

If the harness reports a missing fixture or a schema error, fix the data file or
add the missing fixture — no code changes are needed.

## Pull request checklist

- [ ] The change is scoped to a single issue.
- [ ] `cargo test` passes locally.
- [ ] New rules include a data file under `data/ioc/` and fixtures under
      `data/ioc/fixtures/<rule-id>/`.
- [ ] Every command and link in the change was checked against the repo.
- [ ] `PROGRESS.md` has an entry for the work.
- [ ] The PR description includes `Closes #<issue-number>`.

## Commit and PR style

- Use clear, imperative commit messages (e.g. `fix: validate rule fixtures`).
- Describe the motivation and the approach in the PR description.
- Reference the issue being closed with `Closes #<number>`.

## Questions

Open an issue or comment on the relevant roadmap item — we are happy to help.
