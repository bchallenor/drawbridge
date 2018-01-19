#[macro_use]
extern crate error_chain;
extern crate ipnet;

mod cloud;
mod dns;
mod errors;
mod iprules;

use errors::*;

quick_main!(run);

fn run() -> Result<()> {
    println!("Hello, world!");
    Ok(())
}
