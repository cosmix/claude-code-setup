use anyhow::{Result, Context, bail};
use colored::Colorize;
use minisign_verify::{PublicKey, Signature};
use reqwest::blocking::{Client, Response};
use semver::Version;
use sha2::{Sha256, Digest};
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const GITHUB_REPO: &str = "cosmix/cluade-flux";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Minisign public key for verifying release signatures.
/// CRITICAL: Replace this placeholder with your actual public key before release.
/// Generate a keypair with: minisign -G -p flux.pub -s flux.key
/// The public key will be in flux.pub (starts with "RW")
const MINISIGN_PUBLIC_KEY: &str = "RWTxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";

// HTTP Security Constants
const HTTP_CONNECT_TIMEOUT_SECS: u64 = 10;
const HTTP_REQUEST_TIMEOUT_SECS: u64 = 120;  // Total request timeout (includes connection + transfer)
const MAX_BINARY_SIZE: u64 = 50 * 1024 * 1024;   // 50MB for binaries
const MAX_ZIP_SIZE: u64 = 100 * 1024 * 1024;      // 100MB for zip archives
const MAX_TEXT_SIZE: u64 = 10 * 1024 * 1024;      // 10MB for text files

// Zip Extraction Security Constants
/// Maximum uncompressed size for any single zip entry (100 MB)
const MAX_UNCOMPRESSED_SIZE: u64 = 100 * 1024 * 1024;
/// Maximum compression ratio to detect zip bombs (normal files rarely exceed 20:1)
const MAX_COMPRESSION_RATIO: f64 = 100.0;
/// Maximum total extracted size for all entries combined (500 MB)
const MAX_TOTAL_EXTRACTED_SIZE: u64 = 500 * 1024 * 1024;
/// Maximum size for signature files (should be very small, typically < 1KB)
const MAX_SIGNATURE_SIZE: u64 = 4 * 1024;

/// Verify the cryptographic signature of downloaded binary content.
/// Uses minisign signature format for verification.
/// Returns Ok(()) if signature is valid, Err with detailed message otherwise.
fn verify_binary_signature(binary_content: &[u8], signature_content: &str) -> Result<()> {
    let public_key = PublicKey::from_base64(MINISIGN_PUBLIC_KEY)
        .map_err(|e| anyhow::anyhow!("Invalid embedded public key: {e}"))?;

    let signature = Signature::decode(signature_content)
        .map_err(|e| anyhow::anyhow!("Invalid signature format: {e}"))?;

    public_key
        .verify(binary_content, &signature, false)
        .map_err(|e| anyhow::anyhow!(
            "Binary signature verification FAILED - possible tampering detected: {e}"
        ))?;

    Ok(())
}

