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

fn main() {
    // settings
    let seed = rng::Seed::new();
    let width = 1 << 6;

    let args = terrain_generator::Args {
        seed,
        width,
        continent_count: 6,
        continental_mountain_thickness: width / 2,
        fractal_main_layer: 2,
        fractal_weight: 0.25,
        erosion_kind: terrain_generator::ErosionKind::Rng,
        erosion_iterations: width * width * 6,
        erosion_normalize_mod: width * width * 6,
        erosion_max_lifetime: 20,              // 10
        erosion_start_speed: 1.0,              // 1.0
        erosion_start_water: 2.0,              // 1.0
        erosion_inertia: 0.3,                  // 0.3
        erosion_min_sediment_capacity: 0.01,   // 0.01
        erosion_sediment_capacity_factor: 5.0, // 3.0
        erosion_erode_speed: 0.004,            // 0.004
        erosion_deposit_speed: 0.004,          // 0.003
        erosion_gravity: 8.0,                  // 4.0
        erosion_evaporate_speed: 0.01,         // 0.01
    };

    // run terrain generator
    let result = terrain_generator::run(args);

    // use heightmap as desired
    if let Err(e) = save_as_bin(width, &result) {
        eprintln!("failed to safe bin: {}", e);
    }

    if let Err(e) = save_as_qoi(width, &result) {
        eprintln!("failed to safe qoi: {}", e);
    }

    eprintln!("done! seed: {:?}", seed);
}

fn save_as_bin<'a>(
    width: usize,
    height_maps: impl IntoIterator<Item = &'a crate::terrain_generator::HeightMap>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;

    use crate::terrain_generator::HeightMap;

    for (i, height_map) in height_maps.into_iter().enumerate() {
        let HeightMap { values, side } = height_map;

        eprintln!("serializing bin... {}/6", i + 1);

        let path_string = format!("{}x{}_f32_{}.bin", width, width, side);
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
    use std::path::PathBuf;

    use crate::color::ByteColor;
    use crate::color::Gradient;
    use crate::color::OkLab;
    use crate::color::Rgb;
    use crate::qoi::Channels;
    use crate::qoi::ColorSpace;
    use crate::qoi::QoiDesc;
    use crate::terrain_generator::HeightMap;

    let gradient = Gradient::try_from([
        OkLab::from(Rgb::from_hex("#00008a")?),
        OkLab::from(Rgb::from_hex("#1d90ff")?),
        OkLab::from(Rgb::from_hex("#04e100")?),
        OkLab::from(Rgb::from_hex("#ffff00")?),
        OkLab::from(Rgb::from_hex("#ff8b00")?),
        OkLab::from(Rgb::from_hex("#ff0300")?),
        OkLab::from(Rgb::from_hex("#a64020")?),
    ])?;

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
