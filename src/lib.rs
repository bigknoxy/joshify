pub mod album_art;
pub mod api;
pub mod auth;
pub mod config;
pub mod connect;
pub mod keyring_store;
pub mod player;
pub mod session;
pub mod setup;
pub mod state;
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
}

impl CliArgs {
    pub fn parse() -> Self {
        let mut args = CliArgs::default();
        let mut i = 1;
        let cli_args: Vec<String> = std::env::args().collect();

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
                _ => {
                    i += 1;
                }
            }
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