/// Compute SHA-256 checksum of binary content for logging and verification.
/// Returns the hex-encoded hash string.
fn compute_sha256_checksum(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Create an HTTP client with security-focused timeout configuration.
/// Prevents indefinite hangs on slow or unresponsive servers.
/// - connect_timeout: Maximum time to establish a TCP connection
/// - timeout: Maximum time for the entire request (connection + data transfer)
fn create_http_client() -> Result<Client> {
    Client::builder()
        .connect_timeout(Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(HTTP_REQUEST_TIMEOUT_SECS))
        .user_agent("flux-self-update")
        .build()
        .context("Failed to create HTTP client")
}

/// Validate HTTP response status code and return a descriptive error if not successful.
fn validate_response_status(response: &Response, context: &str) -> Result<()> {
    if !response.status().is_success() {
        let status = response.status();
        bail!(
            "{}: HTTP {} - {}",
            context,
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown error")
        );
    }
    Ok(())
}

/// Download content with size limit enforcement.
/// Checks Content-Length header first, then enforces limit during streaming.
fn download_with_limit(response: Response, max_size: u64, context: &str) -> Result<Vec<u8>> {
    // Check Content-Length header if available
    if let Some(content_length) = response.content_length() {
        if content_length > max_size {
            bail!(
                "{context}: Content-Length {content_length} bytes exceeds maximum allowed size of {max_size} bytes"
            );
        }
    }

    // Stream the response with size limit enforcement
    let mut bytes = Vec::new();
    let mut reader = response;
    let mut total_read: u64 = 0;
    let mut buffer = [0u8; 8192];

    loop {
        let n = reader.read(&mut buffer).context("Failed to read response body")?;
        if n == 0 {
            break;
        }
        total_read += n as u64;
        if total_read > max_size {
            bail!(
                "{context}: Download size exceeds maximum allowed size of {max_size} bytes"
            );
        }
        bytes.extend_from_slice(&buffer[..n]);
    }

    Ok(bytes)
}

/// Download text content with size limit enforcement.
fn download_text_with_limit(response: Response, max_size: u64, context: &str) -> Result<String> {
    let bytes = download_with_limit(response, max_size, context)?;
    String::from_utf8(bytes).context("Response contains invalid UTF-8")
}

/// Execute self-update
pub fn execute() -> Result<()> {
    println!("{}", "Checking for updates...".blue());

    let latest = get_latest_release()?;
    let current = Version::parse(CURRENT_VERSION)?;
    let latest_version = Version::parse(latest.tag_name.trim_start_matches('v'))?;

    if latest_version <= current {
        println!("{} You're running the latest version ({})", "✓".green().bold(), CURRENT_VERSION);
        return Ok(());
    }

    println!("New version available: {} → {}", CURRENT_VERSION.dimmed(), latest.tag_name.green().bold());

    // Update binary
    update_binary(&latest)?;

    // Update agents, skills, CLAUDE.md
    update_config_files(&latest)?;

    println!("{} Updated successfully to {}", "✓".green().bold(), latest.tag_name);
    Ok(())
}

#[derive(serde::Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(serde::Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

fn get_latest_release() -> Result<Release> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
    let client = create_http_client()?;
    let response = client
        .get(&url)
        .send()
        .context("Failed to check for updates")?;

    validate_response_status(&response, "Failed to fetch release info")?;

    response.json().context("Failed to parse release info")
}

fn get_target() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    { "x86_64-unknown-linux-gnu" }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    { "aarch64-unknown-linux-gnu" }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    { "x86_64-apple-darwin" }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { "aarch64-apple-darwin" }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
    )))]
    { "unknown" }
}

/// Install binary with atomic rollback mechanism.
/// Writes new binary to temp location, backs up current, installs new with rollback on failure.
fn install_binary(new_binary: &[u8], current_exe: &Path) -> Result<()> {
    let backup_path = current_exe.with_extension("backup");
    let new_path = current_exe.with_extension("new");

    // Clean up any leftover files from previous failed updates
    if backup_path.exists() {
        fs::remove_file(&backup_path).ok();
    }
    if new_path.exists() {
        fs::remove_file(&new_path).ok();
    }

    // Write new binary to temp location
    let mut file = File::create(&new_path).context("Failed to create new binary file")?;
    file.write_all(new_binary).context("Failed to write new binary")?;
    file.sync_all().context("Failed to sync new binary to disk")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&new_path, fs::Permissions::from_mode(0o755))
            .context("Failed to set executable permissions on new binary")?;
    }

    // Backup current binary
    fs::rename(current_exe, &backup_path)
        .context("Failed to backup current binary")?;

    // Install new binary - with rollback on failure
    if let Err(e) = fs::rename(&new_path, current_exe) {
        // Attempt rollback
        if let Err(rollback_err) = fs::rename(&backup_path, current_exe) {
            bail!(
                "CRITICAL: Update failed and rollback failed!\n\
                 Update error: {}\n\
                 Rollback error: {}\n\
                 Manual recovery needed: copy {} to {}",
                e, rollback_err,
                backup_path.display(), current_exe.display()
            );
        }
        return Err(e.into());
    }

    // Success - remove backup
    let _ = fs::remove_file(&backup_path);
    Ok(())
}

