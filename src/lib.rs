/// Prefix of the first line of `joshify --version`.
///
/// `joshify update` smoke-tests a downloaded binary against this, so the
/// printer and the check must not be able to drift apart.
pub const VERSION_PREFIX: &str = "Joshify ";

/// The exact first line of `joshify --version`.
pub fn version_line() -> String {
    format!("{VERSION_PREFIX}{}", env!("CARGO_PKG_VERSION"))
}

pub mod album_art;
pub mod api;
pub mod auth;
pub mod cli;
pub mod config;
pub mod connect;
pub mod keyring_store;
pub mod librespot_auth;
pub mod logging;
pub mod lyrics;
pub mod manage;
pub mod playback;
pub mod playback_keys;
pub mod player;
pub mod search;
pub mod session;
pub mod setup;
pub mod state;
pub mod themes;
pub mod ui;

/// CLI arguments for non-interactive mode
#[derive(Debug, Clone, Default)]
pub struct CliArgs {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub redirect_uri: Option<String>,
    pub help: bool,
    pub test_search: bool,
    /// Run credential setup and OAuth, then exit without starting the TUI.
    pub setup: bool,
    /// Print the version and exit.
    pub version: bool,
    /// A self-management subcommand, if one was given.
    pub command: Option<Subcommand>,
}

/// Subcommands that do not need a Spotify session.
///
/// Note the playback subcommands in `src/cli.rs` are deliberately NOT wired up:
/// they are stubs that print fabricated data (`cmd_status` returns
/// "Test Track"), so making them reachable would advertise functionality that
/// does not exist. See #48 and #23.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subcommand {
    Update(crate::manage::UpdateOptions),
    Uninstall(crate::manage::UninstallOptions),
}

