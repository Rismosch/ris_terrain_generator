mod color;
mod common;
mod io;
mod matrix;
mod pcg;
mod qoi;
mod quaternion;
mod rng;
mod terrain_generator;
mod util;
mod vector;

use std::path::PathBuf;

use crate::color::ByteColor;
use crate::color::Gradient;
use crate::color::OkLab;
use crate::color::Rgb;
use crate::qoi::Channels;
use crate::qoi::ColorSpace;
use crate::qoi::QoiDesc;
use crate::rng::Seed;
use crate::terrain_generator::Args;
use crate::terrain_generator::ErosionKind;
use crate::terrain_generator::HeightMap;
use crate::terrain_generator::Side;

fn main() {
    // settings
    let seed = Seed::default();
    let width = 1 << 8;
    let preview_width = width;

    let args = Args {
        seed,
        width,
        continent_count: 6,
        continental_mountain_thickness: width / 2,
        fractal_main_layer: 2,
        fractal_weight: 0.25,
        erosion_kind: ErosionKind::Rng,
        erosion_iterations: width * width * 6,
        erosion_normalize_mod: width * width * 6,
        erosion_max_lifetime: 20,
        erosion_start_speed: 1.0,
        erosion_start_water: 2.0,
        erosion_inertia: 0.3,
        erosion_min_sediment_capacity: 0.01,
        erosion_sediment_capacity_factor: 5.0,
        erosion_erode_speed: 0.004,
        erosion_deposit_speed: 0.004,
        erosion_gravity: 8.0,
        erosion_evaporate_speed: 0.01,
    };

    // run terrain generator
    let result = terrain_generator::run(args);

    // use heightmap as desired
    if let Err(e) = save_as_bin(&result) {
        eprintln!("failed to safe bin: {}", e);
    }

    if let Err(e) = save_as_qoi(width, &result) {
        eprintln!("failed to safe qoi: {}", e);
    }

    if let Err(e) = save_as_qoi_preview(width, preview_width, &result) {
        eprintln!("failed to safe qoi preview: {}", e);
    }

    eprintln!("done! seed: {:?}", seed);
}

fn save_as_bin<'a>(
    height_maps: impl IntoIterator<Item = &'a HeightMap>,
) -> Result<(), Box<dyn std::error::Error>> {
    for (i, height_map) in height_maps.into_iter().enumerate() {
        let HeightMap { values, side } = height_map;
        eprintln!("serializing bin... {}/6", i + 1);

        let path_string = format!("height_map_{}.bin", side);
        let filepath = PathBuf::from(path_string);

        if filepath.exists() {
            std::fs::remove_file(&filepath)?;
        }

        let mut file = std::fs::File::create_new(filepath)?;
        let f = &mut file;
        for v in values {
            crate::io::write_f32(f, *v)?;
        }
    }

    Ok(())
}

fn save_as_qoi<'a>(
    width: usize,
    height_maps: impl IntoIterator<Item = &'a crate::terrain_generator::HeightMap>,
) -> Result<(), Box<dyn std::error::Error>> {
    let gradient = colored_height_gradient()?;

    for (i, height_map) in height_maps.into_iter().enumerate() {
        let HeightMap { values, side } = height_map;
        eprintln!("serializing qoi... {}/6", i + 1);

        let mut bytes = Vec::with_capacity(values.len() * 3);

        for &h in values.iter() {
            let lab = gradient.sample(h);
            let rgb = Rgb::from(lab);
            let [r, g, b] = rgb.to_u8();
            bytes.push(r);
            bytes.push(g);
            bytes.push(b);
        }

        let desc = QoiDesc {
            width: width as u32,
            height: width as u32,
            channels: Channels::RGB,
            color_space: ColorSpace::SRGB,
        };
        let qoi_bytes = qoi::encode(&bytes, desc)?;

        let path_string = format!("height_map_{}.qoi", side);
        let filepath = PathBuf::from(path_string);

        if filepath.exists() {
            std::fs::remove_file(&filepath)?;
        }

        let mut file = std::fs::File::create_new(filepath)?;
        let f = &mut file;
        crate::io::write(f, &qoi_bytes)?;
    }

    Ok(())
}

#[derive(Debug)]
struct StringError(String);

impl std::fmt::Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StringError {}

fn save_as_qoi_preview<'a>(
    width: usize,
    preview_width: usize,
    height_maps: impl IntoIterator<Item = &'a crate::terrain_generator::HeightMap>,
) -> Result<(), Box<dyn std::error::Error>> {
    let preview_width = usize::min(width, preview_width);

    if width % preview_width != 0 {
        Err(StringError(format!(
            "preview_width {} must be a divisor of width {} for the downsampling to work correctly",
            preview_width,
            width,
        )))?;
    }

    let kernel_width = width / preview_width;
    let kernel_len = kernel_width * kernel_width;

    let gradient = colored_height_gradient()?;
    let desc = QoiDesc {
        width: preview_width as u32 * 4,
        height: preview_width as u32 * 3,
        channels: Channels::RGB,
        color_space: ColorSpace::SRGB,
    };
    let data_len = (desc.width * desc.height * 3) as usize;
    let mut data = vec![u8::MAX; data_len];

    for (i, height_map) in height_maps.into_iter().enumerate() {
        let HeightMap { values, side } = height_map;
        eprintln!("serializing preview... {}/6", i + 1);

        for iy in 0..preview_width {
            for ix in 0..preview_width {
                let mut sum = 0.0;
                for iy_ in 0..kernel_width {
                    for ix_ in 0..kernel_width {
                        let ix_ = ix * kernel_width + ix_;
                        let iy_ = iy * kernel_width + iy_;
                        let i = iy_ * width + ix_;
                        let h = values[i];
                        sum += h;
                    }
                }

                let h = sum / kernel_len as f32;
                let lab = gradient.sample(h);
                let rgb = Rgb::from(lab);
                let [r, g, b] = rgb.to_u8();

                let (offset_x, offset_y) = match side {
                    Side::L => (0, preview_width),
                    Side::B => (preview_width, preview_width),
                    Side::R => (2 * preview_width, preview_width),
                    Side::F => (3 * preview_width, preview_width),
                    Side::U => (preview_width, 0),
                    Side::D => (preview_width, 2 * preview_width),
                };

                let ix_ = ix + offset_x;
                let iy_ = iy + offset_y;
                let i = iy_ * desc.width as usize + ix_;

                data[i * 3] = r;
                data[i * 3 + 1] = g;
                data[i * 3 + 2] = b;
            }
        }
    }

    let qoi_bytes = qoi::encode(&data, desc)?;

    let filepath = PathBuf::from("preview.qoi");
    if filepath.exists() {
        std::fs::remove_file(&filepath)?;
    }

    let mut file = std::fs::File::create_new(filepath)?;
    let f = &mut file;
    crate::io::write(f, &qoi_bytes)?;

    Ok(())
}

fn colored_height_gradient() -> Result<Gradient<OkLab, 3>, Box<dyn std::error::Error>>{
    let gradient = Gradient::try_from([
        OkLab::from(Rgb::from_hex("#00008a")?),
        OkLab::from(Rgb::from_hex("#1d90ff")?),
        OkLab::from(Rgb::from_hex("#04e100")?),
        OkLab::from(Rgb::from_hex("#ffff00")?),
        OkLab::from(Rgb::from_hex("#ff8b00")?),
        OkLab::from(Rgb::from_hex("#ff0300")?),
        OkLab::from(Rgb::from_hex("#a64020")?),
    ])?;

    Ok(gradient)
}
