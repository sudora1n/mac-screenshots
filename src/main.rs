mod draw;

use crate::draw::frame;
use clap::Parser;
use image::{DynamicImage, ImageFormat, RgbaImage};
use std::io::{self, Cursor, Read, Write};

#[derive(Parser)]
#[command(
    name = "macshot",
    version,
    about = "png processor",
    group(
        clap::ArgGroup::new("mode")
        .required(true)
        .multiple(false)
        .args(&["std", "input"])
    )
)]
struct Args {
    #[arg(short, long, requires = "output")]
    input: Option<String>,
    #[arg(short, long, requires = "input")]
    output: Option<String>,

    #[arg(long, default_value_t = 1.0)]
    scale: f32,
    #[arg(long)]
    title: Option<String>,

    #[arg(long)]
    std: bool,
}

fn read_stdin_image() -> image::ImageResult<DynamicImage> {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;
    image::load_from_memory(&buf)
}

fn write_image_to_stdout(img: &RgbaImage) {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Png).unwrap();

    let mut stdout = std::io::stdout();
    stdout.write_all(buf.get_ref()).unwrap();
    stdout.flush().unwrap();
}

fn main() {
    let args = Args::parse();
    let mut img: RgbaImage;
    if args.std {
        img = read_stdin_image()
            .expect("failed to read PNG from stdin")
            .to_rgba8();

        let framed = frame(&mut img, args.scale, args.title);

        write_image_to_stdout(&framed);
    } else {
        img = image::open(args.input.unwrap())
            .expect("failed to open input")
            .to_rgba8();

        let framed = frame(&mut img, args.scale, args.title);

        framed
            .save(args.output.unwrap())
            .expect("failed to save output");
    }
}
