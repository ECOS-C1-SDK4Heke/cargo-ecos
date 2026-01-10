#![no_std]
#![no_main]

use ecos_ssc1::{ecos_main, println};

#[ecos_main]
fn main() -> ! {
    println!("Hello ECOS from Rust Project [{{project_name}}]!!!");
    loop {}
}
