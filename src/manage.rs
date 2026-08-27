//! Self-management: `joshify update` and `joshify uninstall`.
//!
//! These deliberately mirror what `install.sh` does, because a user who
//! installed with the script and then runs `joshify update` must end up in the
//! same place. In particular: resolve the latest release, verify the download
//! against the published `SHA256SUMS`, and refuse to install on a mismatch.

use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const REPO_SLUG: &str = "bigknoxy/joshify";

/// What `joshify update` was asked to do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateOptions {
    /// Report whether an update exists and change nothing.
    pub check_only: bool,
    /// Install this tag instead of the latest release.
    pub version: Option<String>,
}

/// What `joshify uninstall` was asked to do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UninstallOptions {
    /// Also delete config, credentials and cache.
    pub purge: bool,
    /// Do not prompt before destructive steps.
    pub assume_yes: bool,
}

// --- pure helpers ------------------------------------------------------------
// Kept free of I/O so they can be tested without a network or a real install.

/// The release asset for a platform, or `None` when no binary is published.
///
/// Must stay in sync with the release workflow matrix and with
/// `release_asset_name` in install.sh.
pub fn release_asset_name(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") => Some("joshify-linux-x86_64.tar.gz"),
        ("macos", "aarch64") => Some("joshify-macos-aarch64.tar.gz"),
        _ => None,
    }
}

/// The asset for the platform this binary was built for.
pub fn current_asset_name() -> Option<&'static str> {
    release_asset_name(std::env::consts::OS, std::env::consts::ARCH)
}

/// Strip a leading `v` so tags and crate versions compare cleanly.
pub fn normalize_version(version: &str) -> &str {
    version.strip_prefix('v').unwrap_or(version)
}

/// Whether `latest` differs from `current`.
///
/// Deliberately an inequality rather than an ordering: a pinned older tag is a
/// legitimate target, and joshify does not promise semver ordering across
/// pre-release tags.
pub fn needs_update(current: &str, latest: &str) -> bool {
    normalize_version(current) != normalize_version(latest)
}

/// Pull `tag_name` out of a GitHub release payload.
pub fn parse_latest_tag(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    value.get("tag_name")?.as_str().map(str::to_string)
}

/// Find the expected digest for `asset` in a `SHA256SUMS` file.
pub fn expected_digest(sums: &str, asset: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (digest, name) = line.split_once("  ")?;
        (name.trim() == asset).then(|| digest.trim().to_lowercase())
    })
}

/// Hex-encoded SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut acc, byte| {
            use std::fmt::Write;
            let _ = write!(acc, "{byte:02x}");
            acc
        })
}

/// Whether `joshify uninstall` must ask before it deletes anything.
///
/// Removing the binary is destructive too, so a plain `uninstall` confirms as
/// well - not only `--purge`. Only `--yes` skips the prompt.
pub fn needs_confirmation(options: &UninstallOptions) -> bool {
    !options.assume_yes
}

/// Everything `joshify uninstall` would remove, decided without touching disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UninstallPlan {
    pub binary: Option<PathBuf>,
    /// Config, credentials and cache. Empty unless `--purge`.
    pub data: Vec<PathBuf>,
    pub remove_keyring: bool,
}

/// Decide what to remove. `config_dir`/`cache_dir` are passed in so this stays
/// testable against a temporary directory.
pub fn plan_uninstall(
    binary: Option<PathBuf>,
    config_dir: &Path,
    cache_dir: &Path,
    options: &UninstallOptions,
) -> UninstallPlan {
    let data = if options.purge {
        vec![config_dir.to_path_buf(), cache_dir.to_path_buf()]
    } else {
        Vec::new()
    };

    UninstallPlan {
        binary,
        data,
        remove_keyring: options.purge,
    }
}

// --- I/O ---------------------------------------------------------------------

