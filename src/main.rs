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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // settings
    let width = (1 << 6) + 1;
    let args = terrain_generator::Args {
        only_generate_first_face: false,
        seed: rng::Seed::default(),
        width,
        continent_count: 6,
        kernel_radius: width as f32 * 0.75,
        fractal_main_layer: 1,
        fractal_weight: 0.25,
        erosion_brush_radius: 3,
        erosion_iterations: 1,
        erosion_max_lifetime: 30,
        erosion_start_speed: 1.0,
        erosion_start_water: 1.0,
        erosion_inertia: 0.3,
        erosion_min_sediment_capacity: 0.01,
        erosion_sediment_capacity_factor: 3.0,
        erosion_erode_speed: 0.3,
        erosion_deposit_speed: 0.3,
        erosion_gravity: 4.0,
        erosion_evaporate_speed: 0.01,
    };
    
    // run terrain generator
    let result = terrain_generator::run(args);

    // use heightmap as desired
    save_as_qoi(width, result)
}

fn save_as_qoi(
    width: usize,
    height_maps: impl IntoIterator<Item = crate::terrain_generator::HeightMap>,
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

    for height_map in height_maps.into_iter() {
        let HeightMap { values, side } = height_map;

        eprintln!("convert height map to bytes...");
        let mut bytes = Vec::with_capacity(values.len() * 3);

        for &h in values.iter() {
            let lab = gradient.sample(h);
            let rgb = Rgb::from(lab);
            let [r, g, b] = rgb.to_u8();
            bytes.push(r);
            bytes.push(g);
            bytes.push(b);
        }

        eprintln!("encoding to qoi...");
        let desc = QoiDesc {
            width: width as u32,
            height: width as u32,
            channels: Channels::RGB,
            color_space: ColorSpace::Linear,
        };
        let qoi_bytes = qoi::encode(&bytes, desc)?;

        eprintln!("bytes len: {} qoi len: {}", bytes.len(), qoi_bytes.len(),);

        eprintln!("serializing...");

        let path_string = format!("height_map_{}.qoi", side);
        let filepath = PathBuf::from(path_string);

        if filepath.exists() {
            std::fs::remove_file(&filepath)?;
        }

        let mut file = std::fs::File::create_new(filepath)?;
        let f = &mut file;
        crate::io::write(f, &qoi_bytes)?;
    }

    eprintln!("done!");
    Ok(())
}
