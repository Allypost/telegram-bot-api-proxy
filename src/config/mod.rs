use std::{path::PathBuf, process::exit};

use clap::{ArgAction, Parser};
use hyper::{http::uri::Scheme, Uri};
use lazy_static::lazy_static;
use resolve_path::PathResolveExt;

lazy_static! {
    pub static ref CONFIG: Config = Config::new();
}

#[derive(Debug, Clone)]
pub struct Config {
    pub base_path: PathBuf,
    pub proxy_to: Uri,

    pub log_level: String,

    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn new() -> Self {
        let args = Args::parse();

        let base_folder = match args.base_folder.try_resolve() {
            Ok(path) => path,
            Err(e) => {
                println!("Failed to resolve folder {:?}: {}", args.base_folder, e);
                exit(1);
            }
        };
        let base_path = match PathBuf::from(base_folder.clone()).canonicalize() {
            Ok(path) => path,
            Err(e) => {
                println!("Failed to canonicalize folder {:?}: {}", &base_folder, e);
                exit(1);
            }
        };

        let proxy_to = match args.proxy_to.parse::<Uri>() {
            Ok(uri) => uri,
            Err(e) => {
                println!("Failed to parse proxy_to url {:?}: {}", args.proxy_to, e);
                exit(1);
            }
        };

        match proxy_to.scheme() {
            Some(s) if s == &Scheme::HTTP => {}
            Some(s) => {
                let fixed_url = format!(
                    "http://{authority}{path}",
                    authority = proxy_to
                        .authority()
                        .map(std::string::ToString::to_string)
                        .unwrap_or_default(),
                    path = proxy_to.path()
                );
                println!(
                    "Proxy URL MUST be a HTTP URL!\n{s} is unsupported.\nTry providing `{fixed_url}`\nGot: {:?}",
                    args.proxy_to
                );
                exit(1);
            }
            None => {
                println!("Proxy URL MUST be a HTTP URL!\nTry providing `http://{proxy_to}' instead\nGot: {proxy_to}");
                exit(1);
            }
        }
        let proxy_to = format!(
            "http://{authority}",
            authority = proxy_to
                .authority()
                .map(std::string::ToString::to_string)
                .unwrap_or_default(),
        )
        .parse::<Uri>()
        .expect("Failed to parse proxy_to url");

        Self {
            proxy_to,
            base_path,
            log_level: args.log_level,
            host: args.host,
            port: args.port,
        }
    }
}

#[derive(Debug, Clone, Parser)]
#[clap(disable_help_flag = true)]
/// Proxy for the telegram bot api
///
/// This proxy is used to make a local mode telegram bot api
/// respond to file requests the same way the official api does.
/// It returns the `/file/bot<token>/<file_id>` handler and
/// sets a relative path to the file in the `/bot<token>/GetFile` response.
struct Args {
    /// The URL of the telegram bot api to proxy to.
    ///
    /// MUST be a valid HTTP URL!
    /// Example: http://api.telegram.org
    #[clap(short = 't', long, env = "PROXY_TO")]
    proxy_to: String,

    /// Folder where the telegram-bot-api is storing the files
    #[clap(
        short,
        long,
        default_value = "/var/lib/telegram-bot-api",
        env = "BASE_FOLDER"
    )]
    base_folder: PathBuf,

    /// Log level for the application
    #[clap(
        long,
        default_value = "warn,telegram_bot_api_proxy=info",
        env = "RUST_LOG"
    )]
    log_level: String,

    /// Port to listen on
    #[clap(short, long, default_value = "3000", env = "PORT")]
    port: u16,
    /// Host to listen on
    #[clap(short = 'h', long, default_value = "localhost", env = "HOST")]
    host: String,

    /// Print help (this message)
    #[clap(action = ArgAction::Help, long)]
    help: Option<bool>,
}
