use std::fs;
use std::path::Path;

const TILE: i32 = 64;
const GRID: i32 = 8;
const SIZE: u32 = (TILE * GRID) as u32;

type Rgb = [u8; 3];

fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x27d4eb2d)
        ^ (y as u32).wrapping_mul(0x165667b1)
        ^ seed.wrapping_mul(0x9e3779b1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a2d39);
    h ^= h >> 15;
    (h & 0xffff) as f32 / 65535.0
}

fn noise(x: f32, y: f32, scale: f32, seed: u32) -> f32 {
    let fx = x / scale;
    let fy = y / scale;
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
    let sx = smooth(fx - x0 as f32);
    let sy = smooth(fy - y0 as f32);
    let a = hash(x0, y0, seed);
    let b = hash(x0 + 1, y0, seed);
    let c = hash(x0, y0 + 1, seed);
    let d = hash(x0 + 1, y0 + 1, seed);
    a + (b - a) * sx + (c - a) * sy + (a - b - c + d) * sx * sy
}

fn channel(base: f32, factor: f32) -> u8 {
    (base * factor).clamp(0.0, 255.0) as u8
}

fn shaded(color: Rgb, factor: f32) -> Rgb {
    [
        channel(color[0] as f32, factor),
        channel(color[1] as f32, factor),
        channel(color[2] as f32, factor),
    ]
}

fn mixed(a: Rgb, b: Rgb, amount: f32) -> Rgb {
    let amount = amount.clamp(0.0, 1.0);
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * amount) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * amount) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * amount) as u8,
    ]
}

fn edge_grime(x: i32, y: i32) -> f32 {
    let distance = x.min(y).min(TILE - 1 - x).min(TILE - 1 - y) as f32;
    1.0 - 0.26 * ((3.0 - distance).max(0.0) / 3.0)
}

fn mottled(color: Rgb, x: i32, y: i32, seed: u32, strength: f32) -> Rgb {
    let grain = (noise(x as f32, y as f32, 6.0, seed) - 0.5) * strength
        + (hash(x, y, seed + 1) - 0.5) * strength * 0.5;
    shaded(color, 1.0 + grain)
}

fn white(_x: i32, _y: i32) -> Rgb {
    [255, 255, 255]
}

fn ceramic_cell(x: i32, y: i32) -> (i32, i32, bool) {
    let cell_x = x / 16;
    let cell_y = y / 16;
    let mortar = x % 16 == 0 || y % 16 == 0;
    (cell_x, cell_y, mortar)
}

fn tile_blue(x: i32, y: i32) -> Rgb {
    let (cell_x, cell_y, mortar) = ceramic_cell(x, y);
    if mortar {
        return shaded([70, 82, 92], 0.9 + hash(x, y, 3) * 0.2);
    }
    let tone = 0.92 + hash(cell_x, cell_y, 1) * 0.14;
    let cracked = hash(cell_x, cell_y, 17) > 0.92;
    let base = mottled([152, 192, 212], x, y, 4, 0.08);
    let base = if cracked { shaded(base, 0.78) } else { base };
    shaded(base, tone * edge_grime(x, y))
}

fn tile_dirty(x: i32, y: i32) -> Rgb {
    let base = tile_blue(x, y);
    let drip = noise(x as f32 * 0.8, y as f32 * 0.25, 7.0, 21);
    let amount = ((drip - 0.55) / 0.45).clamp(0.0, 1.0) * (1.0 - y as f32 / 64.0) * 1.4;
    if amount <= 0.0 {
        return base;
    }
    mixed(base, [122, 74, 42], amount.min(0.85))
}

fn condensate(x: i32, y: i32) -> Rgb {
    let mut factor = 1.0 + (noise(x as f32, y as f32, 10.0, 8) - 0.5) * 0.12;
    if x < 2 || y < 2 {
        factor *= 1.18;
    }
    if x > TILE - 3 || y > TILE - 3 {
        factor *= 0.78;
    }
    let mut base = shaded([88, 104, 126], factor * edge_grime(x, y));
    if hash(x, y, 5) > 0.986 {
        base = mixed(base, [205, 228, 248], 0.8);
    }
    base
}

