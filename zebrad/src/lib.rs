//! Zebrad
//!
//! Zebrad

#![deny(
    warnings,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
#![forbid(unsafe_code)]

use app_dirs::AppInfo;

const APP_INFO: AppInfo = AppInfo {
    name: "zebrad",
    author: "Zcash Foundation <zebra@zfnd.org>",
};

pub mod application;
pub mod commands;
pub mod config;
pub mod error;
pub mod prelude;
