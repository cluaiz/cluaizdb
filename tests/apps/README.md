# 🖥️ Application & CLI Tools Tests

## 🎯 Purpose
This workspace tests the external applications built on top of Cluaizdb, such as the command-line interface (CLI) and graphical dashboard (GUI).

## 🧬 What we test here
1. **CLI Commands:** Does `cluaizdb-cli stats` correctly output the database size without crashing?
2. **Configuration Parsing:** Can the apps correctly parse `.toml` configuration files?

## 🚀 How to Run
```bash
cargo test --package apps-tests
```
