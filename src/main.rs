extern crate giffy;

fn main() {
    match giffy::load("test.gif") {
        Ok(_) => {}
        Err(e) => println!("{}", e),
    }
}
