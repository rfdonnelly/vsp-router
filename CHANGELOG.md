# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

No unrelease changes.

## [0.3.0] - 2023-08-01

### Added

- Added Windows support w/o ability to create virtual serial ports
- Added Windows binaries

### Fixed

- Fixed typo in `--help`

## [0.2.1] - 2023-08-01

### Added

- Release automation and release binaries for Linux and macOS

## [0.2.0] - 2022-10-29

### Changed

- Upgraded from clap v3 to clap v4
- Replaced `tokio::select!` + `tokio_util::sync::CancellationToken` with `futures_util::future::Abortable`

## [0.1.1] - 2022-09-11

Minor fixes for publishing on Crates.io.

## [0.1.0] - 2022-09-10

Initial release.

[unreleased]: https://github.com/rfdonnelly/vsp-router/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/rfdonnelly/vsp-router/releases/tag/v0.3.0
[0.2.1]: https://github.com/rfdonnelly/vsp-router/releases/tag/v0.2.1
[0.2.0]: https://github.com/rfdonnelly/vsp-router/releases/tag/v0.2.0
[0.1.1]: https://github.com/rfdonnelly/vsp-router/releases/tag/v0.1.1
[0.1.0]: https://github.com/rfdonnelly/vsp-router/releases/tag/v0.1.0
