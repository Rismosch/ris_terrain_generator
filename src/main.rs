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
    let width = (1 << 6) + 1;
    let args = terrain_generator::Args {
        only_generate_first_face: false,
        seed: rng::Seed::default(),
        width,
        continent_count: 6,
        kernel_radius: width as f32 * 0.75,
        fractal_main_layer: 1,
        fractal_weight: 0.25,
    };
    let result = terrain_generator::run(args);
    save_as_qoi(width, result)
}

fn save_as_qoi(
    width: usize,
    height_maps: impl IntoIterator<Item = crate::terrain_generator::HeightMap>
) -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;

    use crate::color::ByteColor;
    use crate::color::Gradient;
    use crate::color::OkLab;
    use crate::color::Rgb;
    use crate::terrain_generator::HeightMap;
    use crate::qoi::Channels;
    use crate::qoi::ColorSpace;
    use crate::qoi::QoiDesc;

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
        let HeightMap {
            values,
            side,
        } = height_map;

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

        eprintln!(
            "bytes len: {} qoi len: {}",
            bytes.len(),
            qoi_bytes.len(),
        );

        eprintln!("serializing...");

        let path_string = format!("height_map_{}.qoi", side);
        let filepath = PathBuf::from(path_string);

        if filepath.exists() {
            std::fs::remove_file(&filepath)?;
        }

        let mut file = std::fs::File::create_new(filepath)?;
        let f = &mut file;
        crate::io::write(f, &qoi_bytes)?;
    } // end qoi

    eprintln!("done!");
    Ok(())
}

