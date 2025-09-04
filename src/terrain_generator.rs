use core::fmt;
use std::cell::RefCell;
use std::f32::consts::PI;

use crate::matrix::Mat2;
use crate::quaternion::Quat;
use crate::rng::Rng;
use crate::rng::Seed;
use crate::vector::Vec2;
use crate::vector::Vec3;

/// defines the side of a cube:
///
///         ┌───┐
///         │ U │
///     ┌───┼───┼───┬───┐
///     │ L │ B │ R │ F │
///     └───┼───┼───┴───┘
///         │ D │
///         └───┘
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// left (-x)
    L,
    /// back (-y)
    B,
    /// right (+x)
    R,
    /// front (+y)
    F,
    /// up (+z)
    U,
    /// down (-z)
    D,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::L => write!(f, "l"),
            Side::B => write!(f, "b"),
            Side::R => write!(f, "r"),
            Side::F => write!(f, "f"),
            Side::U => write!(f, "u"),
            Side::D => write!(f, "d"),
        }
    }
}

pub struct Args {
    pub only_generate_first_face: bool,
    pub seed: Seed,
    pub width: usize,
    pub continent_count: usize,
    pub kernel_radius: f32,
    pub fractal_main_layer: usize,
    pub fractal_weight: f32,
    pub erosion_iterations: usize,
}

pub struct HeightMap {
    pub values: Vec<f32>,
    pub side: Side,
}

