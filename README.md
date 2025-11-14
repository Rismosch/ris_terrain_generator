# ris_terrain_generator

This repo generates heightmaps for a planet.

## Dependencies

To compile and run this repo, you require a working Rust compiler: https://www.rust-lang.org/tools/install

## How to run

    git clone https://github.com/Rismosch/ris_terrain_generator.git
    cd ris_terrain_generator
    cargo run -r

Passing the `-r` flag to `cargo run` is heavily recommended. At sufficiently large sizes, it will improve performance drastically.

## How to use

The main and only entry point is `terrain_generator::run`. It produces the 6 square faces of a cube. These can then be mapped to a sphere, thus producing planetary terrain.

`terrain_generator::run` takes an `terrain_generator::Args` struct as a parameter, which exposes many settings to adjust the generated terrain. For a quick overview, the two most important settings are `terrain_generator::Args::seed` and `terrain_generator::Args::width`. A given seed will always generate the same terrain; useful for testing different settings on the terrain-structure. The width determines how wide the side of a single cube face will be. For more details, see the doc comments of `terrain_generator::Args`.

⚠ ⚠ ⚠  
**Note that large widths may take very long to generate**!  
⚠ ⚠ ⚠

`terrain_generator::run` returns a `Vec` of the generated sides. These resulting heightmaps are normalized, meaning all values will be between 0 and 1. This makes it easy to transform them into any format you desire. As an example, `save_as_bin`, `save_as_qoi` and `save_as_qoi_preview` in `main.rs` demonstrate how one might use these heightmaps.

⚠ ⚠ ⚠  
**Note that the examples save files at the root of this repo! Existing files will be overwritten! Make sure you create backups of the generated files you want to keep!**  
⚠ ⚠ ⚠

To find `i` to index a pixel in a heightmap, use the following formula:

    i = x + y * width

where the coordinates `x` and `y` are smaller than `width`, and `width` is the width you provided in `terrain_generator::Args`.

As for coordinate system and orientation, the origin `(0, 0)` of each face is in the upper left corner. `+x` is facing right, and `+y` is facing down. The faces are arranged like this:

![cube net](cube_net.png)

where

- L => left
- B => back
- R => right
- F => front
- U => up/top
- D => down/bottom

## How it works

The terrain is generated in 3 distinct steps:

- continent generation
- fractal perlin noise
- hydraulic erosion

[Continent](https://en.wikipedia.org/wiki/Plate_tectonics) generation produces continents. It picks random points on the surface of the cube. One point for each continent. Then it grows these continents using a randomized [breadth-first search](https://en.wikipedia.org/wiki/Breadth-first_search) until the whole cube is covered.

After the continents have been generated, the boundaries between the continets are found, and another breadth-first search starting from the edges assignes each pixel its nearest neighboring continent. Each continent is assigned a random [rotation axis](https://en.wikipedia.org/wiki/Axis%E2%80%93angle_representation). To simulate [continental drift](https://en.wikipedia.org/wiki/Continental_drift), the rotation axis is used to find the direction, in which each pixel is moving in. Then, depending whether neighboring continents [collide or diverge](https://en.wikipedia.org/wiki/Plate_tectonics#Types_of_plate_boundaries), the pixel is raised or lowered.

Continent generation generates highly coarse terrain, but doesn't create fine details, especially at continent centers. So in the next step [fractal perlin noise](https://en.wikipedia.org/wiki/Perlin_noise) is used to generate noise over the entire cube surface.

While the previous steps already produce quite nice looking terrain, it can still be improved.

First, between generation steps, the heightmaps are [normalized](https://en.wikipedia.org/wiki/Normalization_(statistics)) between 0 and 1. Second, a [weighting function](https://en.wikipedia.org/wiki/Weight_function) is applied, making some heights more likely than others.

Then, [hydraulic erosion](https://en.wikipedia.org/wiki/Hydraulic_action) is applied. This combines the previous steps and forms the terrain in a much more natural way: Rain is simulated, by placing waterdroplets randomly on the surface of the terrain. The water then flows downhill, carrying sediment with it and depositing it somewhere else.

The erosion simulator logic was directly taken from [Sebastian Lague](https://youtu.be/eaXk97ujbPQ) (precicely [this](https://github.com/SebLague/Hydraulic-Erosion/blob/f245576d204978e3186f41c8abbd75c326c6857e/Assets/Scripts/ComputeShaders/Erosion.compute) code), rewritten in Rust and heavily modified to work on a cube.

Since the 6 faces tile the cube, great care must be taken at the edges of each face. The randomized breadth-first search, convolution and erosion take this into account, as they walk over the cube surface. The erosion in particular required the kernel and the direction to be rotated, aswell as sampling over the edges. The perlin noise had to be modified such that it's lattice directions are continuous over the edges. But due to the [Hairy ball theorem](https://en.wikipedia.org/wiki/Hairy_ball_theorem), the 8 corners produce no directions for the perlin noise, or in other words, directions with 0 length.

A finished heightmap, with a colored gradient applied, may look like this:

![example](example.png)

_The image above can be generated by passing `terrain_generator::Args::default()` to `terrain_generator::run`_

## Notes

The code started out as a script in [ris_engine](https://github.com/Rismosch/ris_engine). Most code in this repo was copied from my engine and modified, such that it compiles as a standalone project. Due to that, and the fact that `terrain_generator.rs` went through many prototypes very very quickly, the code is in comparably poorer quality.

However, the idea is to run this generator once, and then use the result as a static asset. Thus, the comparably poorer code quality and performance isn't of great significance, because it will never be run again.

Anyway, have fun!
