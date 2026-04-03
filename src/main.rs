//! Claude Code CLI binary entry point

//! All modules are declared in src/lib.rs and accessible via `crate::`
//! from both the binary (main.rs) and the library (lib.rs) crates.

use open_cc::cli;

#[tokio::main]
async fn main() {
    if let Err(e) = cli::run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