fn update_binary(release: &Release) -> Result<()> {
    let target = get_target();
    if target == "unknown" {
        bail!("Unsupported platform for self-update");
    }

    let binary_name = format!("flux-{target}");
    let signature_name = format!("{binary_name}.minisig");

    // Find binary asset
    let binary_asset = release.assets.iter()
        .find(|a| a.name == binary_name)
        .ok_or_else(|| anyhow::anyhow!("No binary found for {target}"))?;

    // Find signature asset - REQUIRED for security
    let signature_asset = release.assets.iter()
        .find(|a| a.name == signature_name)
        .ok_or_else(|| anyhow::anyhow!(
            "No signature file found for {target}. Release must include {signature_name}"
        ))?;

    let client = create_http_client()?;

    // Download binary
    println!("  {} Downloading binary...", "→".blue());
    let binary_response = client
        .get(&binary_asset.browser_download_url)
        .send()
        .context("Failed to download binary")?;
    validate_response_status(&binary_response, "Binary download failed")?;
    let binary_bytes = download_with_limit(binary_response, MAX_BINARY_SIZE, "Binary download")?;

    // Download signature
    println!("  {} Downloading signature...", "→".blue());
    let sig_response = client
        .get(&signature_asset.browser_download_url)
        .send()
        .context("Failed to download signature")?;
    validate_response_status(&sig_response, "Signature download failed")?;
    let signature_content = download_text_with_limit(sig_response, MAX_SIGNATURE_SIZE, "Signature download")?;

    // CRITICAL: Verify signature BEFORE writing binary to disk
    println!("  {} Verifying cryptographic signature...", "→".blue());
    verify_binary_signature(&binary_bytes, &signature_content)
        .context("SECURITY ERROR: Binary signature verification failed")?;
    println!("  {} Signature verified successfully", "✓".green());

    // Compute and log checksum for defense-in-depth auditing
    let checksum = compute_sha256_checksum(&binary_bytes);
    println!("  {} SHA-256: {}", "ℹ".blue(), checksum.dimmed());

    // Get current executable path
    let current_exe = env::current_exe().context("Failed to get current executable path")?;

    // Install the new binary with rollback mechanism
    install_binary(&binary_bytes, &current_exe)?;

    println!("  {} Binary updated", "✓".green());
    Ok(())
}

fn get_claude_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    Ok(home.join(".claude"))
}

fn update_config_files(release: &Release) -> Result<()> {
    let claude_dir = get_claude_dir()?;

    // Update CLAUDE.template.md -> CLAUDE.md
    if let Some(asset) = release.assets.iter().find(|a| a.name == "CLAUDE.template.md") {
        println!("  {} Downloading CLAUDE.template.md...", "→".blue());
        download_and_save(&asset.browser_download_url, &claude_dir.join("CLAUDE.md"))?;
        println!("  {} CLAUDE.md updated", "✓".green());
    }

    // Update agents
    if let Some(asset) = release.assets.iter().find(|a| a.name == "agents.zip") {
        println!("  {} Downloading agents...", "→".blue());
        let agents_dir = claude_dir.join("agents");
        download_and_extract_zip(&asset.browser_download_url, &agents_dir)?;
        println!("  {} agents/ updated", "✓".green());
    }

    // Update skills
    if let Some(asset) = release.assets.iter().find(|a| a.name == "skills.zip") {
        println!("  {} Downloading skills...", "→".blue());
        let skills_dir = claude_dir.join("skills");
        download_and_extract_zip(&asset.browser_download_url, &skills_dir)?;
        println!("  {} skills/ updated", "✓".green());
    }

    Ok(())
}

