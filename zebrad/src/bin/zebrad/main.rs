//! Main entry point for Zebrad

use zebrad::application::APPLICATION;

/// Boot Zebrad
fn main() {
    abscissa::boot(&APPLICATION);
}