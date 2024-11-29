use std::{fs, iter::repeat, path::PathBuf};

use {clap::Parser, easy_signed_distance_field as sdf};

#[derive(Parser)]
struct Args {
    font: PathBuf,
    out: PathBuf,
}

fn main() {
    let args = Args::parse();

    let font_data = fs::read(args.font).expect("Font file should be readable");
    let font = sdf::Font::from_bytes(font_data.as_slice(), sdf::FontSettings::default())
        .expect("Font file should be parsable");

    let rasters = (0..26)
        .map(|offset| {
            let c = (b'a' + offset) as char;
            generate_glyph(&font, 32., c)
        })
        .collect::<Vec<sdf::SdfRaster>>();

    let atlas = build_atlas(rasters, 7, 4);

    let ascii = atlas
        .buffer
        .chunks(atlas.width as usize)
        .map(|row| row.iter().map(|c| lut(*c)).collect::<String>())
        .collect::<Vec<String>>()
        .join("\n");

    std::fs::write(args.out, ascii).unwrap();
}

fn build_atlas(rasters: Vec<sdf::SdfRaster>, width: usize, height: usize) -> sdf::SdfRaster {
    assert!(width * height >= rasters.len());

    let (cell_width, cell_height) = (rasters[0].width as usize, rasters[0].height as usize);
    let cell_size = cell_width * cell_height;

    let mut atlas = sdf::SdfRaster {
        width: (width * cell_width) as u32,
        height: (height * cell_height) as u32,
        buffer: vec![0.; cell_size * (width * height) as usize],
    };

    let buf = atlas.buffer.as_mut_slice();

    for i in 0..height.min(rasters.len() / width + rasters.len() % width) {
        let cell_row_length = if rasters.len() - i * width < width {
            rasters.len() - i * width
        } else {
            width
        };

        for j in dbg!(0..cell_row_length) {
            println!("Handling '{}'", (b'a' + (i * width + j) as u8) as char);

            let id = i * width + j;
            let start = i * width * cell_size + j * cell_width;

            for row in 0..cell_height as usize {
                let buf_row_start = start + row * width * cell_width;
                let buf_row_end = buf_row_start + cell_width;

                let row_start = row * cell_width;
                let row_end = row_start + cell_width;

                buf[buf_row_start..buf_row_end]
                    .copy_from_slice(&rasters[id].buffer[row_start..row_end]);
            }
        }
    }

    atlas
}

fn generate_glyph(font: &sdf::Font, px: f32, c: char) -> sdf::SdfRaster {
    let padding = 2;
    let spread = 6.;

    let (mut _metrics, mut glyph) = font
        .sdf_generate(px, padding, spread, c)
        .expect("SDF should be generated");

    let mut adjusted_px = px;
    while glyph.width as f32 > px || glyph.height as f32 > px {
        adjusted_px -= 1.;

        (_metrics, glyph) = font
            .sdf_generate(adjusted_px, padding, spread, c)
            .expect("SDF should be generated");
    }

    let glyph = square_glyph(glyph, px as u32);

    // #[cfg(debug_assertions)]
    // {
    //     println!("'{c}': ({}, {})", glyph.width, glyph.height);
    //
    //     let ascii = ascii_art(&glyph);
    //     let lid = repeat('-')
    //         .take(glyph.width as usize + 4)
    //         .collect::<String>();
    //
    //     println!("{lid}\n{ascii}\n{lid}");
    // }

    glyph
}

fn ascii_art(glyph: &sdf::SdfRaster) -> String {
    glyph
        .buffer
        .chunks(glyph.width as usize)
        .map(|row| row.iter().map(|c| lut(*c)).collect::<String>())
        .collect::<Vec<String>>()
        .join("\n")
}

fn square_glyph(mut glyph: sdf::SdfRaster, target: u32) -> sdf::SdfRaster {
    assert!(glyph.width <= target);
    assert!(glyph.height <= target);

    let hpad = (target - glyph.width) as usize;
    let lpad = hpad / 2;
    let rpad = lpad + hpad % 2;

    let inner = glyph
        .buffer
        .chunks(glyph.width as usize)
        .flat_map(|chunk| {
            repeat(&0.)
                .take(lpad)
                .chain(chunk)
                .chain(repeat(&0.).take(rpad))
        })
        .collect::<Vec<_>>();

    let vpad = (target - glyph.height) as usize;
    let tpad = vpad / 2;
    let bpad = tpad + vpad % 2;

    let tlid = repeat(&0f32).take(tpad * target as usize);
    let blid = repeat(&0f32).take(bpad * target as usize);

    let buf = tlid
        .chain(inner.into_iter())
        .chain(blid)
        .map(|f| *f)
        .collect::<Vec<_>>();

    glyph.buffer = buf;
    glyph.width = target;
    glyph.height = target;

    glyph
}

fn lut(value: f32) -> char {
    assert!(value >= 0. && value <= 1.);

    b" .:-+=+*#%@"[(value * 10.) as usize] as char
}
