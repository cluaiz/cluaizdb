# Contributing to Cluaizdb

First off, thank you for considering contributing to Cluaizdb! It's people like you that make this database engine a powerful reality.

## 🛠️ Development Setup

Cluaizdb is built in Rust. You will need the latest stable Rust toolchain.

1. **Clone the repo:**
   ```bash
   git clone https://github.com/cluaiz/cluaizdb.git
   cd cluaizdb
   ```

2. **Build the Server:**
   ```bash
   cargo build
   ```

3. **Run the Test Suite:**
   Ensure all unit tests and integration tests pass before submitting a PR.
   ```bash
   cargo test --all
   ```

## 📝 Pull Request Process

1. Fork the repository and create your branch from `main`.
2. If you've added code that should be tested, add tests.
3. If you've changed APIs, update the documentation.
4. Ensure the test suite passes (`cargo test`).
5. Format your code using `cargo fmt`.
6. Make sure your code lints via `cargo clippy`.
7. Issue that pull request!

## 📜 Code of Conduct

Please note that this project is released with a Contributor Code of Conduct. By participating in this project you agree to abide by its terms.
