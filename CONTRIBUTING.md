# Contributing to Housaky AGI

Thank you for your interest in contributing to Housaky AGI!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/housaky`
3. Create a branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Run tests: `cargo test --release`
6. Verify: `./verify.sh`
7. Commit: `git commit -m "Add your feature"`
8. Push: `git push origin feature/your-feature`
9. Open a Pull Request

## Development Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone repository
git clone https://github.com/housaky/housaky
cd housaky

# Build
cargo build --release

# Test
cargo test --release

# Verify
./verify.sh
```

## Code Standards

- Follow Rust conventions
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add tests for new features
- Document public APIs
- Keep commits atomic and descriptive

## Testing

```bash
# Unit tests
cargo test

# Integration tests
cargo test --release

# Comprehensive suite
./test_and_improve.sh
```

## Pull Request Process

1. Update README.md if needed
2. Add tests for new functionality
3. Ensure all tests pass
4. Update documentation
5. Request review from maintainers

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow
- Follow the Apache 2.0 license terms

## Questions?

Open an issue or join our Discord: https://discord.gg/housaky
