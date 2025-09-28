use std::cell::RefCell;
use std::f32::consts::PI;
use std::io::Write;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl Default for Side {
    fn default() -> Self {
        Self::L
    }
}

impl std::fmt::Display for Side {
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

impl Side {
    fn to_index(self) -> usize {
        match self {
            Side::L => 0,
            Side::B => 1,
            Side::R => 2,
            Side::F => 3,
            Side::U => 4,
            Side::D => 5,
        }
    }
}

impl From<usize> for Side {
    fn from(value: usize) -> Self {
        debug_assert!(value < 6);
        match value {
            0 => Side::L,
            1 => Side::B,
            2 => Side::R,
            3 => Side::F,
            4 => Side::U,
            5 => Side::D,
            _ => unreachable!(),
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
    //pub erosion_brush_radius: usize,
    pub erosion_max_lifetime: usize,
    pub erosion_start_speed: f32,
    pub erosion_start_water: f32,
    pub erosion_inertia: f32,
    pub erosion_min_sediment_capacity: f32,
    pub erosion_sediment_capacity_factor: f32,
    pub erosion_erode_speed: f32,
    pub erosion_deposit_speed: f32,
    pub erosion_gravity: f32,
    pub erosion_evaporate_speed: f32,
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
        //erosion_brush_radius,
        erosion_max_lifetime,
        erosion_start_speed,
        erosion_start_water,
        erosion_inertia,
        erosion_min_sediment_capacity,
        erosion_sediment_capacity_factor,
        erosion_erode_speed,
        erosion_deposit_speed,
        erosion_gravity,
        erosion_evaporate_speed,
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
            let side = Side::from(rng.next_i32_between(0, 5) as usize);
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

                let side = &mut sides[candidate.side.to_index()];
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
                    Side::L => ContinentPixel {
                        side: Side::F,
                        ix: width - 1,
                        iy: pixel.iy,
                    },
                    // back -> left
                    Side::B => ContinentPixel {
                        side: Side::L,
                        ix: width - 1,
                        iy: pixel.iy,
                    },
                    // right -> back
                    Side::R => ContinentPixel {
                        side: Side::B,
                        ix: width - 1,
                        iy: pixel.iy,
                    },
                    // front -> right
                    Side::F => ContinentPixel {
                        side: Side::R,
                        ix: width - 1,
                        iy: pixel.iy,
                    },
                    // up -> left
                    Side::U => ContinentPixel {
                        side: Side::L,
                        ix: pixel.iy,
                        iy: 0,
                    },
                    // down -> left
                    Side::D => ContinentPixel {
                        side: Side::L,
                        ix: width - 1 - pixel.iy,
                        iy: width - 1,
                    },
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
                    Side::L => ContinentPixel {
                        side: Side::B,
                        ix: 0,
                        iy: pixel.iy,
                    },
                    // back -> right
                    Side::B => ContinentPixel {
                        side: Side::R,
                        ix: 0,
                        iy: pixel.iy,
                    },
                    // right -> front
                    Side::R => ContinentPixel {
                        side: Side::F,
                        ix: 0,
                        iy: pixel.iy,
                    },
                    // front -> left
                    Side::F => ContinentPixel {
                        side: Side::L,
                        ix: 0,
                        iy: pixel.iy,
                    },
                    // up -> right
                    Side::U => ContinentPixel {
                        side: Side::R,
                        ix: width - 1 - pixel.iy,
                        iy: 0,
                    },
                    // down -> right
                    Side::D => ContinentPixel {
                        side: Side::R,
                        ix: pixel.iy,
                        iy: width - 1,
                    },
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
                    Side::L => ContinentPixel {
                        side: Side::U,
                        ix: 0,
                        iy: pixel.ix,
                    },
                    // back -> up
                    Side::B => ContinentPixel {
                        side: Side::U,
                        ix: pixel.ix,
                        iy: width - 1,
                    },
                    // right -> up
                    Side::R => ContinentPixel {
                        side: Side::U,
                        ix: width - 1,
                        iy: width - 1 - pixel.ix,
                    },
                    // front -> up
                    Side::F => ContinentPixel {
                        side: Side::U,
                        ix: width - 1 - pixel.ix,
                        iy: 0,
                    },
                    // up -> front
                    Side::U => ContinentPixel {
                        side: Side::F,
                        ix: width - 1 - pixel.ix,
                        iy: 0,
                    },
                    // down -> back
                    Side::D => ContinentPixel {
                        side: Side::B,
                        ix: pixel.ix,
                        iy: width - 1,
                    },
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
                    Side::L => ContinentPixel {
                        side: Side::D,
                        ix: 0,
                        iy: width - 1 - pixel.ix,
                    },
                    // back -> down
                    Side::B => ContinentPixel {
                        side: Side::D,
                        ix: pixel.ix,
                        iy: 0,
                    },
                    // right -> down
                    Side::R => ContinentPixel {
                        side: Side::D,
                        ix: width - 1,
                        iy: pixel.ix,
                    },
                    // front -> down
                    Side::F => ContinentPixel {
                        side: Side::D,
                        ix: width - 1 - pixel.ix,
                        iy: width - 1,
                    },
                    // up -> back
                    Side::U => ContinentPixel {
                        side: Side::B,
                        ix: pixel.ix,
                        iy: 0,
                    },
                    // down -> front
                    Side::D => ContinentPixel {
                        side: Side::F,
                        ix: width - 1 - pixel.ix,
                        iy: 0,
                    },
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

                    let o =
                        position_on_sphere((origin_pixel.ix, origin_pixel.iy), width, origin_pixel.side);
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
    normalize(&mut sides, Some(129.8125 / 255.0));

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
    normalize(&mut sides, None);

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

    normalize(&mut sides, None);

    // erosion
    //eprintln!("build erosion brush...");
    //let mut brush = ErosionBrush::default();
    //let mut sum = 0.0;
    //let r = erosion_brush_radius as isize;
    //for iy in -r..=r {
    //    for ix in -r..=r {
    //        let sqr_dst = ix * ix + iy * iy;
    //        let weight = if sqr_dst < r * r {
    //            let weight = 1.0 - f32::sqrt(sqr_dst as f32) / r as f32;
    //            sum += weight;
    //            weight
    //        } else {
    //            0.0
    //        };

    //        let value = ErosionBrushValue {
    //            offset: (ix, iy),
    //            weight,
    //        };
    //        brush.values.push(value);
    //    }
    //}

    //for value in brush.values.iter_mut() {
    //    value.weight /= sum;
    //}

    //eprint!("brush:");
    //let brush_width = erosion_brush_radius * 2 + 1;
    //for (i, value) in brush.values.iter().enumerate() {
    //    if i % brush_width == 0 {
    //        eprintln!()
    //    }

    //    if value.weight == 0.0 {
    //        eprint!("      ");
    //    }
    //    else {
    //        eprint!(" {:.3}", value.weight);
    //    }
    //}
    //eprintln!();
    
    eprintln!("find erosion stride...");

    let phi = (1.0 + f32::sqrt(5.0)) / 2.0; // golden ratio
    let resolution = width * width;
    let modulo = resolution * 6;

    let ideal_stride = modulo as f32 * phi;

    let mut offset = 0;

    let stride = loop {
        let candidate1 = ideal_stride as usize + offset;
        let candidate2 = ideal_stride as usize - offset;

        let gcd1 = gcd(candidate1, modulo);
        let gcd2 = gcd(candidate2, modulo);

        if gcd1 == 1 {
            break candidate1;
        }

        if gcd2 == 1 {
            break candidate2;
        }

        offset += 1;
    };

    eprintln!("stride {}, ideal: {}", stride, ideal_stride);

    let mut idrop = rng.next_usize();
    let iterations = erosion_iterations * modulo;
    for i in 0..iterations {
        if i % 10_000 == 0 {
            let progress = i as f32 / iterations as f32 * 100.0;
            eprintln!(
                "erode... {}%",
                progress,
            );
        }

        idrop = (idrop + stride) % modulo;

        let side = idrop / resolution;
        let mut side = sides[side].height_map.borrow().side;

        let index = idrop % resolution;
        let mut pos = Vec2(
            (index % width) as f32,
            (index / width) as f32,
        );
        let mut dir = Vec2::zero();
        let mut eko = ErosionKernelOrigin::default();
        let mut speed = erosion_start_speed;
        let mut water = erosion_start_water;
        let mut sediment = 0.0;

        let mut path = Vec::new();

        for lifetime in 0..erosion_max_lifetime {
            let Vec2(x, y) = pos;
            let node_x = if x == width as f32 {
                x - 0.001
            } else {
                x
            } as usize;
            let node_y = if y == width as f32 {
                y - 0.001
            } else {
                y
            } as usize;

            let cell_offset = Vec2(
                pos.x() - node_x as f32,
                pos.y() - node_y as f32,
            );

            let (gradient, height) = calculate_gradient_and_height(
                pos,
                width,
                side,
                &sides,
                eko,
            );

            dir.set_x(dir.x() * erosion_inertia - gradient.x() * (1.0 - erosion_inertia));
            dir.set_y(dir.y() * erosion_inertia - gradient.y() * (1.0 - erosion_inertia));
            let len = f32::max(0.01, dir.length());
            dir /= len;
            pos += dir;

            // droplet may crossed to another side. we need to remap
            loop {
                let width = width as f32;

                let Vec2(x, y) = pos;

                if x < 0.0 {
                    // droplet moved left
                    // x is negative, negate to make math more intuitive
                    let x = -x;
                    match side {
                        Side::L => {
                            side = Side::F;
                            pos = Vec2(
                                width - x,
                                y,
                            );
                            eprintln!("debug this {}", i);
                        },
                        Side::B => {
                            side = Side::L;
                            pos = Vec2(
                                width - x,
                                y,
                            );
                        },
                        Side::R => {
                            side = Side::B;
                            pos = Vec2(
                                width - x,
                                y,
                            );
                            eprintln!("debug this {}", i);
                        },
                        Side::F => {
                            side = Side::R;
                            pos = Vec2(
                                width - x,
                                y,
                            );
                        },
                        Side::U => {

                            if lifetime == 12 {
                                eprintln!("hoi {:?} {:?}", pos, dir);
                            }
                            side = Side::L;
                            pos = Vec2(
                                y,
                                x,
                            );
                            // rotate dir ccw
                            dir = Vec2(
                                -dir.y(),
                                dir.x(),
                            );
                            eko.rotate_ccw();
                            if lifetime == 12 {
                                eprintln!("poi {:?} {:?}", pos, dir);
                            }
                            eprintln!("debug this {}", i);
                        },
                        Side::D => {
                            side = Side::L;
                            pos = Vec2(
                                width - y,
                                width - x,
                            );
                            // rotate dir cw
                            dir = Vec2(
                                dir.y(),
                                -dir.x(),
                            );
                            eko.rotate_cw();
                        },
                    }
                } else if x > width {
                    // droplet moved right
                    // mod x back into the side to make math more intuitive
                    match side {
                        Side::L => {
                            side = Side::B;
                            pos = Vec2(
                                x - width,
                                y,
                            );
                        },
                        Side::B => {
                            side = Side::R;
                            pos = Vec2(
                                x - width,
                                y,
                            );
                            eprintln!("debug this {}", i);
                        },
                        Side::R => {
                            side = Side::F;
                            pos = Vec2(
                                x - width,
                                y,
                            );
                            eprintln!("debug this {}", i);
                        },
                        Side::F => {
                            side = Side::L;
                            pos = Vec2(
                                x - width,
                                y,
                            );
                            eprintln!("debug this {}", i);
                        },
                        Side::U => {
                            side = Side::R;
                            pos = Vec2(
                                width - y,
                                x - width,
                            );
                            // rotate dir cw
                            dir = Vec2(
                                dir.y(),
                                -dir.x(),
                            );
                            eko.rotate_cw();
                            eprintln!("debug this {}", i);
                        },
                        Side::D => {
                            side = Side::R;
                            pos = Vec2(
                                y,
                                2.0 * width - x,
                            );
                            // rotate dir ccw
                            dir = Vec2(
                                -dir.y(),
                                dir.x(),
                            );
                            eko.rotate_ccw();
                            eprintln!("debug this {}", i);
                        },
                    }
                } else if y < 0.0 {
                    // droplet moved up
                    // y is negative, negate to make math more intuitive
                    let y = -y;
                    match side {
                        Side::L => {
                            side = Side::U;
                            pos = Vec2(
                                y,
                                x,
                            );
                            // rotate dir cw
                            dir = Vec2(
                                dir.y(),
                                -dir.x(),
                            );
                            eko.rotate_cw();
                            eprintln!("debug this {}", i);
                        },
                        Side::B => {
                            side = Side::U;
                            pos = Vec2(
                                x,
                                width - y,
                            );
                            eprintln!("debug this {}", i);
                        },
                        Side::R => {
                            side = Side::U;
                            pos = Vec2(
                                width - y,
                                width - x,
                            );
                            // rotate dir ccw
                            dir = Vec2(
                                -dir.y(),
                                dir.x(),
                            );
                            eko.rotate_ccw();
                            eprintln!("debug this {}", i);
                        },
                        Side::F => {
                            side = Side::U;
                            pos = Vec2(
                                width - x,
                                y,
                            );
                            // rotate dir 180°
                            dir = Vec2(
                                dir.y(),
                                dir.x(),
                            );
                            eko.rotate_180();
                            eprintln!("debug this {}", i);
                        },
                        Side::U => {
                            side = Side::F;
                            pos = Vec2(
                                width - x,
                                y,
                            );
                            // rotate dir 180°
                            dir = Vec2(
                                dir.y(),
                                dir.x(),
                            );
                            eko.rotate_180();
                            eprintln!("debug this {}", i);
                        },
                        Side::D => {
                            side = Side::B;
                            pos = Vec2(
                                x,
                                width - y,
                            );
                            eprintln!("debug this {}", i);
                        },
                    }
                } else if y > width {
                    // droplet moved down
                    match side {
                        Side::L => {

                            side = Side::D;
                            pos = Vec2(
                                y - width,
                                width - x,
                            );
                            // rotate dir ccw
                            dir = Vec2(
                                -dir.y(),
                                dir.x(),
                            );
                            eko.rotate_ccw();
                        },
                        Side::B => {
                            side = Side::D;
                            pos = Vec2(
                                x,
                                y - width,
                            );
                            eprintln!("debug this {}", i);
                        },
                        Side::R => {
                            side = Side::D;
                            pos = Vec2(
                                2.0 * width - y,
                                x,
                            );
                            // rotate dir cw
                            dir = Vec2(
                                dir.y(),
                                -dir.x(),
                            );
                            eko.rotate_cw();
                            eprintln!("debug this {}", i);
                        },
                        Side::F => {
                            side = Side::D;
                            pos = Vec2(
                                width - x,
                                2.0 * width - y,
                            );
                            // rotate dir 180°
                            dir = Vec2(
                                dir.y(),
                                dir.x(),
                            );
                            eko.rotate_180();
                            eprintln!("debug this {}", i);
                        },
                        Side::U => {
                            side = Side::B;
                            pos = Vec2(
                                x,
                                y - width,
                            );
                            eprintln!("debug this {}", i);
                        },
                        Side::D => {
                            side = Side::F;
                            pos = Vec2(
                                width - x,
                                2.0 * width - y,
                            );
                            // rotate dir 180°
                            dir = Vec2(
                                dir.y(),
                                dir.x(),
                            );
                            eko.rotate_180();
                            eprintln!("debug this {}", i);
                        },
                    }
                } else {
                    break;
                }
            }

            path.push((pos, side));

            if dir.x() == 0.0 && dir.y() == 0.0 {
                break;
            }

            let (_, new_height) = calculate_gradient_and_height(
                pos,
                width,
                side,
                &sides,
                eko,
            );
            let delta_height = new_height - height;

            let sediment_capacity = f32::max(
                -delta_height * speed * water * erosion_sediment_capacity_factor,
                erosion_min_sediment_capacity,
            );

            if sediment > sediment_capacity || delta_height > 0.0 {
                let amount_to_deposit = if delta_height > 0.0 {
                    f32::min(delta_height, sediment)
                } else {
                    (sediment - sediment_capacity) * erosion_deposit_speed
                };

                sediment -= amount_to_deposit;

                eprintln!("use eko here to find the correct nw, ne, sw, and se indices");
                //let deposit_nw = amount_to_deposit * (1.0 - cell_offset.x()) * (1.0 - cell_offset.y());
                //let deposit_ne = amount_to_deposit * cell_offset.x() * (1.0 - cell_offset.y());
                //let deposit_sw = amount_to_deposit * (1.0 - cell_offset.x()) * cell_offset.y();
                //let deposit_se = amount_to_deposit * cell_offset.x() * cell_offset.y();

                //deposit_sediment(
                //    (node_x, node_y),
                //    width,
                //    side,
                //    &sides,
                //    deposit_nw,
                //);
                //deposit_sediment(
                //    (node_x + 1, node_y),
                //    width,
                //    side,
                //    &sides,
                //    deposit_ne,
                //);
                //deposit_sediment(
                //    (node_x, node_y + 1),
                //    width,
                //    side,
                //    &sides,
                //    deposit_sw,
                //);
                //deposit_sediment(
                //    (node_x + 1, node_y + 1),
                //    width,
                //    side,
                //    &sides,
                //    deposit_se,
                //);
            } else {
                let amount_to_erode = f32::min(
                    (sediment_capacity - sediment) * erosion_erode_speed,
                    -delta_height,
                );

                ////if node_y * width + node_x == 4229 {
                //if i == 57316 || i == 57315 {
                //    eprintln!("err {} {:?}", i, pos);
                //}

                let mut h = sides[side.to_index()].height_map.borrow().get(node_x, node_y);
                let delta_sediment = if h.height < amount_to_erode {
                    h.height
                } else {
                    amount_to_erode
                };
                h.height -= delta_sediment;
                sediment += delta_sediment;
                sides[side.to_index()].height_map.borrow_mut().set(node_x, node_y, h);
            }

            speed = f32::sqrt(f32::max(
                0.0,
                speed * speed + delta_height * erosion_gravity,
            ));
            water *= 1.0 - erosion_evaporate_speed;
        }

        // 22
        // 23
        // 29
        // 30
        if i == 21 && false {
            let mut csv = format!("n;lx;ly;bx;by;rx;ry;fx;fy;ux;uy;dx;dy;\n");

            for (i, (Vec2(x, y), side)) in path.iter().enumerate() {
                let to_append = match side {
                    Side::L => format!("{};{};{};;;;;;;;;;;\n", i, x, -y),
                    Side::B => format!("{};;;{};{};;;;;;;;;\n", i, x, -y),
                    Side::R => format!("{};;;;;{};{};;;;;;;\n", i, x, -y),
                    Side::F => format!("{};;;;;;;{};{};;;;;\n", i, x, -y),
                    Side::U => format!("{};;;;;;;;;{};{};;;\n", i, x, -y),
                    Side::D => format!("{};;;;;;;;;;;{};{};\n", i, x, -y),
                };

                csv.push_str(&to_append);
            }

            let file_path = std::path::PathBuf::from(format!("C:/Users/Rismosch/source/repos/terrain_generator/debug_{}.csv", i));
            if file_path.exists() {
                std::fs::remove_file(&file_path).unwrap();
            }

            let mut file = std::fs::File::create_new(&file_path).unwrap();
            file.write_all(csv.as_bytes()).unwrap();

            break;
        }
    }

    normalize(&mut sides, None);

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
    side: Side,
    ix: usize,
    iy: usize,
}

#[derive(Default, Debug, Clone)]
struct Continent {
    origin: ContinentPixel,
    discovered_pixels: Vec<ContinentPixel>,
    rotation_axis: Vec3,
}

//#[derive(Default, Debug, Clone)]
//struct ErosionBrush {
//    values: Vec<ErosionBrushValue>,
//}
//
//#[derive(Debug, Clone)]
//struct ErosionBrushValue {
//    offset: (isize, isize),
//    weight: f32,
//}

// erosion samples 4 cells at different steps. when the droplet goes over a cube edge, the kernel
// may be rotated, and thus changing the "origin cell" that the droplet may find itself in.
#[derive(Clone, Copy)]
enum ErosionKernelOrigin {
    NW,
    NE,
    SW,
    SE,
}

impl Default for ErosionKernelOrigin {
    fn default() -> Self {
        Self::NW
    }
}

impl ErosionKernelOrigin {
    fn rotate_cw(&mut self) {
        *self = match self {
            Self::NW => Self::NE,
            Self::NE => Self::SE,
            Self::SW => Self::NW,
            Self::SE => Self::SW,
        };
    }

    fn rotate_ccw(&mut self) {
        *self = match self {
            Self::NW => Self::SW,
            Self::NE => Self::NW,
            Self::SW => Self::SE,
            Self::SE => Self::NE,
        };
    }

    fn rotate_180(&mut self) {
        self.rotate_cw();
        self.rotate_cw();
    }
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

fn normalize(sides: &mut [ProtoSide], nan_replacement: Option<f32>) {
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
                h.height = match (h.height.is_nan(), nan_replacement) {
                    (true, Some(nan_replacement)) => nan_replacement,
                    _ => (h.height - min) / (max - min)
                };
            }
        }
    }

