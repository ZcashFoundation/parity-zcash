//! Zebrad Subcommands
//!
//! This is where you specify the subcommands of your application.
//!
//! The default application comes with two subcommands:
//!
//! - `start`: launches the application
//! - `version`: print application version
//!
//! See the `impl Configurable` below for how to specify the path to the
//! application's configuration file.

mod start;
mod version;

use self::{start::StartCommand, version::VersionCommand};
use crate::config::ZebradConfig;
use abscissa::{Command, Configurable, Help, Options, Runnable};
use std::path::PathBuf;

/// Zebrad Subcommands
#[derive(Command, Debug, Options, Runnable)]
pub enum ZebradCommand {
    /// The `help` subcommand
    #[options(help = "get usage information")]
    Help(Help<Self>),

    /// The `start` subcommand
    #[options(help = "start the application")]
    Start(StartCommand),

    /// The `version` subcommand
    #[options(help = "display version information")]
    Version(VersionCommand),
}

/// This trait allows you to define how application configuration is loaded.
impl Configurable<ZebradConfig> for ZebradCommand {
    fn config_path(&self) -> Option<PathBuf> {
        // Have `config_path` return `Some(path)` in order to trigger the
        // application configuration being loaded.
        None
    }
}
