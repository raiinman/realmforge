use std::io::{self, Read};

use realmforge_gate_independent::hash_native_password;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut password = String::new();
    io::stdin().read_to_string(&mut password)?;
    while password.ends_with(['\n', '\r']) {
        password.pop();
    }
    let hash = hash_native_password(&password)?;
    println!("{hash}");
    Ok(())
}
