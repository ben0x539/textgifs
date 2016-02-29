extern crate gif;

use std::{process, env, io, path, fs};
use std::io::Write;

#[derive(Debug)]
enum Err {
    Io(io::Error),
    Gif(gif::DecodingError)
}

impl From<io::Error> for Err {
    fn from(e: io::Error) -> Err { Err::Io(e) }
}

impl From<gif::DecodingError> for Err {
    fn from(e: gif::DecodingError) -> Err { Err::Gif(e) }
}

fn main() {
    let mut exit_status = 0;
    for arg in env::args_os().skip(1) {
        let file: &path::Path = arg.as_ref();
        if let Err(e) = process(&file) {
            let _ = write!(io::stderr(), "file {}: {:?}", file.display(), e);
            exit_status = -1;
        }
    }

    process::exit(exit_status);
}

fn pick_color(palette: &[u8], color: usize) -> char {
    const SHITTY_PALETTE: [char; 5] = [' ', '░', '▒', '▓', '█'];
    let rgb = &palette[color * 3 .. color * 3 + 3];
    let level = get_level(rgb[0], rgb[1], rgb[2]);

    return SHITTY_PALETTE[level];

    fn get_level(r: u8, g: u8, b: u8) -> usize {
        let (r, g, b) = (r as u32, g as u32, b as u32);
        let abs = (r * r + g * g + b * b) as f64;
        let bound = 255.0 * 255.0 * 3.0 + 1.0;
        ((abs / bound).sqrt() * SHITTY_PALETTE.len() as f64) as usize
    }
}

fn copy(&gif::Frame { top, left, width, height, .. }: &gif::Frame,
        image_width: u16, from: &[char], to: &mut [char]) {
    for x in left .. left + width {
        for y in top .. top + height {
            let p = x as usize + y as usize * image_width as usize;
            to[p] = from[p];
        }
    }
}

fn blank(&gif::Frame { top, left, width, height, .. }: &gif::Frame,
         image_width: u16,
         buf: &mut [char]) {
    for x in left .. left + width {
        for y in top .. top + height {
            let p = x as usize + y as usize * image_width as usize;
            buf[p] = '▞';
        }
    }
}

fn show(width: u16, buf: &[char]) {
    let width = width as usize;
    for y in 0 .. buf.len() / width {
        for x in 0 .. width {
            print!("{}", buf[x + y * width]);
        }
        print!("\n");
    }
}

fn process(p: &path::Path) -> Result<(), Err> {
    let file = try!(fs::File::open(p));
    let mut decoder = try!(gif::Decoder::new(file).read_info());
    let global_bg = decoder.bg_color();
    let global_palette = decoder.global_palette().unwrap_or(&[]).to_owned();
    let width = decoder.width();
    let mut prev = vec!['▞'; (decoder.width() * decoder.height()) as usize];
    let mut curr = prev.clone();

    let mut n = 0;
    while let Some(frame) = try!(decoder.read_next_frame()) {
        let palette = frame.palette.as_ref().unwrap_or(&global_palette);
        let bg = frame.transparent.map(|bg| bg as usize).or(global_bg);
        for frame_x in 0 .. frame.width as usize {
            let image_x = frame_x + frame.left as usize;
            for frame_y in 0 .. frame.height as usize {
                let frame_i = frame_x + frame_y * frame.width as usize;
                let image_y = frame_y + frame.top as usize;
                let image_i = image_x + image_y * width as usize;

                let index = frame.buffer[frame_i] as usize;
                if Some(index) != bg {
                    curr[image_i] = pick_color(palette, index);
                }
            }
        }

        println!("frame {}", n);
        show(width, &curr);
        println!("");
        n += 1;

        use gif::DisposalMethod::*;
        match frame.dispose {
            Any | Keep => { copy(frame, width, &curr, &mut prev); }
            Previous => { copy(frame, width, &prev, &mut curr); }
            Background => { blank(frame, width, &mut curr); }
        }
    }

    Ok(())
}