pub fn run(args: Args) -> Vec<HeightMap> {
    let Args {
        only_generate_first_face,
        seed,
        width,
        continent_count,
        kernel_radius,
        fractal_main_layer,
        fractal_weight,
        erosion_iterations,
    } = args;

    eprintln!("seed: {:?}", seed);
    let mut rng = Rng::new(seed);
    let kernel_radius = kernel_radius as isize;

    eprintln!("resolution: {}x{}", width, width);

    let mut sides = vec![
        ProtoSide {
            perlin_sampler: PerlinSampler {
                offset: (0, 0),
                edge0: None,
                edge1: None,
                edge2: None,
                edge3: None,
            },
            height_map: RefCell::new(ProtoHeightMap::new(Side::L, width)),
        },
        ProtoSide {
            perlin_sampler: PerlinSampler {
                offset: (1, 0),
                edge0: None,
                edge1: None,
                edge2: None,
                edge3: None,
            },
            height_map: RefCell::new(ProtoHeightMap::new(Side::B, width)),
        },
        ProtoSide {
            perlin_sampler: PerlinSampler {
                offset: (2, 0),
                edge0: None,
                edge1: None,
                edge2: None,
                edge3: None,
            },
            height_map: RefCell::new(ProtoHeightMap::new(Side::R, width)),
        },
        ProtoSide {
            perlin_sampler: PerlinSampler {
                offset: (3, 0),
                edge0: None,
                edge1: Some(Box::new(|iy, _| ((0, iy), Mat2::identity()))),
                edge2: None,
                edge3: None,
            },
            height_map: RefCell::new(ProtoHeightMap::new(Side::F, width)),
        },
        ProtoSide {
            perlin_sampler: PerlinSampler {
                offset: (1, -1),
                edge0: Some(Box::new(move |iy, _| {
                    ((iy, 0), Mat2(Vec2(0.0, 1.0), Vec2(-1.0, 0.0)))
                })),
                edge1: Some(Box::new(move |iy, (gw, _)| {
                    ((gw - iy + gw * 2, 0), Mat2(Vec2(0.0, -1.0), Vec2(1.0, 0.0)))
                })),
                edge2: Some(Box::new(move |ix, (gw, _)| {
                    (
                        (gw - ix + gw * 3, 0),
                        Mat2(Vec2(-1.0, 0.0), Vec2(0.0, -1.0)),
                    )
                })),
                edge3: None,
            },
            height_map: RefCell::new(ProtoHeightMap::new(Side::U, width)),
        },
        ProtoSide {
            perlin_sampler: PerlinSampler {
                offset: (1, 1),
                edge0: Some(Box::new(move |iy, (gw, gh)| {
                    ((gw - iy, gh), Mat2(Vec2(0.0, -1.0), Vec2(1.0, 0.0)))
                })),
                edge1: Some(Box::new(move |iy, (gw, gh)| {
                    ((iy + 2 * gw, gh), Mat2(Vec2(0.0, 1.0), Vec2(-1.0, 0.0)))
                })),
                edge2: None,
                edge3: Some(Box::new(move |ix, (gw, gh)| {
                    (
                        (gw - ix + gw * 3, gh),
                        Mat2(Vec2(-1.0, 0.0), Vec2(0.0, -1.0)),
                    )
                })),
            },
            height_map: RefCell::new(ProtoHeightMap::new(Side::D, width)),
        },
    ];

    let mut continents = vec![Continent::default(); continent_count];

    // continents
    eprintln!("determine continent starting positions...");
    let mut starting_positions = Vec::<ContinentPixel>::with_capacity(continent_count);

    for _ in 0..starting_positions.capacity() {
        loop {
            let side = rng.next_i32_between(0, 5) as usize;
            let min = 0;
            let max = width as i32 - 1;
            let ix = rng.next_i32_between(min, max) as usize;
            let iy = rng.next_i32_between(min, max) as usize;

            let candidate = ContinentPixel { side, ix, iy };

            let candidate_exists = starting_positions.iter().any(|x| *x == candidate);
            if candidate_exists {
                continue;
            }

            starting_positions.push(candidate);
            break;
        }
    }

    for (i, starting_position) in starting_positions.into_iter().enumerate() {
        let continent = &mut continents[i];
        continent.origin = starting_position.clone();
        continent.discovered_pixels.push(starting_position);
        continent.rotation_axis = rng.next_dir_3();
    }

    let mut discovered_pixel_count = 0;
    loop {
        // discover new pixels
        let mut new_pixel_was_discovered = false;

        for (continent_index, continent) in continents.iter_mut().enumerate() {
            let mut pixel = None;
            loop {
                if continent.discovered_pixels.is_empty() {
                    break;
                }

                let min = 0i32;
                let max = continent.discovered_pixels.len() as i32 - 1;
                let index = rng.next_i32_between(min, max) as usize;
                let candidate = continent.discovered_pixels.swap_remove(index);

                let side = &mut sides[candidate.side];
                let mut h = side.height_map.borrow().get(candidate.ix, candidate.iy);

                if h.continent_index != usize::MAX {
                    continue;
                }

                new_pixel_was_discovered = true;

                h.continent_index = continent_index;
                discovered_pixel_count += 1;

                if discovered_pixel_count % 1000000 == 0 {
                    let total = width * width * 6;
                    let percentage = discovered_pixel_count as f32 / total as f32;
                    eprintln!("generate continents... {}", percentage);
                }

                side.height_map
                    .borrow_mut()
                    .set(candidate.ix, candidate.iy, h);

                pixel = Some(candidate);
                break;
            }

            let Some(pixel) = pixel else {
                continue;
            };

            // walk left
            let new_pixel = if pixel.ix == 0 {
                match pixel.side {
                    // left -> front
                    0 => ContinentPixel {
                        side: 3,
                        ix: width - 1,
                        iy: pixel.iy,
                    },
                    // back -> left
                    1 => ContinentPixel {
                        side: 0,
                        ix: width - 1,
                        iy: pixel.iy,
                    },
                    // right -> back
                    2 => ContinentPixel {
                        side: 1,
                        ix: width - 1,
                        iy: pixel.iy,
                    },
                    // front -> right
                    3 => ContinentPixel {
                        side: 2,
                        ix: width - 1,
                        iy: pixel.iy,
                    },
                    // up -> left
                    4 => ContinentPixel {
                        side: 0,
                        ix: pixel.iy,
                        iy: 0,
                    },
                    // down -> left
                    5 => ContinentPixel {
                        side: 0,
                        ix: width - 1 - pixel.iy,
                        iy: width - 1,
                    },
                    _ => unreachable!(),
                }
            } else {
                ContinentPixel {
                    side: pixel.side,
                    ix: pixel.ix - 1,
                    iy: pixel.iy,
                }
            };
            continent.discovered_pixels.push(new_pixel);

            // walk right
            let new_pixel = if pixel.ix == width - 1 {
                match pixel.side {
                    // left -> back
                    0 => ContinentPixel {
                        side: 1,
                        ix: 0,
                        iy: pixel.iy,
                    },
                    // back -> right
                    1 => ContinentPixel {
                        side: 2,
                        ix: 0,
                        iy: pixel.iy,
                    },
                    // right -> front
                    2 => ContinentPixel {
                        side: 3,
                        ix: 0,
                        iy: pixel.iy,
                    },
                    // front -> left
                    3 => ContinentPixel {
                        side: 0,
                        ix: 0,
                        iy: pixel.iy,
                    },
                    // up -> right
                    4 => ContinentPixel {
                        side: 2,
                        ix: width - 1 - pixel.iy,
                        iy: 0,
                    },
                    // down -> right
                    5 => ContinentPixel {
                        side: 2,
                        ix: pixel.iy,
                        iy: width - 1,
                    },
                    _ => unreachable!(),
                }
            } else {
                ContinentPixel {
                    side: pixel.side,
                    ix: pixel.ix + 1,
                    iy: pixel.iy,
                }
            };
            continent.discovered_pixels.push(new_pixel);

            // walk up
            let new_pixel = if pixel.iy == 0 {
                match pixel.side {
                    // left -> up
                    0 => ContinentPixel {
                        side: 4,
                        ix: 0,
                        iy: pixel.ix,
                    },
                    // back -> up
                    1 => ContinentPixel {
                        side: 4,
                        ix: pixel.ix,
                        iy: width - 1,
                    },
                    // right -> up
                    2 => ContinentPixel {
                        side: 4,
                        ix: width - 1,
                        iy: width - 1 - pixel.ix,
                    },
                    // front -> up
                    3 => ContinentPixel {
                        side: 4,
                        ix: width - 1 - pixel.ix,
                        iy: 0,
                    },
                    // up -> front
                    4 => ContinentPixel {
                        side: 3,
                        ix: width - 1 - pixel.ix,
                        iy: 0,
                    },
                    // down -> back
                    5 => ContinentPixel {
                        side: 1,
                        ix: pixel.ix,
                        iy: width - 1,
                    },
                    _ => unreachable!(),
                }
            } else {
                ContinentPixel {
                    side: pixel.side,
                    ix: pixel.ix,
                    iy: pixel.iy - 1,
                }
            };
            continent.discovered_pixels.push(new_pixel);

            // walk down
            let new_pixel = if pixel.iy == width - 1 {
                match pixel.side {
                    // left -> down
                    0 => ContinentPixel {
                        side: 5,
                        ix: 0,
                        iy: width - 1 - pixel.ix,
                    },
                    // back -> down
                    1 => ContinentPixel {
                        side: 5,
                        ix: pixel.ix,
                        iy: 0,
                    },
                    // right -> down
                    2 => ContinentPixel {
                        side: 5,
                        ix: width - 1,
                        iy: pixel.ix,
                    },
                    // front -> down
                    3 => ContinentPixel {
                        side: 5,
                        ix: width - 1 - pixel.ix,
                        iy: width - 1,
                    },
                    // up -> back
                    4 => ContinentPixel {
                        side: 1,
                        ix: pixel.ix,
                        iy: 0,
                    },
                    // down -> front
                    5 => ContinentPixel {
                        side: 3,
                        ix: width - 1 - pixel.ix,
                        iy: 0,
                    },
                    _ => unreachable!(),
                }
            } else {
                ContinentPixel {
                    side: pixel.side,
                    ix: pixel.ix,
                    iy: pixel.iy + 1,
                }
            };
            continent.discovered_pixels.push(new_pixel);
        }

        if !new_pixel_was_discovered {
            break;
        }
    }

    // generate kernel
    eprintln!("generate kernel");
    let mut kernel = Vec::new();

    for iy in -kernel_radius..=kernel_radius {
        for ix in -kernel_radius..=kernel_radius {
            let x = ix as f32;
            let y = iy as f32;
            let d = f32::sqrt(x * x + y * y);
            if d < kernel_radius as f32 {
                kernel.push(((ix, iy), d));
            }
        }
    }

    kernel.sort_by(|l, r| l.1.total_cmp(&r.1));

    // calculate heights on continent boundaries
    eprintln!(
        "calculate height based on plate boundaries... {}",
        discovered_pixel_count
    );

    let mut min_continent = f32::MAX;
    let mut max_continent = f32::MIN;

    for side in sides.iter() {
        let ProtoSide {
            perlin_sampler: _,
            height_map,
        } = side;

        for iy in 0..width {
            if iy % 5 == 0 {
                eprintln!(
                    "finding plate boundaries {}... progress: {}/{}",
                    height_map.borrow().side,
                    iy,
                    width,
                );
            }

            for ix in 0..width {
                let h = height_map.borrow().get(ix, iy);
                let continent_index_lhs = h.continent_index;
                let continent = &continents[continent_index_lhs];

                for &((kx, ky), kd) in kernel.iter() {
                    if kx == 0 && ky == 0 {
                        continue;
                    }

                    let ix_ = ix as isize + kx;
                    let iy_ = iy as isize + ky;
                    let side_ = height_map.borrow().side;

                    let w = width as isize;

                    let falls_on_left = ix_ < 0;
                    let falls_on_right = ix_ >= w;
                    let falls_on_upper = iy_ < 0;
                    let falls_on_lower = iy_ >= w;

                    let falls_on_upper_left = falls_on_upper && falls_on_left;
                    let falls_on_upper_right = falls_on_upper && falls_on_right;
                    let falls_on_lower_left = falls_on_lower && falls_on_left;
                    let falls_on_lower_right = falls_on_lower && falls_on_right;

                    let falls_on_corner = falls_on_upper_left
                        || falls_on_upper_right
                        || falls_on_lower_left
                        || falls_on_lower_right;

                    if falls_on_corner {
                        continue;
                    }

                    // map ix_ and iy_ onto correct side
                    let (ix_, iy_, mapped_side_) = if falls_on_left {
                        let d = ix as isize + 1;
                        // kx is negative, negate to make math more intuitive
                        let kx = -kx;
                        match side_ {
                            Side::L => (w - 1 - kx + d, iy_, Side::F),
                            Side::B => (w - 1 - kx + d, iy_, Side::L),
                            Side::R => (w - 1 - kx + d, iy_, Side::B),
                            Side::F => (w - 1 - kx + d, iy_, Side::R),
                            Side::U => (iy_, kx - d, Side::L),
                            Side::D => (w - 1 - iy_, w - 1 - kx + d, Side::L),
                        }
                    } else if falls_on_right {
                        let d = width as isize - ix as isize;
                        match side_ {
                            Side::L => (kx - d, iy_, Side::B),
                            Side::B => (kx - d, iy_, Side::R),
                            Side::R => (kx - d, iy_, Side::F),
                            Side::F => (kx - d, iy_, Side::L),
                            Side::U => (w - 1 - iy_, kx - d, Side::R),
                            Side::D => (iy_, w - 1 - kx + d, Side::R),
                        }
                    } else if falls_on_upper {
                        let d = iy as isize + 1;
                        // ky is negative, negate to make math more intuitive
                        let ky = -ky;
                        match side_ {
                            Side::L => (ky - d, ix_, Side::U),
                            Side::B => (ix_, w - 1 - ky + d, Side::U),
                            Side::R => (w - 1 - ky + d, w - 1 - ix_, Side::U),
                            Side::F => (w - 1 - ix_, ky - d, Side::U),
                            Side::U => (w - 1 - ix_, ky - d, Side::F),
                            Side::D => (ix_, w - 1 - ky + d, Side::B),
                        }
                    } else if falls_on_lower {
                        let d = width as isize - iy as isize;
                        match side_ {
                            Side::L => (ky - d, w - 1 - ix_, Side::D),
                            Side::B => (ix_, ky - d, Side::D),
                            Side::R => (w - 1 - ky + d, ix_, Side::D),
                            Side::F => (w - 1 - ix_, w - 1 - ky + d, Side::D),
                            Side::U => (ix_, ky - d, Side::B),
                            Side::D => (w - 1 - ix_, w - 1 - ky + d, Side::F),
                        }
                    } else {
                        (ix_, iy_, side_)
                    };

                    let ix_ = ix_ as usize;
                    let iy_ = iy_ as usize;

                    let height_map_ = &sides
                        .iter()
                        .find(|x| x.height_map.borrow().side == mapped_side_)
                        .expect("height map to exist")
                        .height_map;
                    let Some(h_) = height_map_.borrow().try_get(ix_, iy_) else {
                        println!("after map 1: {} {} {} {}", width, side_, ix_, iy_);
                        println!("after map 2: {} {} {} {}", kx, ky, ix, iy);
                        panic!();
                    };

                    if h.continent_index == h_.continent_index {
                        continue;
                    }

                    // boundary found, calculate height
                    let continent_ = &continents[h_.continent_index];

                    let angle = 2.0 * PI / (4 * width) as f32;
                    let q = Quat::angle_axis(angle, continent.rotation_axis);
                    let q_ = Quat::angle_axis(angle, continent_.rotation_axis);

                    let p = position_on_sphere((ix, iy), width, height_map.borrow().side);
                    let p_ = position_on_sphere((ix_, iy_), width, height_map_.borrow().side);

                    let v = (q.rotate(p) - p).normalize();
                    let v_ = (q_.rotate(p_) - p_).normalize();

                    let origin_pixel = continent.origin.clone();
                    let origin_side = match origin_pixel.side {
                        0 => Side::L,
                        1 => Side::B,
                        2 => Side::R,
                        3 => Side::F,
                        4 => Side::U,
                        5 => Side::D,
                        _ => unreachable!(),
                    };

                    let o =
                        position_on_sphere((origin_pixel.ix, origin_pixel.iy), width, origin_side);
                    let d = p - o;
                    let d_ = p_ - o;

                    // formular for smoother, but in my opinion
                    // less interesting terrain:
                    // let m = (p * p_) / 2.0;
                    // let d = p - m;
                    // let d_ = m - p_;

                    let dot = Vec3::dot(v.normalize(), d.normalize());
                    let dot_ = Vec3::dot(v_.normalize(), d_.normalize());

                    let boundary_height = match (dot.is_sign_positive(), dot_.is_sign_positive()) {
                        (false, false) => dot * dot_,
                        (true, false) => dot * dot_ * -1.0,
                        (false, true) => dot * dot_,
                        (true, true) => dot * dot_,
                    };

                    // https://www.desmos.com/calculator/2oekg4vn5i
                    let a = (kd.abs() / kernel_radius as f32) - 1.0;
                    let weight = 1.0 - f32::sqrt(1.0 - a * a);

                    let mut h = height_map.borrow().get(ix, iy);
                    h.height += weight * boundary_height;
                    height_map.borrow_mut().set(ix, iy, h);

                    min_continent = f32::min(min_continent, h.height);
                    max_continent = f32::max(max_continent, h.height);

                    break;
                }
            }
        }

        if only_generate_first_face {
            break;
        }
    }

    eprintln!("continent min: {}, max: {}", min_continent, max_continent);

    // continents end
    normalize(&mut sides);

    // sides
    for (i, side) in sides.iter().enumerate() {
        let ProtoSide {
            perlin_sampler,
            height_map,
        } = side;

        eprintln!("generating side... {} ({})", height_map.borrow().side, i);

        let mut layer = 0;
        loop {
            let grid_width: i32 = 1 << (layer + 1);

            // https://www.desmos.com/calculator/xnwqm8vdez
            let a = 1.0;
            let b = fractal_main_layer as f32;
            let x = layer as f32;
            let grid_weight = fractal_weight / (f32::abs(a * x - a * b) + 1.0);

            layer += 1;

            if grid_width >= width as i32 {
                break;
            }

            for iy in 0..width {
                if iy % 100 == 0 {
                    eprintln!(
                        "generating side {} ({})... progress: {}/{} layer: {}",
                        height_map.borrow().side,
                        i,
                        iy,
                        width,
                        layer,
                    );
                }

                for ix in 0..width {
                    let coord = Vec2(ix as f32 + 0.5, iy as f32 + 0.5);
                    let heigh_map_width = height_map.borrow().width as f32;
                    let size = Vec2(heigh_map_width, heigh_map_width);
                    let normalized = coord / size;
                    let grid = Vec2(grid_width as f32, grid_width as f32);
                    let p = normalized * grid;

                    // this closure connects the edges and corners of different sizes, to
                    // ensure that the perlin noise ist continuous over the whole cube
                    let apply_net = |ix: i32, iy: i32| {
                        let offset_x = perlin_sampler.offset.0 * grid_width;
                        let offset_y = perlin_sampler.offset.1 * grid_width;
                        let default_x = ix + offset_x;
                        let default_y = iy + offset_y;
                        let default = ((default_x, default_y), Mat2::identity());

                        #[allow(clippy::if_same_then_else)]
                        // justification: makes things easier to reason about. each branch is an
                        // individual corner, edge or center pixel
                        if ix == 0 {
                            if iy == 0 {
                                ((default_x, default_y), Mat2::init(0.0))
                            } else if iy == grid_width {
                                ((default_x, default_y), Mat2::init(0.0))
                            } else {
                                perlin_sampler
                                    .edge0
                                    .as_ref()
                                    .map(|edge| edge(iy, (grid_width, grid_width)))
                                    .unwrap_or(default)
                            }
                        } else if ix == grid_width {
                            if iy == 0 {
                                ((default_x, default_y), Mat2::init(0.0))
                            } else if iy == grid_width {
                                ((default_x, default_y), Mat2::init(0.0))
                            } else {
                                perlin_sampler
                                    .edge1
                                    .as_ref()
                                    .map(|edge| edge(iy, (grid_width, grid_width)))
                                    .unwrap_or(default)
                            }
                        } else if iy == 0 {
                            perlin_sampler
                                .edge2
                                .as_ref()
                                .map(|edge| edge(ix, (grid_width, grid_width)))
                                .unwrap_or(default)
                        } else if iy == grid_width {
                            perlin_sampler
                                .edge3
                                .as_ref()
                                .map(|edge| edge(ix, (grid_width, grid_width)))
                                .unwrap_or(default)
                        } else {
                            default
                        }
                    };

                    // perlin noise
                    let m0 = p.x().floor() as i32;
                    let m1 = m0 + 1;
                    let n0 = p.y().floor() as i32;
                    let n1 = n0 + 1;

                    let (iq0, mat0) = apply_net(m0, n0);
                    let (iq1, mat1) = apply_net(m1, n0);
                    let (iq2, mat2) = apply_net(m0, n1);
                    let (iq3, mat3) = apply_net(m1, n1);
                    let g0 = mat0 * random_gradient(iq0.0, iq0.1, seed);
                    let g1 = mat1 * random_gradient(iq1.0, iq1.1, seed);
                    let g2 = mat2 * random_gradient(iq2.0, iq2.1, seed);
                    let g3 = mat3 * random_gradient(iq3.0, iq3.1, seed);

                    let q0 = Vec2(m0 as f32, n0 as f32);
                    let q1 = Vec2(m1 as f32, n0 as f32);
                    let q2 = Vec2(m0 as f32, n1 as f32);
                    let q3 = Vec2(m1 as f32, n1 as f32);

                    let s0 = g0.dot(p - q0);
                    let s1 = g1.dot(p - q1);
                    let s2 = g2.dot(p - q2);
                    let s3 = g3.dot(p - q3);

                    let h = |x: f32| (3.0 - x * 2.0) * x * x;
                    let Vec2(x, y) = p - q0;
                    let f0 = s0 * h(1.0 - x) + s1 * h(x);
                    let f1 = s2 * h(1.0 - x) + s3 * h(x);
                    let f = f0 * h(1.0 - y) + f1 * h(y);
                    // perlin noise end

                    let mut h = height_map.borrow().get(ix, iy);
                    h.height += f * grid_weight;
                    height_map.borrow_mut().set(ix, iy, h);
                }
            }
        }

        if only_generate_first_face {
            break;
        }
    } // end sides

    // normalize and apply weight to heightmap
    eprintln!("normalize and apply weight...");
    normalize(&mut sides);

    for side in sides.iter_mut() {
        for h in side.height_map.borrow_mut().values.iter_mut() {
            //// sigmoid
            //let steepness = 10.0;
            //let center = 0.5;
            //*h = 1.0 / (1.0 + f32::exp(-steepness * (*h - center)));

            // https://www.desmos.com/calculator/9qm31r4kfd
            let inverse_smoothstep = 0.5 - f32::sin(f32::asin(1.0 - 2.0 * h.height) / 3.0);
            let power = h.height * h.height;
            let weight = 1.0 - h.height;
            h.height = crate::common::mix(inverse_smoothstep, power, weight);
        }
    }

    normalize(&mut sides);

    // erosion
    // todo

    // prepare result
    let mut result = Vec::new();
    for side in sides.into_iter() {
        let height_map = side.height_map.borrow();

        let values = height_map
            .values
            .iter()
            .map(|x| x.height)
            .collect::<Vec<_>>();
        let side = height_map.side;

        let height_map = HeightMap { values, side };

        result.push(height_map);
    }

    result
}

