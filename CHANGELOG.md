# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Multi-model architecture with 10 database paradigms via WASM DNA.
- CDQL (Cluaiz Database Query Language) parser and execution planner.
- LMDB-based 3-Tier Storage architecture (Hot, Warm, Cold).
- C-FFI direct bindings for 0ms latency in Python/C++ integration.
- Dynamic Garbage Collector for automatic payload stripping.

### Changed
- Standardized all terminology to strict `cdql` and `cluaizdb` naming conventions.
- Refactored AST to `CdqlQuery` and unified the namespace.

### Fixed
- Replaced marketing fluff with strict, industrial-grade technical documentation.
