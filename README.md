# ris_terrain_generator

This terrain generator generates heightmaps for a planet.

## How to run

    git clone https://github.com/Rismosch/ris_terrain_generator.git
    cd ris_terrain_generator
    cargo run -r

## How to use

The main and only entry point is `terrain_generator::run`. It produces the 6 square faces of a cube. These then can be mapped to sphere, thus producing planetary terrain.

`terrain_generator::Args` exposes a few settings to modify the generators behaviour:

- `only_generate_first_face` is for debugging purposes. When tweaking the generation algorithm, which may provide poor performance, it may be helpful to only generate the first face to safe time.
- `seed` is a wrapper around a `u128`, which controls the RNG of the generator. The same seed will produce the same terrain. Use `Seed::new()` for new unpredictable terrain. Use `Seed::default()` or `Seed(your_number)` for expected terrain.
- `width` is the width of a single face. This has no effect on the overall structure of the terrain. It only affects the resolution. Note that the bigger the width, the longer the generator takes. For example my target width is `(1 << 12) + 1`, which takes several minutes to procude.
- `continent_count` determines how many continents should be generated. For more information see [How it works](#How-it-works).
- `kernel_radius` has an effect on the width of mountain ranges. A higher radius produces thicker mountains, but massively increases generation time.
- `fractal_main_layer` describes which layer of the fractal perlin noise is the main one. Every other will be weighted less than the main layer.
- `fractal_weight` determines the weight of the fractal perlin noise in comparison to the continental terrain generation.

The resulting heightmap is normalized. This means all values will be between 0 and 1. This makes it easy to transform the resulting `Vec<f32>` to transform into any format you desire. `save_as_qoi` in `main.rs` is an example, which demonstrates how to convert these values into [qoi images](https://qoiformat.org/) and save them to the root of this repository.

To find `i` to index a pixel in the heightmap, use the following formula:

    i = x + y * width

where `x` and `y` are your coordinates, and `width` is the width you provided in `terrain_generator::Args`.

## How it works

The terrain is generated in 3 distinc steps:

- continent generation
- fractal perlin noise
- hydraulic erosion

Continent generation produces continents. It picks random points on the surface of the cube. One point for each continent. Then it grows these continents using a Breadth-first search until the whole cube is covered.

After the continents have been generated, a convolution with a kernel over the entire cube is performed to find the continental boundaries and the nearest touching continent of each point. Each continent is assigned a rotation axis. To get the moving direction of any given point, it is rotated by a small angle around it's continents rotation axis. Then, depending whether neighboring continents collide or diverge, the pixel is raised or lowered.

Continent generation has the highest priority in the heightmaps. It generates highly coarse terrain, but doesn't create fine details, especially at the continent centers. So in the next step [fractal perlin noise](https://en.wikipedia.org/wiki/Perlin_noise) is used to generate noise over the entire cube surface. The perlin noise algorithm was modified to perfectly tile the cube.

While the previous steps already produce quite nice looking terrain, it can still be improved. Especially the continent generation produces very coars terrain with harsh edges, which need to be smoothed out. Hydraulic erosion takes care of that. It simulates rain falling on the terrain, which then flows downhill. The water carries sediment with it and deposits it somewhere else, thus modifying the terrain.

Multiple times between and after these steps, normalization is performed.

## Notes

The code started out as a script in [ris_engine](https://github.com/Rismosch/ris_engine).

The code quality of `terrain_generator.rs` is poor and not performant, as it went through many prototypes with little regard of long term maintenance. Most code was hapharzidly copied from [ris_engine](https://github.com/Rismosch/ris_engine) and modified, such that it compiles as a standalone project.

The idea is to run this generator once, and then use the static asset in the engine. Thus, the poor quality is of no concern, as the intention of this code is to be run once and then never again.

Have fun!