    eprintln!("normalized: {} {}", min, max);
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }

    a
}

fn remap_erosion_index(
    i: (isize, isize),
    width: usize,
    side: Side,
) -> Result<((usize, usize), Side), (((usize, usize), Side), ((usize, usize), Side))> {
    let (ix, iy) = i;
    let w = width as isize;

    eprintln!("hoi {:?} {:?}", i, side);

    let ((new_ix, new_iy), new_side) = if ix >= 0 && ix < w && iy >= 0 && iy < w {
        // x and y are in range, nothing needs to be wrapped
        (i, side)
    } else if ix < 0 && iy >= 0 && iy < w {
        // x is too small, y is in range
        // move left
        match side {
            Side::L => (
                (
                    w + ix,
                    iy,
                ),
                Side::F,
            ),
            Side::B => (
                (
                    w + ix,
                    iy,
                ),
                Side::L,
            ),
            Side::R => (
                (
                    w + ix,
                    iy,
                ),
                Side::B
            ),
            Side::F => (
                (
                    w + ix,
                    iy,
                ),
                Side::R,
            ),
            Side::U => (
                (
                    iy,
                    -ix - 1,
                ),
                Side::L,
            ),
            Side::D => (
                (
                    w - iy - 1,
                    w + ix,
                ),
                Side::L,
            ),
        }
    } else if ix >= w && iy >= 0 && iy < w {
        // x is too large, y is in range
        // move right
        match side {
            Side::L => (
                (
                    ix - w,
                    iy,
                ),
                Side::B,
            ),
            Side::B => (
                (
                    ix - w,
                    iy,
                ),
                Side::R,
            ),
            Side::R => (
                (
                    ix - w,
                    iy,
                ),
                Side::F,
            ),
            Side::F => (
                (
                    ix - w,
                    iy,
                ),
                Side::L,
            ),
            Side::U => (
                (
                    w - iy - 1,
                    ix - w,
                ),
                Side::R,
            ),
            Side::D => (
                (
                    iy,
                    2 * w - ix - 1,
                ),
                Side::R,
            ),
        }
    } else if ix >= 0 && ix < w && iy < 0 {
        // x is in range, y is too small
        // move up
        match side {
            Side::L => (
                (
                    -iy - 1,
                    ix,
                ),
                Side::U,
            ),
            Side::B => (
                (
                    ix,
                    w + iy,
                ),
                Side::U,
            ),
            Side::R => (
                (
                    w + iy,
                    w - ix - 1,
                ),
                Side::U,
            ),
            Side::F => (
                (
                    w - ix - 1,
                    -iy - 1,
                ),
                Side::U,
            ),
            Side::U => (
                (
                    w - ix - 1,
                    -iy - 1,
                ),
                Side::F,
            ),
            Side::D => (
                (
                    ix,
                    w + iy,
                ),
                Side::B,
            ),
        }
    } else if ix >= 0 && ix < w && iy >= w {
        // x is in range, y is too large
        // move down
        match side {
            Side::L => (
                (
                    iy - w,
                    w - ix - 1,
                ),
                Side::D,
            ),
            Side::B => (
                (
                    ix,
                    iy - w,
                ),
                Side::D,
            ),
            Side::R => (
                (
                    2 * w - iy - 1,
                    ix,
                ),
                Side::D,
            ),
            Side::F => (
                (
                    w - ix - 1,
                    2 * w - iy - 1,
                ),
                Side::D,
            ),
            Side::U => (
                (
                    ix,
                    iy - w,
                ),
                Side::B,
            ),
            Side::D => (
                (
                    w - ix - 1,
                    2 * w - iy - 1,
                ),
                Side::F,
            ),
        }
    } else {
        // neither is in range. client must wrap x and y themself
        todo!("handle each corner differently")
        //return None;
    };

    eprintln!("poi {:?} {:?}", (new_ix, new_iy), new_side);

    Ok(((new_ix as usize, new_iy as usize), new_side))
}

