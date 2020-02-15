#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use serde::{Deserialize, Serialize};
use serde_json::Result;
use serde_json::*;
use std::io::Cursor;

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
struct Something {
    first: u64,
    second: f64,
}
fn main() {
    let mut vec_data = Vec::<u8>::new();
    for i in 0..255 {
        vec_data.push(i as u8);
    }
    // &vec_data[8..10];
    println!("Hello, world!");
    let s = Something { first: 0, second: 0.0 };
    let buf = &mut vec_data[0..29];
    let mut c = Cursor::new(buf);
    let res = to_writer(&mut c, &s).unwrap();
    dbg!(res);
    dbg!(c.position());
}