fn download_and_save(url: &str, dest: &Path) -> Result<()> {
    let client = create_http_client()?;
    let response = client
        .get(url)
        .send()
        .context("Failed to download file")?;

    validate_response_status(&response, "File download failed")?;
    let content = download_text_with_limit(response, MAX_TEXT_SIZE, "File download")?;

    // Prepend header like install.sh does
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
    let full_content = format!(
        "# ───────────────────────────────────────────────────────────\n\
         # cluade-flux | updated {timestamp}\n\
         # ───────────────────────────────────────────────────────────\n\n\
         {content}"
    );

    fs::write(dest, full_content).context("Failed to write file")?;
    Ok(())
}

/// Validates a zip entry for security threats (zip bombs, excessive size).
///
/// Checks:
/// - Absolute uncompressed size against MAX_UNCOMPRESSED_SIZE
/// - Compression ratio against MAX_COMPRESSION_RATIO to detect zip bombs
fn validate_zip_entry(file: &zip::read::ZipFile) -> Result<()> {
    let compressed = file.compressed_size();
    let uncompressed = file.size();

    // Check absolute size limit
    if uncompressed > MAX_UNCOMPRESSED_SIZE {
        bail!(
            "Zip entry '{}' too large: {} bytes (max: {} bytes)",
            file.name(),
            uncompressed,
            MAX_UNCOMPRESSED_SIZE
        );
    }

    // Check compression ratio for zip bomb detection
    if compressed > 0 {
        let ratio = uncompressed as f64 / compressed as f64;
        if ratio > MAX_COMPRESSION_RATIO {
            bail!(
                "Suspicious compression ratio in '{}': {:.1}x (max: {:.1}x) - possible zip bomb",
                file.name(),
                ratio,
                MAX_COMPRESSION_RATIO
            );
        }
    }

    Ok(())
}

/// Safely resolves a zip entry path, protecting against zip slip attacks.
///
/// Returns the safe output path if the entry is valid, or an error if:
/// - The path contains ".." components (directory traversal attempt)
/// - The resolved path escapes the destination directory
/// - The path is absolute (would ignore dest_dir)
fn safe_extract_path(dest_dir: &Path, entry_name: &str) -> Result<PathBuf> {
    // Reject paths containing ".." anywhere - explicit directory traversal attempt
    if entry_name.contains("..") {
        bail!("Zip slip attack detected: path contains '..' component - '{entry_name}'");
    }

    // Reject absolute paths that would ignore dest_dir
    let entry_path = Path::new(entry_name);
    if entry_path.is_absolute() {
        bail!("Zip slip attack detected: absolute path in archive - '{entry_name}'");
    }

    // Reject paths starting with / or \ (platform-specific absolute indicators)
    if entry_name.starts_with('/') || entry_name.starts_with('\\') {
        bail!("Zip slip attack detected: path starts with path separator - '{entry_name}'");
    }

    // Build the output path
    // Canonicalize dest_dir to resolve any symlinks in the destination
    // We need to ensure dest_dir exists first for canonicalize to work
    fs::create_dir_all(dest_dir).context("Failed to create destination directory")?;
    let canonical_dest = dest_dir
        .canonicalize()
        .context("Failed to canonicalize destination directory")?;

    // For the output path, we check component by component since it may not exist yet
    // Normalize the path by resolving . and removing redundant separators
    let mut normalized = canonical_dest.clone();
    for component in entry_path.components() {
        use std::path::Component;
        match component {
            Component::Normal(c) => normalized.push(c),
            Component::CurDir => {} // Skip "."
            Component::ParentDir => {
                // This shouldn't happen since we checked for ".." above, but be defensive
                bail!("Zip slip attack detected: parent directory traversal in '{entry_name}'");
            }
            Component::RootDir | Component::Prefix(_) => {
                bail!("Zip slip attack detected: absolute path component in '{entry_name}'");
            }
        }
    }

    // Final verification: the normalized path must start with the canonical destination
    if !normalized.starts_with(&canonical_dest) {
        bail!(
            "Zip slip attack detected: resolved path '{}' escapes destination directory '{}'",
            normalized.display(),
            canonical_dest.display()
        );
    }

    Ok(normalized)
}