fn sample_height(
    i: (isize, isize),
    width: usize,
    side: Side,
    sides: &[ProtoSide],
) -> f32 {
    eprintln!("sample height {:?} {:?}",i, side);
    match remap_erosion_index(i, width, side) {
        Ok(((ix, iy), side)) => {
            let side_index = side.to_index();
            let h = sides[side_index].height_map.borrow().get(ix, iy);
            h.height
        },
        Err((((lix, liy), lside), ((rix, riy), rside))) => {
            let lside_index = lside.to_index();
            let lh = sides[lside_index].height_map.borrow().get(lix, liy);
            let lval = lh.height;

            let rside_index = rside.to_index();
            let rh = sides[rside_index].height_map.borrow().get(rix, riy);
            let rval = rh.height;

            (lval + rval) / 2.0
        },
    }

}

fn calculate_gradient_and_height(
    pos: Vec2,
    width: usize,
    side: Side,
    sides: &[ProtoSide],
    eko: ErosionKernelOrigin,
) -> (Vec2, f32) {
    let coord_x = pos.x() as isize;
    let coord_y = pos.y() as isize;

    let x = pos.x() - coord_x as f32;
    let y = pos.y() - coord_y as f32;

    let (onw, one, osw, ose) = match eko {
        ErosionKernelOrigin::NW => ((0,0),(1,0),(0,1),(1,1)),
        ErosionKernelOrigin::NE => ((-1,0),(0,0),(-1,1),(0,1)),
        ErosionKernelOrigin::SW => ((0,-1),(1,-1),(0,0),(1,0)),
        ErosionKernelOrigin::SE => ((-1,-1),(0,-1),(-1,0),(0,0)),
    };

    let inw = (coord_x + onw.0, coord_y + onw.1);
    let ine = (coord_x + one.0, coord_y + one.1);
    let isw = (coord_x + osw.0, coord_y + osw.1);
    let ise = (coord_x + ose.0, coord_y + ose.1);

    let nw = sample_height(inw, width, side, sides);
    let ne = sample_height(ine, width, side, sides);
    let sw = sample_height(isw, width, side, sides);
    let se = sample_height(ise, width, side, sides);

    let gradient_x = (ne - nw) * (1.0 - y) + (se * sw) * y;
    let gradient_y = (sw - nw) * (1.0 - x) + (se - ne) * x;
    let gradient = Vec2(gradient_x, gradient_y);

    let height = 
        nw * (1.0 - x) * (1.0 - y) +
        ne * x * (1.0 - y) +
        sw * (1.0 - x) * y +
        se * x * y;

    (gradient, height)
}

fn deposit_sediment(
    ipos: (isize, isize),
    width: usize,
    side: Side,
    sides: &[ProtoSide],
    sediment: f32,
) {
    todo!("deposit sediment");
    //match remap_erosion_index(ipos, width, side) {
    //    Some(((ix, iy), side)) => {
    //        let side_index = side.to_index();
    //        let mut h = sides[side_index].height_map.borrow().get(ix, iy);
    //        h.height += sediment;
    //        let side = &sides[side_index];
    //        let height_map = &side.height_map;
    //        let mut height_map = height_map.borrow_mut();
    //        height_map.set(ix, iy, h);
    //    },
    //    None => {
    //        let (ix, iy) = ipos;

    //        deposit_sediment((ix - 1, iy), width, side, sides, sediment * 0.5);
    //        deposit_sediment((ix, iy - 1), width, side, sides, sediment * 0.5);
    //    }
    //}
}
