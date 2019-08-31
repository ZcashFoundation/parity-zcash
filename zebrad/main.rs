#[macro_use]
extern crate clap;
#[macro_use]
extern crate tracing;
extern crate tracing_fmt;
extern crate app_dirs;
extern crate libc;

extern crate zebra_chain;
extern crate zebra_db;
extern crate zebra_import;
extern crate zebra_keys;
extern crate zebra_message;
extern crate zebra_network;
extern crate zebra_p2p;
extern crate zebra_primitives;
extern crate zebra_rpc;
extern crate zebra_script;
extern crate zebra_storage;
extern crate zebra_sync;
extern crate zebra_verification;

mod commands;
mod config;
mod rpc;
mod rpc_apis;
mod seednodes;
mod util;

use app_dirs::AppInfo;

pub const APP_INFO: AppInfo = AppInfo {
    name: "zebra",
    author: "Zcash Foundation",
};
pub const PROTOCOL_VERSION: u32 = 70_014;
pub const PROTOCOL_MINIMUM: u32 = 70_001;
pub const ZCASH_PROTOCOL_VERSION: u32 = 170_007;
pub const ZCASH_PROTOCOL_MINIMUM: u32 = 170_007;
pub const USER_AGENT: &'static str = "zebra";
pub const REGTEST_USER_AGENT: &'static str = "/Satoshi:0.12.1/";

fn main() {
    // Always print backtrace on panic.
    ::std::env::set_var("RUST_BACKTRACE", "1");

    if let Err(err) = run() {
        println!("{}", err);
    }
}

fn run() -> Result<(), String> {
    let yaml = load_yaml!("cli.yml");
    let matches = clap::App::from_yaml(yaml).get_matches();
    let cfg = try!(config::parse(&matches));

    // Initialize a tracing subscriber to print tracing events
    let subscriber = tracing_fmt::FmtSubscriber::builder()
        .with_ansi(true)
        .with_filter(tracing_fmt::filter::EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|_| "Could not initialize tracing subscriber")?;

    match matches.subcommand() {
        ("import", Some(import_matches)) => commands::import(cfg, import_matches),
        ("rollback", Some(rollback_matches)) => commands::rollback(cfg, rollback_matches),
        _ => commands::start(cfg),
    }
}
