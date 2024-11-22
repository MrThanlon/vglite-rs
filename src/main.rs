use std::{io::Write, thread::sleep};

use vglite_rs::{Context, Buffer, Color, Format, Rectangle};

fn main() {
    println!("Hello, world!");
    let ctx = Context::new(640, 480).unwrap();
    let mut buffer = Buffer::allocate(640, 480, Format::RGBA8888).unwrap();
    buffer.clear(None, Color { r: 0, g: 0, b: 0, a: 255 }).unwrap();
    buffer.clear(Some(&mut Rectangle { x: 300, y: 370, width: 300, height: 100 }), Color { r: 255, g: 0, b: 0, a: 255 }).unwrap();
    ctx.finish().unwrap();
    println!("finish");
    // drop(buffer);
    // write buffer to file
    let mut file = std::fs::File::create("test.raw").unwrap();
    file.write_all(buffer.data()).unwrap();
    println!("write to file");
}