#[derive(Clone, Copy)]
struct ProtoHeightMapValue {
    height: f32,
    continent_index: usize,
}

struct ProtoHeightMap {
    side: Side,
    values: Vec<ProtoHeightMapValue>,
    width: usize,
}

impl ProtoHeightMap {
    fn new(side: Side, width: usize) -> Self {
        let value = ProtoHeightMapValue {
            height: 0.0,
            continent_index: usize::MAX,
        };

        Self {
            side,
            values: vec![value; width * width],
            width,
        }
    }

    fn get(&self, x: usize, y: usize) -> ProtoHeightMapValue {
        let i = self.index(x, y);
        self.values[i]
    }

    fn try_get(&self, x: usize, y: usize) -> Option<ProtoHeightMapValue> {
        let i = x.checked_add(y.checked_mul(self.width)?)?;
        self.values.get(i).cloned()
    }

    fn set(&mut self, x: usize, y: usize, value: ProtoHeightMapValue) {
        let i = self.index(x, y);
        self.values[i] = value;
    }

    fn index(&self, x: usize, y: usize) -> usize {
        x + y * self.width
    }
}

struct ProtoSide {
    perlin_sampler: PerlinSampler,
    height_map: RefCell<ProtoHeightMap>,
}

type PerlinSamplerCallback = Box<dyn Fn(i32, (i32, i32)) -> ((i32, i32), Mat2)>;