impl CliArgs {
    pub fn parse() -> Self {
        let mut args = CliArgs::default();
        let cli_args: Vec<String> = std::env::args().collect();

        if let Some(name) = cli_args.get(1) {
            let rest = &cli_args[2..];
            let has = |flag: &str| rest.iter().any(|a| a == flag);
            let is_subcommand = matches!(name.as_str(), "update" | "uninstall");

            // `joshify update --help` must still print help rather than being
            // swallowed by the subcommand branch.
            if is_subcommand && (has("--help") || has("-h")) {
                args.help = true;
                return args;
            }

            match name.as_str() {
                "update" => {
                    // A bare `--version` with nothing after it is a mistake, not
                    // a request for the latest release - say so instead of
                    // quietly doing something else.
                    let pinned = rest.iter().position(|a| a == "--version").map(|i| {
                        rest.get(i + 1).cloned().unwrap_or_else(|| {
                            eprintln!(
                                "error: --version needs a release tag, e.g. --version v0.7.7"
                            );
                            std::process::exit(2);
                        })
                    });

                    args.command = Some(Subcommand::Update(crate::manage::UpdateOptions {
                        check_only: has("--check"),
                        version: pinned,
                    }));
                    return args;
                }
                "uninstall" => {
                    args.command = Some(Subcommand::Uninstall(crate::manage::UninstallOptions {
                        // --keep-data is the explicit opposite of --purge, for
                        // scripts that want to state the default outright.
                        purge: has("--purge") && !has("--keep-data"),
                        assume_yes: has("--yes") || has("-y"),
                    }));
                    return args;
                }
                _ => {}
            }
        }

        let mut i = 1;

        while i < cli_args.len() {
            match cli_args[i].as_str() {
                "--client-id" => {
                    if i + 1 < cli_args.len() {
                        args.client_id = Some(cli_args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--client-secret" => {
                    if i + 1 < cli_args.len() {
                        args.client_secret = Some(cli_args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--access-token" => {
                    if i + 1 < cli_args.len() {
                        args.access_token = Some(cli_args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--refresh-token" => {
                    if i + 1 < cli_args.len() {
                        args.refresh_token = Some(cli_args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--redirect-uri" => {
                    if i + 1 < cli_args.len() {
                        args.redirect_uri = Some(cli_args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--help" | "-h" => {
                    args.help = true;
                    i += 1;
                }
                "--test-search" => {
                    args.test_search = true;
                    i += 1;
                }
                "--setup" => {
                    args.setup = true;
                    i += 1;
                }
                "--version" | "-V" => {
                    args.version = true;
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }

        args
    }

    /// Print the version and nothing else.
    ///
    /// Kept trivially parseable: install.sh reads the last whitespace-separated
    /// field of the first line to decide whether an install is already current.
    pub fn print_version() {
        println!("{}", crate::version_line());
    }

    pub fn print_help() {
        println!("Joshify - Terminal Spotify Client");
        println!();
        println!("USAGE:");
        println!("    joshify [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("    --client-id <ID>       Spotify Client ID (or SPOTIFY_CLIENT_ID)");
        println!("    --client-secret <SEC>  Spotify Client Secret (or SPOTIFY_CLIENT_SECRET)");
        println!("    --access-token <TOK>   Spotify Access Token (or SPOTIFY_ACCESS_TOKEN)");
        println!("    --refresh-token <TOK>  Spotify Refresh Token (or SPOTIFY_REFRESH_TOKEN)");
        println!("    --redirect-uri <URI>   OAuth Redirect URI (default: http://127.0.0.1:8888/callback)");
        println!("    --setup                Run credential setup and OAuth, then exit");
        println!("    --version, -V          Print the version and exit");
        println!("    --test-search          Test search API and exit");
        println!("    --help, -h             Show this help message");
        println!();
        println!("COMMANDS:");
        println!("    update                 Update to the latest release (no-op if current)");
        println!("      --check              Report whether an update exists, change nothing");
        println!("      --version <TAG>      Install a specific release instead of the latest");
        println!("    uninstall              Remove joshify, keeping config and cache");
        println!("      --purge              Also delete config, credentials and cache");
        println!("      --keep-data          Explicitly keep them (the default)");
        println!("      --yes, -y            Do not prompt before deleting data");
        println!();
        println!("ENVIRONMENT VARIABLES:");
        println!("    SPOTIFY_CLIENT_ID      Spotify Client ID");
        println!("    SPOTIFY_CLIENT_SECRET  Spotify Client Secret");
        println!("    SPOTIFY_ACCESS_TOKEN   Spotify Access Token");
        println!("    SPOTIFY_REFRESH_TOKEN  Spotify Refresh Token");
        println!("    SPOTIFY_TOKEN_EXPIRES_AT  Token expiry as a Unix timestamp");
        println!("    SPOTIFY_REDIRECT_URI   OAuth Redirect URI");
        println!();
        println!("    SPOTIFY_CLIENT_ID, SPOTIFY_CLIENT_SECRET and one of");
        println!("    SPOTIFY_ACCESS_TOKEN / SPOTIFY_REFRESH_TOKEN must ALL be set");
        println!("    together to skip the browser. Setting only some of them still");
        println!("    opens a browser and waits for the callback.");
        println!();
        println!("EXAMPLES:");
        println!("    # Interactive mode (default)");
        println!("    joshify");
        println!();
        println!("    # Non-interactive with environment variables");
        println!("    export SPOTIFY_CLIENT_ID=xxx");
        println!("    export SPOTIFY_CLIENT_SECRET=yyy");
        println!("    export SPOTIFY_ACCESS_TOKEN=zzz");
        println!("    joshify");
        println!();
        println!("    # Non-interactive with CLI flags");
        println!("    joshify --client-id xxx --access-token zzz");
    }
}

#[cfg(test)]
mod subcommand_tests {

    /// Regression for #48/#54: a subcommand that is parsed but never dispatched
    /// is worse than one that does not exist. `--version` shipped broken for
    /// four releases exactly this way, so assert the wiring, not just the enum.
    #[test]
    fn main_dispatches_every_subcommand() {
        let main_src = include_str!("main.rs");
        // Match the full call expression: a bare name prefix would also match a
        // renamed-away function such as run_update_UNWIRED.
        for handler in [
            "manage::run_update(&options)",
            "manage::run_uninstall(&options)",
        ] {
            assert!(
                main_src.contains(handler),
                "{handler} is unreachable from main; parsing it is not enough (#48)"
            );
        }
    }

    /// The stub playback commands in src/cli.rs must stay unreachable: they
    /// print fabricated data (cmd_status returns "Test Track"), so wiring them
    /// up would advertise functionality that does not exist (#48, #23).
    #[test]
    fn stub_playback_commands_are_not_wired_up() {
        let main_src = include_str!("main.rs");
        assert!(
            !main_src.contains("CliHandler"),
            "src/cli.rs is a stub; do not make it reachable until it is real (#48)"
        );
    }

    #[test]
    fn help_documents_the_subcommands() {
        let lib_src = include_str!("lib.rs");
        for text in ["    update ", "    uninstall ", "--purge", "--keep-data"] {
            assert!(lib_src.contains(text), "help text should mention {text}");
        }
    }
}