fn rust(x: i32, y: i32) -> Rgb {
    let blotch =
        noise(x as f32, y as f32, 9.0, 3) * 0.65 + noise(x as f32, y as f32, 4.0, 9) * 0.35;
    let base = mixed([74, 38, 22], [186, 104, 42], blotch);
    shaded(base, edge_grime(x, y) * (0.9 + hash(x, y, 6) * 0.2))
}

fn grate(x: i32, y: i32) -> Rgb {
    let hole = (x / 8 + y / 8) % 2 == 0 && x % 8 > 1 && x % 8 < 7 && y % 8 > 1 && y % 8 < 7;
    if hole {
        return shaded([24, 26, 30], 0.9 + hash(x, y, 2) * 0.3);
    }
    let mut factor = 1.0 + (noise(x as f32, y as f32, 5.0, 12) - 0.5) * 0.15;
    if x % 8 == 1 || y % 8 == 1 {
        factor *= 1.25;
    }
    shaded([62, 66, 72], factor * edge_grime(x, y))
}

fn ice(x: i32, y: i32) -> Rgb {
    let frost = noise(x as f32 * 0.6, y as f32 * 0.25, 5.0, 14);
    let base = mixed([148, 172, 204], [225, 236, 248], frost);
    let streak = noise(x as f32 * 0.8, y as f32 * 0.15, 6.0, 15);
    let base = if streak > 0.68 {
        mixed(base, [240, 246, 252], 0.5)
    } else {
        base
    };
    shaded(base, edge_grime(x, y))
}

fn concrete_blue(x: i32, y: i32) -> Rgb {
    let mut base = mottled([118, 126, 138], x, y, 31, 0.12);
    if hash(x, y, 32) > 0.965 {
        base = shaded(base, 0.72);
    }
    shaded(base, edge_grime(x, y))
}

fn water(x: i32, y: i32) -> Rgb {
    let wave = ((y as f32 + noise(x as f32, y as f32, 9.0, 41) * 9.0) * 0.55).sin();
    let base = shaded([20, 34, 54], 0.9 + wave * 0.12);
    if hash(x, y, 42) > 0.994 {
        return mixed(base, [130, 168, 208], 0.7);
    }
    base
}

fn concrete(x: i32, y: i32) -> Rgb {
    let mut base = mottled([128, 124, 118], x, y, 51, 0.1);
    let crack = noise(x as f32 * 0.4, y as f32 * 0.4, 11.0, 52);
    if crack > 0.8 {
        base = shaded(base, 0.74);
    }
    shaded(base, edge_grime(x, y))
}

fn concrete_dark(x: i32, y: i32) -> Rgb {
    shaded(concrete(x, y), 0.62)
}

fn metal_panel(x: i32, y: i32) -> Rgb {
    let mut factor = 1.0 + (noise(x as f32 * 1.5, y as f32 * 0.4, 6.0, 61) - 0.5) * 0.08;
    if x < 3 || y < 3 {
        factor *= 1.16;
    }
    if x > TILE - 4 || y > TILE - 4 {
        factor *= 0.76;
    }
    for (rx, ry) in [(7, 7), (TILE - 8, 7), (7, TILE - 8), (TILE - 8, TILE - 8)] {
        let distance = ((x - rx) * (x - rx) + (y - ry) * (y - ry)) as f32;
        if distance < 6.0 {
            factor *= 1.1;
        }
        if (2.5..6.0).contains(&distance) {
            factor *= 0.85;
        }
    }
    shaded([138, 138, 144], factor * edge_grime(x, y))
}