/// Fetch a URL.
///
/// Async on purpose: this runs inside the `#[tokio::main]` runtime, and
/// `reqwest::blocking` panics when constructed there rather than returning an
/// error, which would abort every update.
async fn download(url: &str) -> Result<Vec<u8>> {
    let response = reqwest::Client::builder()
        .user_agent(concat!("joshify/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(300))
        .build()?
        .get(url)
        .send()
        .await
        .with_context(|| format!("requesting {url}"))?;

    if !response.status().is_success() {
        bail!("{url} returned HTTP {}", response.status());
    }
    Ok(response.bytes().await?.to_vec())
}

async fn fetch_text(url: &str) -> Result<String> {
    Ok(String::from_utf8_lossy(&download(url).await?).into_owned())
}

/// Replace `dest` with `bytes` atomically.
///
/// The staged file is written next to the destination so the rename cannot
/// cross a filesystem boundary, and a running process keeps its open handle to
/// the old inode.
pub fn atomic_replace(dest: &Path, bytes: &[u8]) -> Result<()> {
    let staged = stage_beside(dest, bytes)?;
    commit_staged(&staged, dest)
}

/// Where the new binary is written before it takes over from `dest`.
pub fn staging_path_for(dest: &Path) -> Result<PathBuf> {
    let dir = dest
        .parent()
        .ok_or_else(|| anyhow!("{} has no parent directory", dest.display()))?;
    Ok(dir.join(format!(
        ".{}.new",
        dest.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("joshify")
    )))
}

/// Write the new binary next to `dest` and make it executable.
///
/// Deliberately beside the destination rather than in a temp directory. The
/// update smoke-tests the new binary by running it, and hardened systems mount
/// `/tmp` with `noexec` - where `execve` fails with `EACCES` ("Permission
/// denied") no matter what the file's mode bits say. The directory the running
/// binary lives in is by definition allowed to execute, and staging there also
/// keeps the final rename on a single filesystem, which is what makes it
/// atomic.
pub fn stage_beside(dest: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let staged = staging_path_for(dest)?;

    std::fs::write(&staged, bytes)
        .with_context(|| format!("writing {} (is it writable?)", staged.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)) {
            let _ = std::fs::remove_file(&staged);
            return Err(e).with_context(|| format!("making {} executable", staged.display()));
        }
    }

    Ok(staged)
}

/// Move a staged binary into place, cleaning up if the rename fails.
pub fn commit_staged(staged: &Path, dest: &Path) -> Result<()> {
    std::fs::rename(staged, dest)
        .inspect_err(|_| {
            let _ = std::fs::remove_file(staged);
        })
        .with_context(|| format!("replacing {}", dest.display()))?;
    Ok(())
}

/// Explain an exec failure in the terms most likely to be the actual cause.
///
/// `EACCES` from `execve` on a file we just wrote and chmod'ed almost always
/// means the filesystem is mounted `noexec`, not that the permissions are
/// wrong - a distinction the raw "Permission denied" hides.
fn describe_exec_failure(path: &Path, e: &std::io::Error) -> String {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        format!(
            "cannot execute the downloaded binary at {}: {e}\n\
             The file is executable, so the filesystem holding it is most likely \
             mounted `noexec`. Install with the script instead:\n  \
             curl -fsSL https://raw.githubusercontent.com/{REPO_SLUG}/main/install.sh | bash",
            path.display()
        )
    } else {
        format!("running the downloaded binary at {}: {e}", path.display())
    }
}

/// Extract the single binary from a release tarball.
fn extract_binary(tarball: &[u8], asset: &str, into: &Path) -> Result<PathBuf> {
    let expected = asset.trim_end_matches(".tar.gz");
    let status = std::process::Command::new("tar")
        .arg("-xzf")
        .arg("-")
        .arg("-C")
        .arg(into)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .expect("stdin was piped")
                .write_all(tarball)?;
            child.wait()
        })
        .context("extracting the release tarball")?;

    if !status.success() {
        bail!("tar exited with {status}");
    }

    let path = into.join(expected);
    if !path.exists() {
        bail!("{expected} was not found inside {asset}");
    }
    Ok(path)
}

