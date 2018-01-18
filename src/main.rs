#[macro_use]
extern crate error_chain;

mod errors;

use errors::*;

quick_main!(run);

fn run() -> Result<()> {
    println!("Hello, world!");
    Ok(())
}
