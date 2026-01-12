#![no_std]
#![no_main]

use ecos_ssc1::{ecos_main, println};

#[ecos_main]
fn main() -> ! {
    loop {
        if '\n' as u8 == ecos_ssc1::Uart::read_byte_blocking() {
            break;
        }
    }
    println!("Hello ECOS from Rust Project [{{project_name}}]!!!");
    loop {}
}