fn metal_brushed(x: i32, y: i32) -> Rgb {
    let streak = (hash(0, y, 71) - 0.5) * 0.12 + (hash(x / 24, y, 72) - 0.5) * 0.06;
    shaded([142, 140, 136], (1.0 + streak) * edge_grime(x, y))
}

fn warning(x: i32, y: i32) -> Rgb {
    let stripe = (x + y) % 24 < 12;
    let base = if stripe { [222, 182, 44] } else { [32, 30, 28] };
    let wear = noise(x as f32, y as f32, 7.0, 81);
    let base = if wear > 0.72 {
        mixed(base, [110, 106, 100], 0.6)
    } else {
        base
    };
    shaded(base, edge_grime(x, y))
}

fn floor_plate(x: i32, y: i32) -> Rgb {
    let mut factor = 1.0 + (noise(x as f32, y as f32, 8.0, 91) - 0.5) * 0.1;
    if (x + y) % 16 < 3 {
        factor *= 1.14;
    }
    if (x - y).rem_euclid(16) < 3 {
        factor *= 0.9;
    }
    shaded([112, 114, 120], factor * edge_grime(x, y))
}

fn dirt(x: i32, y: i32) -> Rgb {
    let mut base = mottled([72, 64, 56], x, y, 101, 0.22);
    if hash(x, y, 102) > 0.978 {
        base = shaded(base, 1.35);
    }
    shaded(base, edge_grime(x, y))
}

fn lab_seam(x: i32, y: i32, period: i32) -> bool {
    x % period == 0 || y % period == 0
}

fn lab_tile(x: i32, y: i32) -> Rgb {
    if lab_seam(x, y, 32) {
        return shaded([188, 194, 200], 0.95);
    }
    let mut base = mottled([236, 238, 240], x, y, 111, 0.03);
    if noise(x as f32, y as f32, 9.0, 112) > 0.85 {
        base = shaded(base, 0.93);
    }
    base
}

fn lab_stripe(x: i32, y: i32) -> Rgb {
    let base = lab_tile(x, y);
    if (24..40).contains(&(y % 64)) && !lab_seam(x, y, 32) {
        let worn = noise(x as f32, y as f32, 6.0, 121) > 0.7;
        return if worn {
            mixed(base, [178, 52, 48], 0.5)
        } else {
            mixed(base, [196, 56, 50], 0.9)
        };
    }
    base
}

fn lab_panel(x: i32, y: i32) -> Rgb {
    if lab_seam(x, y, 16) {
        return shaded([176, 180, 184], 0.96);
    }
    let cell = (x / 16, y / 16);
    let tone = 0.97 + hash(cell.0, cell.1, 131) * 0.06;
    shaded(
        mottled([208, 212, 216], x, y, 132, 0.04),
        tone * edge_grime(x, y),
    )
}

fn brick_layout(x: i32, y: i32) -> (i32, i32, bool) {
    let row = y / 16;
    let shifted = x + (row % 2) * 16;
    let column = shifted / 32;
    let mortar = y % 16 < 2 || shifted % 32 < 2;
    (column, row, mortar)
}

fn brick(x: i32, y: i32) -> Rgb {
    let (column, row, mortar) = brick_layout(x, y);
    if mortar {
        return shaded([62, 55, 50], 0.9 + hash(x, y, 141) * 0.2);
    }
    let tone = 0.82 + hash(column, row, 142) * 0.3;
    shaded(
        mottled([156, 82, 60], x, y, 143, 0.1),
        tone * edge_grime(x, y),
    )
}

fn brick_dark(x: i32, y: i32) -> Rgb {
    shaded(brick(x, y), 0.66)
}