/// Run `joshify update`.
pub async fn run_update(options: &UpdateOptions) -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");

    let Some(asset) = current_asset_name() else {
        bail!(
            "No prebuilt binary is published for {}/{}. \
             Reinstall from source instead:\n  \
             curl -fsSL https://raw.githubusercontent.com/{REPO_SLUG}/main/install.sh | bash",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    };

    let tag = match &options.version {
        Some(pinned) => pinned.clone(),
        None => {
            let body = fetch_text(&format!(
                "https://api.github.com/repos/{REPO_SLUG}/releases/latest"
            ))
            .await
            .context("asking GitHub for the latest release")?;
            parse_latest_tag(&body)
                .ok_or_else(|| anyhow!("could not read a tag name from the GitHub response"))?
        }
    };

    let target = normalize_version(&tag);
    if !needs_update(current, target) {
        println!("joshify {current} is already the latest release.");
        return Ok(());
    }

    println!("Update available: {current} -> {target}");
    if options.check_only {
        return Ok(());
    }

    let base = format!("https://github.com/{REPO_SLUG}/releases/download/{tag}");
    println!("Downloading {asset}...");
    let tarball = download(&format!("{base}/{asset}")).await?;

    // A release without checksums cannot be verified, and an unverified binary
    // is not something to silently install over the running one.
    let sums = fetch_text(&format!("{base}/SHA256SUMS"))
        .await
        .context("this release publishes no SHA256SUMS, so the download cannot be verified")?;
    let expected = expected_digest(&sums, asset)
        .ok_or_else(|| anyhow!("{asset} is not listed in SHA256SUMS"))?;
    let actual = sha256_hex(&tarball);
    if actual != expected {
        bail!("checksum mismatch for {asset}\n  expected {expected}\n  got      {actual}");
    }
    println!("Checksum verified.");

    let dest = std::env::current_exe().context("locating the running binary")?;
    let unpacked = tempfile::tempdir().context("creating a staging directory")?;
    let extracted = extract_binary(&tarball, asset, unpacked.path())?;

    // Stage beside the destination before the smoke test, not in the temp
    // directory: running it from /tmp fails with EACCES wherever /tmp is
    // mounted noexec, which is common on hardened and enterprise images.
    let staged = stage_beside(&dest, &std::fs::read(&extracted)?)?;

    // Confirm the new binary runs before it replaces the running one.
    let probe = match std::process::Command::new(&staged)
        .arg("--version")
        .output()
    {
        Ok(probe) => probe,
        Err(e) => {
            let _ = std::fs::remove_file(&staged);
            bail!("{}", describe_exec_failure(&staged, &e));
        }
    };
    let reported = String::from_utf8_lossy(&probe.stdout).trim().to_string();
    if !probe.status.success() || !reported.starts_with(crate::VERSION_PREFIX) {
        let _ = std::fs::remove_file(&staged);
        bail!("the downloaded binary did not report a version; not installing it\n{reported}");
    }

    commit_staged(&staged, &dest)?;
    println!("Updated to {reported} at {}", dest.display());
    Ok(())
}

/// Run `joshify uninstall`.
pub fn run_uninstall(options: &UninstallOptions) -> Result<()> {
    let binary = std::env::current_exe().ok();
    let config_dir = crate::auth::get_config_dir().unwrap_or_default();
    let cache_dir = dirs_next::cache_dir()
        .map(|d| d.join("joshify"))
        .unwrap_or_default();

    let plan = plan_uninstall(binary, &config_dir, &cache_dir, options);

    println!("This will remove:");
    if let Some(ref path) = plan.binary {
        println!("  {}", path.display());
    }
    for path in &plan.data {
        println!("  {} (config, credentials, cache)", path.display());
    }
    if plan.remove_keyring {
        println!("  the saved credentials in your OS keyring");
    }
    if plan.data.is_empty() {
        println!("\nConfig and cache are kept. Use --purge to remove them too.");
    }

    // Removing the binary is destructive too, so confirm for any run that is
    // going to delete something - not just --purge.
    if needs_confirmation(options) {
        let prompt = if options.purge {
            "\nDelete the binary, credentials and config?"
        } else {
            "\nRemove joshify?"
        };
        if !confirm(prompt)? {
            println!("Cancelled.");
            return Ok(());
        }
    }

    for path in &plan.data {
        if path.exists() {
            std::fs::remove_dir_all(path)
                .with_context(|| format!("removing {}", path.display()))?;
            println!("Removed {}", path.display());
        }
    }

    if plan.remove_keyring {
        match crate::keyring_store::delete_credentials_keyring() {
            Ok(()) => println!("Removed the keyring entry."),
            Err(e) => println!("No keyring entry removed: {e}"),
        }
    }

    // The binary goes last: on Unix the running executable can be unlinked and
    // this process keeps running, but nothing may re-read it afterwards.
    if let Some(ref path) = plan.binary {
        std::fs::remove_file(path).with_context(|| format!("removing {}", path.display()))?;
        println!("Removed {}", path.display());
    }

    println!("\nJoshify uninstalled.");
    Ok(())
}