struct PerlinSampler {
    offset: (i32, i32),
    edge0: Option<PerlinSamplerCallback>,
    edge1: Option<PerlinSamplerCallback>,
    edge2: Option<PerlinSamplerCallback>,
    edge3: Option<PerlinSamplerCallback>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
struct ContinentPixel {
    side: usize,
    ix: usize,
    iy: usize,
}

#[derive(Default, Debug, Clone)]
struct Continent {
    origin: ContinentPixel,
    discovered_pixels: Vec<ContinentPixel>,
    rotation_axis: Vec3,
}

fn position_on_sphere(texture_coordinate: (usize, usize), width: usize, side: Side) -> Vec3 {
    let (ix, iy) = texture_coordinate;

    // normalize texture coordinates
    let x = 2.0 * (ix as f32 / width as f32) - 1.0;
    let y = 2.0 * (iy as f32 / width as f32) - 1.0;

    // get position on cube
    let v = match side {
        Side::L => Vec3(-1.0, -x, -y),
        Side::B => Vec3(x, -1.0, -y),
        Side::R => Vec3(1.0, x, -y),
        Side::F => Vec3(-x, 1.0, -y),
        Side::U => Vec3(x, -y, 1.0),
        Side::D => Vec3(x, y, -1.0),
    };

    // normalize to get position on sphere
    let Vec3(x, y, z) = v;
    let x2 = x * x;
    let y2 = y * y;
    let z2 = z * z;
    let sx = x * f32::sqrt(1.0 - y2 / 2.0 - z2 / 2.0 + y2 * z2 / 3.0);
    let sy = y * f32::sqrt(1.0 - x2 / 2.0 - z2 / 2.0 + x2 * z2 / 3.0);
    let sz = z * f32::sqrt(1.0 - x2 / 2.0 - y2 / 2.0 + x2 * y2 / 3.0);

    Vec3(sx, sy, sz)
}

fn random_gradient(ix: i32, iy: i32, seed: Seed) -> Vec2 {
    let Seed(seed_value) = seed;
    let seed_a = seed_value & 0xFFFFFFFF;
    let seed_b = (seed_value >> 32) & 0xFFFFFFFF;

    let w = (8 * std::mem::size_of::<u32>()) as u32;
    let s = w / 2;
    let a = (ix as u32) ^ (seed_a as u32);
    let b = (iy as u32) ^ (seed_b as u32);
    let a = a.wrapping_mul(3284157443);
    let b = b ^ ((a << s) | (a >> (w - s)));
    let b = b.wrapping_mul(1911520717);
    let a = a ^ ((b << s) | (b >> (w - s)));
    let a = a.wrapping_mul(2048419325);
    let random = a as f32 * (PI / (!(!0u32 >> 1) as f32));
    let v_x = f32::cos(random);
    let v_y = f32::sin(random);
    Vec2(v_x, v_y)
}

fn normalize(sides: &mut [ProtoSide]) {
    let mut min = f32::MAX;
    let mut max = f32::MIN;

    for side in sides.iter() {
        for h in side.height_map.borrow().values.iter() {
            min = f32::min(min, h.height);
            max = f32::max(max, h.height);
        }
    }

    if min < max {
        for side in sides.iter_mut() {
            for h in side.height_map.borrow_mut().values.iter_mut() {
                h.height = (h.height - min) / (max - min);
            }
        }
    }

    eprintln!("normalized: {} {}", min, max);
}