fn stone_blocks(x: i32, y: i32) -> Rgb {
    let column = x / 32;
    let row = y / 32;
    let mortar = x % 32 < 3 || y % 32 < 3;
    if mortar {
        return shaded([72, 68, 62], 0.9 + hash(x, y, 151) * 0.2);
    }
    let tone = 0.78 + hash(column, row, 152) * 0.4;
    let chip = if (x % 32 < 5 || y % 32 < 5) && hash(x, y, 153) > 0.6 {
        0.85
    } else {
        1.0
    };
    shaded(
        mottled([132, 127, 116], x, y, 154, 0.13),
        tone * chip * edge_grime(x, y),
    )
}

fn moss(x: i32, y: i32) -> Rgb {
    let base = brick(x, y);
    let growth = noise(x as f32 * 0.7, y as f32 * 0.5, 7.0, 161);
    let reach = 18.0 + noise(0.0, x as f32, 9.0, 162) * 16.0;
    if growth > 0.55 && (y as f32) < reach {
        return mixed(base, [74, 112, 58], ((growth - 0.55) / 0.45).min(0.85));
    }
    if growth > 0.72 {
        return mixed(base, [64, 98, 52], 0.5);
    }
    base
}

type Painter = fn(i32, i32) -> Rgb;

fn main() {
    let tiles: Vec<(&str, f32, Painter)> = vec![
        ("white", 4.0, white),
        ("tile_blue", 2.0, tile_blue),
        ("tile_dirty", 2.0, tile_dirty),
        ("condensate", 2.0, condensate),
        ("rust", 2.5, rust),
        ("grate", 1.0, grate),
        ("ice", 2.0, ice),
        ("concrete_blue", 3.0, concrete_blue),
        ("water", 3.0, water),
        ("concrete", 3.0, concrete),
        ("concrete_dark", 3.0, concrete_dark),
        ("metal_panel", 2.0, metal_panel),
        ("metal_brushed", 2.0, metal_brushed),
        ("warning", 2.0, warning),
        ("floor_plate", 2.0, floor_plate),
        ("dirt", 3.0, dirt),
        ("lab_tile", 2.0, lab_tile),
        ("lab_stripe", 2.0, lab_stripe),
        ("lab_panel", 2.0, lab_panel),
        ("brick", 2.5, brick),
        ("brick_dark", 2.5, brick_dark),
        ("stone_blocks", 3.0, stone_blocks),
        ("moss", 2.5, moss),
    ];
    assert!(
        tiles.len() <= (GRID * GRID) as usize,
        "too many tiles for the atlas"
    );

    let mut atlas = image::RgbaImage::new(SIZE, SIZE);
    let mut manifest_tiles = String::new();
    for (index, (name, meters, paint)) in tiles.iter().enumerate() {
        let index = index as i32;
        let column = index % GRID;
        let row = index / GRID;
        for y in 0..TILE {
            for x in 0..TILE {
                let [r, g, b] = paint(x, y);
                atlas.put_pixel(
                    (column * TILE + x) as u32,
                    (row * TILE + y) as u32,
                    image::Rgba([r, g, b, 255]),
                );
            }
        }
        manifest_tiles.push_str(&format!(
            "    (name: \"{name}\", col: {column}, row: {row}, meters: {meters}),\n"
        ));
    }
    let directory = Path::new("assets/textures");
    fs::create_dir_all(directory).expect("cannot create assets/textures");
    atlas
        .save(directory.join("atlas.png"))
        .expect("cannot write atlas.png");
    let manifest =
        format!("(image: \"atlas.png\",\n tile_size: {TILE},\n tiles: [\n{manifest_tiles} ])\n");
    fs::write(directory.join("atlas.ron"), manifest).expect("cannot write atlas.ron");

    let white_pixel = atlas.get_pixel(1, 1);
    assert_eq!(
        white_pixel.0,
        [255, 255, 255, 255],
        "white tile is not white"
    );
    let warning_pixel = atlas.get_pixel(5 * 64 + 4, 64 + 4);
    assert!(
        warning_pixel.0[0] > 150,
        "warning stripes lost their yellow"
    );
    println!("wrote {} tiles to assets/textures/atlas.png", tiles.len());
}
