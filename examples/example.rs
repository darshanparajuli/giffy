use image::bmp::BMPEncoder;
use image::ColorType;
use std::env;
use std::fs::File;
use std::io;
use std::path::Path;

fn main() -> Result<(), io::Error> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Expected 2 arguments: <GIF file> <output folder>"),
        ));
    }

    let input_path = Path::new(&args[0]);
    let mut output_path = Path::new(&args[1]).to_path_buf();
    let mut file = File::open(&input_path)?;

    match giffy::load(&mut file) {
        Ok(gif) => {
            println!("Frame count: {}", gif.image_frames.len());

            let mut counter = 1;
            for frame in gif.image_frames {
                let file_name = format!(
                    "{}-frame-{}.bmp",
                    input_path.file_name().unwrap().to_str().unwrap(),
                    counter
                );
                output_path.push(&file_name);

                let mut file = File::create(&output_path)?;
                let mut encoder = BMPEncoder::new(&mut file);

                println!("Writing frame #{} to '{}'", counter, file_name);

                let mut colors = vec![];
                for c in frame.colors.iter() {
                    let values: [u8; 3] = (*c).into();
                    colors.extend(&values);
                }

                encoder.encode(&colors, gif.width, gif.height, ColorType::RGB(8))?;

                output_path.pop();
                counter += 1;
            }
        }

        Err(e) => println!("{}", e),
    }

    Ok(())
}