fn confirm(prompt: &str) -> Result<bool> {
    use std::io::{BufRead, Write};
    print!("{prompt} [y/N] ");
    std::io::stdout().flush()?;

    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer)?;
    Ok(matches!(answer.trim().to_lowercase().as_str(), "y" | "yes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serve one minimal HTTP response on an ephemeral loopback port.
    async fn serve_once(body: &'static str) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let addr = listener.local_addr().expect("addr");

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut scratch = [0u8; 1024];
                let _ = socket.read(&mut scratch).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });

        format!("http://{addr}/")
    }

    /// Regression guard for the review finding that mattered most.
    ///
    /// `run_update` is dispatched from inside `#[tokio::main]`. A
    /// `reqwest::blocking` client panics when built there ("cannot block the
    /// current thread from within a runtime") rather than returning an error,
    /// so every `joshify update` aborted on its first request. The CLI could
    /// not catch it: on a platform with no published asset the asset check
    /// returns before any network call happens.
    ///
    /// Hermetic - loopback only, no external network.
    #[tokio::test]
    async fn http_client_works_inside_the_tokio_runtime() {
        let url = serve_once(r#"{"tag_name":"v9.9.9"}"#).await;

        let body = fetch_text(&url)
            .await
            .expect("fetching inside a tokio runtime must not panic or fail");

        assert_eq!(parse_latest_tag(&body), Some("v9.9.9".to_string()));
    }

    #[tokio::test]
    async fn http_errors_are_reported_not_panicked() {
        // Nothing is listening on this port.
        let result = fetch_text("http://127.0.0.1:1/").await;
        assert!(result.is_err(), "a refused connection must be an Err");
    }

    #[test]
    fn asset_names_match_the_release_matrix() {
        assert_eq!(
            release_asset_name("linux", "x86_64"),
            Some("joshify-linux-x86_64.tar.gz")
        );
        assert_eq!(
            release_asset_name("macos", "aarch64"),
            Some("joshify-macos-aarch64.tar.gz")
        );
        // No binaries are published for these yet (issue #33); update must say
        // so rather than downloading something that cannot run.
        assert_eq!(release_asset_name("linux", "aarch64"), None);
        assert_eq!(release_asset_name("macos", "x86_64"), None);
        assert_eq!(release_asset_name("windows", "x86_64"), None);
    }

    #[test]
    fn versions_compare_with_or_without_a_leading_v() {
        assert_eq!(normalize_version("v1.2.3"), "1.2.3");
        assert_eq!(normalize_version("1.2.3"), "1.2.3");
        assert!(!needs_update("0.7.7", "v0.7.7"), "must be idempotent");
        assert!(!needs_update("0.7.7", "0.7.7"));
        assert!(needs_update("0.7.7", "v0.7.8"));
        // A pinned older tag is a legitimate target.
        assert!(needs_update("0.7.7", "v0.7.6"));
    }

    #[test]
    fn latest_tag_is_read_from_the_github_payload() {
        let body = r#"{"tag_name":"v0.7.7","name":"Joshify v0.7.7","draft":false}"#;
        assert_eq!(parse_latest_tag(body), Some("v0.7.7".to_string()));
        assert_eq!(parse_latest_tag("not json"), None);
        assert_eq!(parse_latest_tag(r#"{"name":"no tag here"}"#), None);
    }

    #[test]
    fn digests_are_looked_up_by_asset_name() {
        let sums = "aaaa  joshify-linux-x86_64.tar.gz\nbbbb  joshify-macos-aarch64.tar.gz\n";
        assert_eq!(
            expected_digest(sums, "joshify-linux-x86_64.tar.gz"),
            Some("aaaa".to_string())
        );
        assert_eq!(
            expected_digest(sums, "joshify-macos-aarch64.tar.gz"),
            Some("bbbb".to_string())
        );
        // An asset that is not listed must not silently match another line.
        assert_eq!(expected_digest(sums, "joshify-windows.tar.gz"), None);
        assert_eq!(expected_digest("", "joshify-linux-x86_64.tar.gz"), None);
    }

    #[test]
    fn sha256_matches_a_known_vector() {
        // Standard test vector for the empty input.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_ne!(sha256_hex(b"a"), sha256_hex(b"b"));
    }

    #[test]
    fn uninstall_confirms_before_removing_anything() {
        // Removing the binary is destructive even without --purge.
        assert!(needs_confirmation(&UninstallOptions::default()));
        assert!(needs_confirmation(&UninstallOptions {
            purge: true,
            assume_yes: false
        }));
        // Only --yes skips the prompt.
        assert!(!needs_confirmation(&UninstallOptions {
            purge: false,
            assume_yes: true
        }));
        assert!(!needs_confirmation(&UninstallOptions {
            purge: true,
            assume_yes: true
        }));
    }

    #[test]
    fn smoke_test_prefix_matches_what_the_version_printer_emits() {
        // These drifting apart would make every update fail with "did not
        // report a version" instead of a real error.
        assert!(crate::version_line().starts_with(crate::VERSION_PREFIX));
    }

    #[test]
    fn uninstall_keeps_user_data_by_default() {
        let plan = plan_uninstall(
            Some(PathBuf::from("/usr/local/bin/joshify")),
            Path::new("/home/u/.config/joshify"),
            Path::new("/home/u/.cache/joshify"),
            &UninstallOptions::default(),
        );
        assert_eq!(plan.binary, Some(PathBuf::from("/usr/local/bin/joshify")));
        assert!(
            plan.data.is_empty(),
            "credentials must survive an uninstall unless --purge is given"
        );
        assert!(!plan.remove_keyring);
    }

    #[test]
    fn purge_removes_config_cache_and_keyring() {
        let options = UninstallOptions {
            purge: true,
            assume_yes: true,
        };
        let plan = plan_uninstall(
            None,
            Path::new("/home/u/.config/joshify"),
            Path::new("/home/u/.cache/joshify"),
            &options,
        );
        assert_eq!(
            plan.data,
            vec![
                PathBuf::from("/home/u/.config/joshify"),
                PathBuf::from("/home/u/.cache/joshify"),
            ]
        );
        assert!(plan.remove_keyring);
    }

    #[test]
    fn atomic_replace_swaps_contents_and_leaves_no_staging_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("joshify");
        std::fs::write(&target, b"old").expect("seed");

        atomic_replace(&target, b"new").expect("replace");

        assert_eq!(std::fs::read(&target).expect("read"), b"new");
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with('.'))
            .collect();
        assert!(
            leftovers.is_empty(),
            "staging file left behind: {leftovers:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn staging_happens_next_to_the_destination_not_in_tmp() {
        // The bug: the update smoke-tested the new binary from
        // tempfile::tempdir() (i.e. /tmp). Where /tmp is mounted noexec -
        // common on hardened images - execve returns EACCES and the update
        // died with a bare "Permission denied" after a verified download.
        // The staged binary must live in the destination's own directory,
        // which is by definition allowed to execute.
        // Pure path math against a realistic install location, so the check
        // is about where staging goes rather than about the test's fixture.
        let dest = Path::new("/usr/local/bin/joshify");

        let staged = staging_path_for(dest).expect("staging path");

        assert_eq!(
            staged,
            Path::new("/usr/local/bin/.joshify.new"),
            "staged binary must sit beside the destination"
        );
        assert_eq!(staged.parent(), dest.parent());
        assert_ne!(staged, dest, "staging must not clobber the running binary");
    }

    #[cfg(unix)]
    #[test]
    fn a_staged_binary_can_actually_be_executed() {
        // The real property the fix is about: after staging, the file runs.
        // Asserting mode bits alone would not have caught the original bug,
        // because the mode bits were always right.
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("joshify");

        let staged = stage_beside(&dest, b"#!/bin/sh\necho 'Joshify 9.9.9'\n").expect("stage");

        let mode = std::fs::metadata(&staged)
            .expect("stat")
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111, "staged binary must be executable");

        let out = std::process::Command::new(&staged)
            .arg("--version")
            .output()
            .expect("the staged binary must be executable in place");
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).starts_with("Joshify "));
    }

    #[test]
    fn commit_staged_moves_the_staged_binary_into_place() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("joshify");
        std::fs::write(&dest, b"old").expect("seed");

        let staged = stage_beside(&dest, b"new").expect("stage");
        assert_eq!(std::fs::read(&dest).expect("read"), b"old", "not yet live");

        commit_staged(&staged, &dest).expect("commit");
        assert_eq!(std::fs::read(&dest).expect("read"), b"new");
        assert!(!staged.exists(), "staging file must not survive the commit");
    }

    #[test]
    fn a_failed_smoke_test_leaves_the_running_binary_untouched() {
        // Staging beside the destination puts a file next to the live binary;
        // if the new one turns out to be broken, that file must not be left
        // behind and the running binary must be exactly as it was.
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("joshify");
        std::fs::write(&dest, b"old").expect("seed");

        let staged = stage_beside(&dest, b"broken").expect("stage");
        // Simulate the update bailing out after a failed probe.
        std::fs::remove_file(&staged).expect("cleanup");

        assert_eq!(std::fs::read(&dest).expect("read"), b"old");
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with('.'))
            .collect();
        assert!(
            leftovers.is_empty(),
            "staging file left behind: {leftovers:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_makes_the_binary_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("joshify");
        atomic_replace(&target, b"#!/bin/sh\n").expect("replace");
        let mode = std::fs::metadata(&target)
            .expect("stat")
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111, "installed binary must be executable");
    }
}