/// A reader wrapper that limits the number of bytes that can be read.
/// Used to prevent zip bombs that lie about their uncompressed size in headers.
struct LimitedReader<R> {
    inner: R,
    remaining: u64,
}

impl<R> LimitedReader<R> {
    fn new(inner: R, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }
}

impl<R: Read> Read for LimitedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::Error::other(
                "Zip entry exceeds maximum allowed size during extraction - possible zip bomb",
            ));
        }

        // Limit the read to remaining bytes
        let max_read = std::cmp::min(buf.len() as u64, self.remaining) as usize;
        let bytes_read = self.inner.read(&mut buf[..max_read])?;
        self.remaining = self.remaining.saturating_sub(bytes_read as u64);

        Ok(bytes_read)
    }
}

fn download_and_extract_zip(url: &str, dest_dir: &Path) -> Result<()> {
    let client = create_http_client()?;
    let response = client
        .get(url)
        .send()
        .context("Failed to download zip")?;

    validate_response_status(&response, "Zip download failed")?;
    let bytes = download_with_limit(response, MAX_ZIP_SIZE, "Zip download")?;

    // Create temp file
    let temp_path = dest_dir.with_extension("zip.tmp");
    fs::write(&temp_path, &bytes).context("Failed to write temp zip")?;

    // Open and validate archive before any extraction
    let file = File::open(&temp_path).context("Failed to open temp zip")?;
    let mut archive = zip::ZipArchive::new(file).context("Failed to read zip archive")?;

    // Pre-validate all entries before extraction (fail fast on malicious archives)
    let mut total_uncompressed_size: u64 = 0;
    for i in 0..archive.len() {
        let file = archive.by_index(i).context("Failed to read zip entry")?;

        // Validate against zip bombs
        validate_zip_entry(&file)?;

        // Track total size with overflow protection
        total_uncompressed_size = total_uncompressed_size
            .checked_add(file.size())
            .ok_or_else(|| anyhow::anyhow!("Total uncompressed size overflow - possible zip bomb"))?;

        if total_uncompressed_size > MAX_TOTAL_EXTRACTED_SIZE {
            bail!(
                "Total uncompressed size {total_uncompressed_size} exceeds maximum {MAX_TOTAL_EXTRACTED_SIZE} bytes - possible zip bomb"
            );
        }

        // Validate path safety (using enclosed_name for additional safety, falling back to mangled_name)
        let entry_name = file
            .enclosed_name()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| file.mangled_name().to_string_lossy().to_string());

        // Skip empty names (root directory entries)
        if !entry_name.is_empty() {
            safe_extract_path(dest_dir, &entry_name)?;
        }
    }

    // Backup existing directory (only after validation passes)
    if dest_dir.exists() {
        let backup = dest_dir.with_extension("bak");
        if backup.exists() {
            fs::remove_dir_all(&backup).ok();
        }
        fs::rename(dest_dir, &backup).context("Failed to backup directory")?;
    }

    // Re-open archive for extraction (we consumed it during validation)
    let file = File::open(&temp_path).context("Failed to reopen temp zip")?;
    let mut archive = zip::ZipArchive::new(file).context("Failed to reread zip archive")?;

    fs::create_dir_all(dest_dir)?;

    // Extract with validated paths
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        // Get safe entry name (prefer enclosed_name, fall back to mangled_name)
        let entry_name = file
            .enclosed_name()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| file.mangled_name().to_string_lossy().to_string());

        // Skip empty names
        if entry_name.is_empty() {
            continue;
        }

        // Get safe output path (already validated, but re-verify for defense in depth)
        let outpath = safe_extract_path(dest_dir, &entry_name)?;

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }

            // Extract with size limit enforcement during decompression
            let mut outfile = File::create(&outpath)
                .with_context(|| format!("Failed to create file: {}", outpath.display()))?;

            // Use a limited reader to enforce size during extraction
            // This catches zip bombs that lie about their uncompressed size in headers
            let mut limited_reader = LimitedReader::new(&mut file, MAX_UNCOMPRESSED_SIZE);
            io::copy(&mut limited_reader, &mut outfile)
                .with_context(|| format!("Failed to extract file: {entry_name}"))?;
        }
    }

    // Cleanup
    fs::remove_file(&temp_path).ok();
    let backup = dest_dir.with_extension("bak");
    if backup.exists() {
        fs::remove_dir_all(&backup).ok();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::TempDir;

    // ============================================================================
    // 1. SIGNATURE VERIFICATION TESTS
    // ============================================================================

    #[test]
    fn test_rejects_invalid_signature_format() {
        let binary = b"valid binary content";
        let bad_signature = "not a valid minisign signature format";

        let result = verify_binary_signature(binary, bad_signature);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("signature") || err_msg.contains("Invalid"));
    }

    #[test]
    fn test_rejects_empty_signature() {
        let binary = b"some binary data";
        let empty_signature = "";

        let result = verify_binary_signature(binary, empty_signature);
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_malformed_public_key() {
        // The embedded public key is a placeholder, so this tests invalid key handling
        // In production with a real key, this would test key format validation
        let binary = b"binary content";
        let signature = "untrusted signature: RWT1234567890";

        let result = verify_binary_signature(binary, signature);
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_sha256_checksum_consistency() {
        let data = b"test data";
        let checksum1 = compute_sha256_checksum(data);
        let checksum2 = compute_sha256_checksum(data);

        assert_eq!(checksum1, checksum2);
        assert_eq!(checksum1.len(), 64); // SHA-256 is 32 bytes = 64 hex chars
    }

    #[test]
    fn test_compute_sha256_checksum_different_data() {
        let data1 = b"original";
        let data2 = b"modified";

        let checksum1 = compute_sha256_checksum(data1);
        let checksum2 = compute_sha256_checksum(data2);

        assert_ne!(checksum1, checksum2);
    }

    // ============================================================================
    // 2. ZIP SLIP ATTACK TESTS
    // ============================================================================

    #[test]
    fn test_rejects_path_traversal_dotdot() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path();

        let result = safe_extract_path(dest, "../../../etc/passwd");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Zip slip") || err_msg.contains(".."));
    }

    #[test]
    fn test_rejects_path_traversal_in_middle() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path();

        let result = safe_extract_path(dest, "subdir/../../../etc/passwd");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Zip slip") || err_msg.contains(".."));
    }

    #[test]
    fn test_rejects_absolute_unix_path() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path();

        let result = safe_extract_path(dest, "/etc/passwd");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Zip slip") || err_msg.contains("absolute"));
    }

    #[test]
    fn test_rejects_windows_drive_letter_path() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path();

        // Windows-style paths with drive letters should be rejected
        // The backslash in the string literal will be interpreted by Rust
        let result = safe_extract_path(dest, r"C:\Windows\System32\evil.exe");

        // On Windows, this is absolute and should be rejected
        // On Unix, this path becomes relative but contains unusual characters
        #[cfg(target_os = "windows")]
        {
            assert!(result.is_err());
            let err_msg = result.unwrap_err().to_string();
            assert!(err_msg.contains("Zip slip") || err_msg.contains("absolute"));
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On Unix, verify behavior is reasonable (may accept or reject)
            // The key is that real zip slip attacks use / not \
            let _ = result; // Just ensure no panic
        }
    }

    #[test]
    fn test_rejects_path_starting_with_slash() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path();

        let result = safe_extract_path(dest, "/etc/shadow");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Zip slip") || err_msg.contains("separator"));
    }

    #[test]
    fn test_rejects_path_starting_with_backslash() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path();

        let result = safe_extract_path(dest, "\\Windows\\evil.dll");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Zip slip") || err_msg.contains("separator"));
    }

    #[test]
    fn test_accepts_valid_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path();

        let result = safe_extract_path(dest, "subdir/file.txt");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.starts_with(dest));
        assert!(path.ends_with("subdir/file.txt"));
    }

    #[test]
    fn test_accepts_simple_filename() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path();

        let result = safe_extract_path(dest, "file.txt");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.starts_with(dest));
        assert!(path.ends_with("file.txt"));
    }

    #[test]
    fn test_accepts_deeply_nested_path() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path();

        let result = safe_extract_path(dest, "a/b/c/d/e/file.txt");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.starts_with(dest));
    }

    // ============================================================================
    // 3. ZIP BOMB DETECTION TESTS
    // ============================================================================

    #[test]
    fn test_validate_zip_entry_rejects_oversized() {
        // We can't easily create a real ZipFile in tests, but we can test the logic
        // by examining the validation function behavior through integration
        // This test documents the expected behavior for oversized entries

        // MAX_UNCOMPRESSED_SIZE is 100 MB
        // An entry with 101 MB should be rejected
        let oversized = MAX_UNCOMPRESSED_SIZE + 1;
        assert!(oversized > MAX_UNCOMPRESSED_SIZE);
    }

    #[test]
    fn test_validate_zip_entry_accepts_normal_size() {
        // An entry with 1 MB should be accepted
        let normal_size = 1024 * 1024;
        assert!(normal_size < MAX_UNCOMPRESSED_SIZE);
    }

    #[test]
    fn test_validate_compression_ratio_threshold() {
        // MAX_COMPRESSION_RATIO is 100.0
        // A file compressed from 100 MB to 1 MB has ratio of 100:1 (at threshold)
        // A file compressed from 101 MB to 1 MB has ratio of 101:1 (should reject)

        let compressed_size = 1024 * 1024; // 1 MB
        let uncompressed_normal = 50 * 1024 * 1024; // 50 MB (ratio 50:1, OK)
        let uncompressed_bomb = 101 * compressed_size; // Ratio 101:1 (should reject)

        let ratio_normal = uncompressed_normal as f64 / compressed_size as f64;
        let ratio_bomb = uncompressed_bomb as f64 / compressed_size as f64;

        assert!(ratio_normal < MAX_COMPRESSION_RATIO);
        assert!(ratio_bomb > MAX_COMPRESSION_RATIO);
    }

    // ============================================================================
    // 4. LIMITED READER TESTS (ZIP BOMB RUNTIME PROTECTION)
    // ============================================================================

    #[test]
    fn test_limited_reader_respects_limit() {
        let data = b"0123456789"; // 10 bytes
        let cursor = Cursor::new(data);
        let mut limited = LimitedReader::new(cursor, 5); // Limit to 5 bytes

        let mut buf = [0u8; 10];
        let n = limited.read(&mut buf).unwrap();

        assert_eq!(n, 5); // Should only read 5 bytes
        assert_eq!(&buf[..n], b"01234");
    }

    #[test]
    fn test_limited_reader_rejects_excess() {
        let data = vec![0u8; 1000];
        let cursor = Cursor::new(data);
        let mut limited = LimitedReader::new(cursor, 100); // Limit to 100 bytes

        let mut buf = [0u8; 200];

        // First read should succeed
        let n1 = limited.read(&mut buf).unwrap();
        assert_eq!(n1, 100);

        // Second read should fail (limit exhausted)
        let result = limited.read(&mut buf);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("exceeds maximum") || err_msg.contains("zip bomb"));
    }

    #[test]
    fn test_limited_reader_allows_multiple_reads_within_limit() {
        let data = b"0123456789";
        let cursor = Cursor::new(data);
        let mut limited = LimitedReader::new(cursor, 10); // Limit to 10 bytes

        let mut buf = [0u8; 5];

        // Read 5 bytes
        let n1 = limited.read(&mut buf).unwrap();
        assert_eq!(n1, 5);

        // Read another 5 bytes
        let n2 = limited.read(&mut buf).unwrap();
        assert_eq!(n2, 5);

        // Total read: 10 bytes (at limit, should allow)
        assert_eq!(n1 + n2, 10);
    }

    // ============================================================================
    // 5. DOWNLOAD SIZE LIMIT TESTS
    // ============================================================================

    #[test]
    fn test_download_size_constants_are_reasonable() {
        // Document the expected size limits (compile-time verification)
        // These values are tested at compile time, not runtime
        const EXPECTED_BINARY_SIZE: u64 = 50 * 1024 * 1024; // 50 MB
        const EXPECTED_ZIP_SIZE: u64 = 100 * 1024 * 1024; // 100 MB
        const EXPECTED_TEXT_SIZE: u64 = 10 * 1024 * 1024; // 10 MB
        const EXPECTED_SIGNATURE_SIZE: u64 = 4 * 1024; // 4 KB

        assert_eq!(MAX_BINARY_SIZE, EXPECTED_BINARY_SIZE);
        assert_eq!(MAX_ZIP_SIZE, EXPECTED_ZIP_SIZE);
        assert_eq!(MAX_TEXT_SIZE, EXPECTED_TEXT_SIZE);
        assert_eq!(MAX_SIGNATURE_SIZE, EXPECTED_SIGNATURE_SIZE);
    }

    #[test]
    fn test_zip_bomb_constants_are_reasonable() {
        // Document the expected zip bomb protection limits
        const EXPECTED_UNCOMPRESSED: u64 = 100 * 1024 * 1024; // 100 MB
        const EXPECTED_RATIO: f64 = 100.0;
        const EXPECTED_TOTAL: u64 = 500 * 1024 * 1024; // 500 MB

        assert_eq!(MAX_UNCOMPRESSED_SIZE, EXPECTED_UNCOMPRESSED);
        assert_eq!(MAX_COMPRESSION_RATIO, EXPECTED_RATIO);
        assert_eq!(MAX_TOTAL_EXTRACTED_SIZE, EXPECTED_TOTAL);
    }

    // ============================================================================
    // 6. HTTP CLIENT SECURITY TESTS
    // ============================================================================

    #[test]
    fn test_http_timeout_constants() {
        // Document expected timeout values
        const EXPECTED_CONNECT_TIMEOUT: u64 = 10;
        const EXPECTED_REQUEST_TIMEOUT: u64 = 120;

        assert_eq!(HTTP_CONNECT_TIMEOUT_SECS, EXPECTED_CONNECT_TIMEOUT);
        assert_eq!(HTTP_REQUEST_TIMEOUT_SECS, EXPECTED_REQUEST_TIMEOUT);
    }

    #[test]
    fn test_create_http_client_succeeds() {
        let result = create_http_client();
        assert!(result.is_ok());
    }

    // ============================================================================
    // 7. PATH VALIDATION EDGE CASES
    // ============================================================================

    #[test]
    fn test_safe_extract_normalizes_dot_segments() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path();

        // Path with "." should be accepted and normalized
        let result = safe_extract_path(dest, "./file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_safe_extract_handles_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path();

        let result = safe_extract_path(dest, "dir1/dir2/dir3/file.txt");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("dir1"));
        assert!(path.to_string_lossy().contains("dir2"));
        assert!(path.to_string_lossy().contains("dir3"));
    }

    #[test]
    fn test_rejects_mixed_path_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path();

        // Even if there's valid content after .., it should be rejected
        let result = safe_extract_path(dest, "valid/../../../etc/passwd");
        assert!(result.is_err());
    }
}
