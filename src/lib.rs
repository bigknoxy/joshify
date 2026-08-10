pub mod album_art;
pub mod api;
pub mod auth;
pub mod cli;
pub mod config;
pub mod connect;
pub mod daemon;
pub mod keyring_store;
pub mod logging;
pub mod lyrics;
pub mod media_control;
pub mod notifications;
pub mod playback;
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
    /// Positional CLI subcommand (e.g. `play`, `pause`, `status`) and its args
    pub command: Option<Vec<String>>,
}

impl CliArgs {
    pub fn parse() -> Self {
        let cli_args: Vec<String> = std::env::args().collect();
        Self::parse_from(&cli_args)
    }

    /// Parse CLI arguments from an argv slice (argv[0] is the program name).
    fn parse_from(cli_args: &[String]) -> Self {
        let mut args = CliArgs::default();
        let mut i = 1;
        let mut positional: Vec<String> = Vec::new();

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
                "--format" | "-f" | "--limit" | "-l" => {
                    // CLI subcommand flags that take a value - keep the flag
                    // and its value in the positional command so parse_args
                    // can process them (e.g. `status --format json`).
                    positional.push(cli_args[i].clone());
                    if i + 1 < cli_args.len() {
                        positional.push(cli_args[i + 1].clone());
                    }
                    i += 2;
                }
                arg if arg.starts_with('-') => {
                    // Unknown flag - skip it and its value
                    i += 1;
                }
                arg => {
                    // Positional argument (CLI subcommand)
                    positional.push(arg.to_string());
                    i += 1;
                }
            }
        }

        if !positional.is_empty() {
            args.command = Some(positional);
        }

        args
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
        println!("    --test-search          Test search API and exit");
        println!("    --help, -h             Show this help message");
        println!();
        println!("ENVIRONMENT VARIABLES:");
        println!("    SPOTIFY_CLIENT_ID      Spotify Client ID");
        println!("    SPOTIFY_CLIENT_SECRET  Spotify Client Secret");
        println!("    SPOTIFY_ACCESS_TOKEN   Spotify Access Token");
        println!("    SPOTIFY_REFRESH_TOKEN  Spotify Refresh Token");
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
mod tests {
    use super::*;
    use crate::cli::{parse_args, CliCommand, OutputFormat};

    /// Parse a command line (excluding argv[0]) into a CliCommand via the
    /// full CliArgs::parse() -> parse_args() pipeline.
    fn parse_cli(args: &[&str]) -> CliCommand {
        let mut argv = vec!["joshify".to_string()];
        argv.extend(args.iter().map(|s| s.to_string()));
        let cli_args = CliArgs::parse_from(&argv);
        let command = cli_args.command.expect("expected a CLI subcommand");
        parse_args(&command).expect("command should parse")
    }

    #[test]
    fn test_cli_status_format_json_flag_value_not_leaked() {
        let cmd = parse_cli(&["status", "--format", "json"]);
        assert_eq!(
            cmd,
            CliCommand::Status {
                format: OutputFormat::Json
            }
        );
    }

    #[test]
    fn test_cli_search_limit_flag_value_not_leaked() {
        let cmd = parse_cli(&["search", "foo", "--limit", "5"]);
        assert_eq!(
            cmd,
            CliCommand::Search {
                query: "foo".to_string(),
                limit: 5
            }
        );
    }

    #[test]
    fn test_cli_play_uri_positional() {
        let cmd = parse_cli(&["play", "spotify:track:abc"]);
        assert_eq!(
            cmd,
            CliCommand::Play {
                uri: Some("spotify:track:abc".to_string())
            }
        );
    }
}
