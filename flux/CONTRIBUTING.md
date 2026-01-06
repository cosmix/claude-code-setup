# Contributing to Flux

## Development Setup

```bash
cd flux
cargo build
cargo test
```

## Quality Gates

All changes must pass these checks before merge:

```bash
cargo check                                    # Compilation
cargo test                                     # All tests pass
cargo clippy --all-targets -- -D warnings      # No lint warnings
cargo audit                                    # No security vulnerabilities
```

## Code Standards

- **File size limit:** 400 lines max
- **Function size limit:** 50 lines max
- **No `unwrap()` in production code** - use proper error handling with `anyhow`

## Release Process

### Binary Signing with Minisign

All release binaries are cryptographically signed using [minisign](https://jedisct1.github.io/minisign/).

#### Public Key

```text
RWTxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

> **Note:** Replace with actual public key after keypair generation.

#### Verifying a Release

Users can verify downloaded binaries:

```bash
# Install minisign
# macOS: brew install minisign
# Linux: apt install minisign

# Download binary and signature
curl -LO https://github.com/OWNER/REPO/releases/download/vX.Y.Z/flux-x86_64-unknown-linux-gnu
curl -LO https://github.com/OWNER/REPO/releases/download/vX.Y.Z/flux-x86_64-unknown-linux-gnu.minisig

# Verify signature
minisign -Vm flux-x86_64-unknown-linux-gnu -P 'RWTxxxxxx...'
```

#### Release Signing (Maintainers Only)

1. **One-time setup** - Generate keypair (store private key securely):

   ```bash
   minisign -G -p flux.pub -s flux.key
   ```

2. **Sign release binaries**:

   ```bash
   minisign -Sm flux-x86_64-unknown-linux-gnu -s flux.key
   minisign -Sm flux-x86_64-apple-darwin -s flux.key
   minisign -Sm flux-x86_64-pc-windows-msvc.exe -s flux.key
   ```

3. **Upload both binary and `.minisig` file** to the GitHub release.

4. **Update public key** in:
   - `src/commands/self_update.rs:18` (`MINISIGN_PUBLIC_KEY` constant)
   - This file (CONTRIBUTING.md)

### CI/CD Integration

For automated releases, store the private key as a GitHub secret and add to your workflow:

```yaml
- name: Sign release binaries
  env:
    MINISIGN_KEY: ${{ secrets.MINISIGN_PRIVATE_KEY }}
  run: |
    echo "$MINISIGN_KEY" > flux.key
    for binary in flux-*; do
      minisign -Sm "$binary" -s flux.key
    done
    rm flux.key
```

## Security

- Report security vulnerabilities privately via GitHub Security Advisories
- See `doc/plans/PLAN-0002-flux-security-remediation.md` for security audit details
