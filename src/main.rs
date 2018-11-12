extern crate giffy;

use std::env;

fn main() {
    for a in env::args().skip(1) {
        match giffy::load(&a) {
            Ok(_) => {}
            Err(e) => println!("{}", e),
        }
    }
}
