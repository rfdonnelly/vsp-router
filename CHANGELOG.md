# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

No unrelease changes.

## Version 1.0.2 (2024-08-15)

[GitHub Release page](https://github.com/rfdonnelly/vsp-router/releases/tag/v1.0.1)

### Fixed

- Fixed blocking on full virtual serial port (PTY)

  Previous to this fix, vsp-router would block if a virtual serial port buffer became full.
  Now, vsp-router drops the data.
  See [#16](https://github.com/rfdonnelly/vsp-router/pull/16) for more details.

## Version 1.0.1 (2023-08-01)

[GitHub Release page](https://github.com/rfdonnelly/vsp-router/releases/tag/v1.0.1)

### Fixed

- Fixed typo in README

## Version 1.0.0 (2023-08-01)

[GitHub Release page](https://github.com/rfdonnelly/vsp-router/releases/tag/v1.0.0)

### Changed

- Renamed the `--virtual` option to `--create`
- Renamed the `--physical` option to `--attach`

## Version 0.3.0 (2023-08-01)

[GitHub Release page](https://github.com/rfdonnelly/vsp-router/releases/tag/v0.3.0)

### Added

- Added Windows support w/o ability to create virtual serial ports
- Added Windows binaries

### Fixed

- Fixed typo in `--help`

## Version 0.2.1 (2023-08-01)

[GitHub Release page](https://github.com/rfdonnelly/vsp-router/releases/tag/v0.2.1)

### Added

- Release automation and release binaries for Linux and macOS

## Version 0.2.0 (2022-10-29)

[GitHub Release page](https://github.com/rfdonnelly/vsp-router/releases/tag/v0.2.0)

### Changed

- Upgraded from clap v3 to clap v4
- Replaced `tokio::select!` + `tokio_util::sync::CancellationToken` with `futures_util::future::Abortable`

## Version 0.1.1 (2022-09-11)

[GitHub Release page](https://github.com/rfdonnelly/vsp-router/releases/tag/v0.1.1)

Minor fixes for publishing on Crates.io.

## Version 0.1.0 (2022-09-10)

[GitHub Release page](https://github.com/rfdonnelly/vsp-router/releases/tag/v0.1.0)

Initial release.
