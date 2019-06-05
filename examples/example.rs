use image::bmp::BMPEncoder;
use image::ColorType;
use std::env;
use std::fs::File;
use std::path::Path;

fn main() -> Result<(), std::io::Error> {
    for a in env::args().skip(1) {
        let path = Path::new(&a);
        let mut file = File::open(&path)?;
        match giffy::load(&mut file) {
            Ok(gif) => {
                println!("Frame count: {}", gif.image_frames.len());
                let mut counter = 1;
                for frame in gif.image_frames {
                    let file_name = format!(
                        "test_frames/{}-frame-{}.bmp",
                        path.file_name().unwrap().to_str().unwrap(),
                        counter
                    );
                    let mut file = File::create(&file_name)?;
                    let mut encoder = BMPEncoder::new(&mut file);

                    println!("Writing frame #{} to '{}'", counter, file_name);
                    let mut colors = vec![];
                    for c in frame.color_values.iter() {
                        colors.push(c.r());
                        colors.push(c.g());
                        colors.push(c.b());
                    }
                    encoder.encode(&colors, gif.width, gif.height, ColorType::RGB(8))?;

                    counter += 1;
                }
            }
            Err(e) => println!("{}", e),
        }
    }

    Ok(())
}
