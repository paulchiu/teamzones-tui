# Changelog

This file documents notable changes to the project.

The format is based on [Keep a Changelog v1.1.0](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- The `teamzones-core` library: it reads and checks `config.toml` (naming the line of any problem), resolves cities from a bundled GeoNames list, understands typed times such as `11-1pm`, `mon 3pm sast` and `next`, finds the slots where a group's question fits, and writes the say line and status lines.

## [0.1.0] - 2026-10-24

### Added

- The `teamzones` binary, which so far prints its version with `teamzones --version`.
