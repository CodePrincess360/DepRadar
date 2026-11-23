# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

This changelog is generated with [git-cliff](https://github.com/orhun/git-cliff)
from [Conventional Commits](https://www.conventionalcommits.org/).

See the [RepoGuard roadmap](https://github.com/DepRadarOrg/DepRadar/blob/main/ROADMAP.md)
for the release plan behind these versions.

## [Unreleased]

### Added

- Changelog and tagged releases with git-cliff (#152)
- Release RepoGuard: tag a version, generate the changelog, and publish binaries (#317)

### Documentation

- Add `cliff.toml` configuration for conventional commits
- Add `CHANGELOG.md` generated from the full git history
- Add release workflow on `v*` tags that creates a GitHub Release with the changelog section
- Link the [RepoGuard roadmap](https://github.com/DepRadarOrg/DepRadar/blob/main/ROADMAP.md) from release notes

<!--
The sections below are populated by `git cliff -o CHANGELOG.md` once the
repository has tagged releases. Until the first `v*` tag exists, the
`Unreleased` section above is the canonical entry point.

To regenerate this file locally:

    git cliff -o CHANGELOG.md

To preview the changelog for an upcoming release:

    git cliff --unreleased --tag vX.Y.Z --prepend CHANGELOG.md
-->
