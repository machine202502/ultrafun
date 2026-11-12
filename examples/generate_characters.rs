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

fn worn(color: Rgb, x: i32, y: i32, seed: u32, strength: f32) -> Rgb {
    let grain = (noise(x as f32, y as f32, 5.0, seed) - 0.5) * strength
        + (hash(x, y, seed + 1) - 0.5) * strength * 0.6;
    shaded(color, 1.0 + grain)
}

const DIGITS: [&str; 10] = [
    "111101101101111",
    "001001001001001",
    "111001111100111",
    "111001111001111",
    "101101111001001",
    "111100111001111",
    "111100111101111",
    "111001001001001",
    "111101111101111",
    "111101111001111",
];

fn stencil_digit(color: Rgb, x: i32, y: i32, digit: u8, scale: i32) -> Rgb {
    let pattern = DIGITS[digit as usize];
    let cell_x = x / scale;
    let cell_y = y / scale;
    if (0..3).contains(&cell_x) && (0..5).contains(&cell_y) {
        let bit = pattern.as_bytes()[(cell_y * 3 + cell_x) as usize];
        if bit == b'1' {
            return color;
        }
    }
    Rgb::default()
}

fn draw_number(base: Rgb, x: i32, y: i32, number: &str) -> Rgb {
    let scale = 3;
    let index = (x - 8) / (4 * scale);
    if (0..number.len() as i32).contains(&index) && (8..8 + 5 * scale).contains(&y) {
        let digit = number.as_bytes()[index as usize] - b'0';
        let digit = stencil_digit(
            [16, 16, 18],
            (x - 8 - index * 4 * scale) / scale,
            (y - 8) / scale,
            digit,
            1,
        );
        if digit != Rgb::default() {
            return digit;
        }
    }
    base
}

fn player_head(x: i32, y: i32) -> Rgb {
    let mut base = worn([44, 46, 54], x, y, 301, 0.1);
    if y == 31 || y == 32 {
        base = mixed(base, [90, 220, 230], 0.55);
    }
    if (14..18).contains(&y) && hash(x, y, 302) > 0.85 {
        base = shaded(base, 0.8);
    }
    base
}

fn player_torso(x: i32, y: i32) -> Rgb {
    let mut base = worn([52, 58, 70], x, y, 311, 0.12);
    if (20..44).contains(&y) && (x == 18 || x == 19 || x == 44 || x == 45) {
        base = shaded([38, 40, 46], 1.0);
    }
    if (50..58).contains(&y) {
        base = if (x + y) % 16 < 8 {
            mixed(base, [200, 160, 40], 0.85)
        } else {
            shaded(base, 0.7)
        };
    }
    draw_number(base, x, y, "4417")
}

fn player_arms(x: i32, y: i32) -> Rgb {
    let mut base = worn([48, 52, 62], x, y, 321, 0.1);
    if y > 48 && (0..7).contains(&(x % 16)) {
        base = shaded(base, 0.82);
    }
    if (12..20).contains(&y) {
        base = mixed(base, [200, 160, 40], 0.3);
    }
    base
}

fn player_legs(x: i32, y: i32) -> Rgb {
    let mut base = worn([40, 42, 48], x, y, 331, 0.12);
    if (24..40).contains(&y) {
        base = shaded([58, 60, 66], 0.95 + (hash(x / 8, y / 8, 332) - 0.5) * 0.1);
    }
    base
}

fn player_back(x: i32, y: i32) -> Rgb {
    let mut base = worn([46, 48, 56], x, y, 341, 0.12);
    if (12..52).contains(&x) && (12..40).contains(&y) {
        base = shaded(base, 0.85);
        if (x == 12 || x == 51 || y == 12 || y == 39) && (x + y) % 8 < 4 {
            base = mixed(base, [120, 200, 210], 0.4);
        }
    }
    base
}

fn grunt_head(x: i32, y: i32) -> Rgb {
    let mut base = worn([176, 160, 122], x, y, 401, 0.08);
    if y == 32 || (x == 32 && y > 32) {
        base = shaded(base, 0.75);
    }
    if (10..22).contains(&x) && (10..18).contains(&y) && hash(x / 2, y / 2, 402) > 0.5 {
        base = shaded(base, 0.85);
    }
    base
}

fn grunt_torso(x: i32, y: i32) -> Rgb {
    let mut base = worn([168, 152, 116], x, y, 411, 0.09);
    if y == 20 || y == 44 {
        base = shaded(base, 0.8);
    }
    draw_number(base, x, y, "0770")
}

fn grunt_limb(x: i32, y: i32) -> Rgb {
    let mut base = worn([172, 156, 118], x, y, 421, 0.08);
    if y % 16 == 0 {
        base = shaded(base, 0.78);
    }
    base
}

fn grunt_markings(x: i32, y: i32) -> Rgb {
    let mut base = worn([200, 190, 170], x, y, 431, 0.05);
    if (16..48).contains(&x) && (16..48).contains(&y) {
        base = if (x + y) % 12 < 6 {
            mixed(base, [190, 140, 30], 0.9)
        } else {
            shaded(base, 0.3)
        };
    }
    base
}

fn welder_head(x: i32, y: i32) -> Rgb {
    let mut base = worn([50, 44, 40], x, y, 501, 0.12);
    if (20..44).contains(&x) && (22..38).contains(&y) {
        base = mixed(base, [255, 170, 40], 0.75 + hash(x, y, 502) * 0.25);
        if hash(x, y, 503) > 0.9 {
            base = mixed(base, [120, 60, 20], 0.5);
        }
    }
    base
}

fn welder_torso(x: i32, y: i32) -> Rgb {
    let mut base = worn([96, 66, 40], x, y, 511, 0.14);
    if y < 18 {
        base = worn([64, 58, 54], x, y, 512, 0.1);
    }
    if (52..60).contains(&y) && hash(x / 3, y / 3, 513) > 0.7 {
        base = mixed(base, [200, 110, 30], 0.4);
    }
    base
}

fn welder_arm(x: i32, y: i32) -> Rgb {
    let mut base = worn([70, 62, 56], x, y, 521, 0.12);
    if (40..56).contains(&x) && (40..56).contains(&y) {
        base = mixed(base, [90, 90, 96], 0.8);
    }
    base
}

fn inspector_head(x: i32, y: i32) -> Rgb {
    let mut base = worn([210, 206, 200], x, y, 601, 0.05);
    if (22..42).contains(&x) && (24..30).contains(&y) {
        base = mixed(base, [80, 230, 120], 0.6);
    }
    if (22..42).contains(&x) && y == 30 {
        base = mixed(base, [40, 120, 70], 0.7);
    }
    base
}

fn inspector_torso(x: i32, y: i32) -> Rgb {
    let mut base = worn([186, 184, 180], x, y, 611, 0.06);
    if x == 32 {
        base = shaded(base, 0.88);
    }
    if (12..24).contains(&x) && (20..44).contains(&y) {
        base = mixed(base, [236, 234, 228], 0.7);
    }
    base
}

fn inspector_limb(x: i32, y: i32) -> Rgb {
    let mut base = worn([180, 178, 174], x, y, 621, 0.06);
    if y > 52 {
        base = shaded([60, 58, 56], 1.0);
    }
    base
}

fn loader_head(x: i32, y: i32) -> Rgb {
    let mut base = worn([64, 60, 56], x, y, 701, 0.12);
    if (14..50).contains(&y) && (x + y) % 20 < 10 {
        base = mixed(base, [210, 170, 30], 0.8);
    }
    if (22..42).contains(&x) && (26..36).contains(&y) {
        base = mixed(base, [255, 120, 40], 0.5);
    }
    base
}

fn loader_torso(x: i32, y: i32) -> Rgb {
    let mut base = worn([188, 150, 36], x, y, 711, 0.12);
    if (16..48).contains(&y) {
        let chevron = ((y - 16) / 6) % 2 == 0;
        if chevron && (x % 16) < 8 {
            base = shaded(base, 0.72);
        }
    }
    if y % 16 == 0 {
        base = shaded(base, 0.8);
    }
    base
}

fn loader_limb(x: i32, y: i32) -> Rgb {
    let mut base = worn([120, 104, 40], x, y, 721, 0.12);
    if (8..56).contains(&x) && ((14..20).contains(&y) || (44..50).contains(&y)) {
        base = mixed(base, [70, 72, 78], 0.85);
    }
    base
}

type Painter = fn(i32, i32) -> Rgb;

fn main() {
    let tiles: Vec<(&str, Painter)> = vec![
        ("player.head", player_head),
        ("player.torso", player_torso),
        ("player.arms", player_arms),
        ("player.legs", player_legs),
        ("player.back", player_back),
        ("grunt.head", grunt_head),
        ("grunt.torso", grunt_torso),
        ("grunt.limb", grunt_limb),
        ("grunt.markings", grunt_markings),
        ("welder.head", welder_head),
        ("welder.torso", welder_torso),
        ("welder.arm", welder_arm),
        ("inspector.head", inspector_head),
        ("inspector.torso", inspector_torso),
        ("inspector.limb", inspector_limb),
        ("loader.head", loader_head),
        ("loader.torso", loader_torso),
        ("loader.limb", loader_limb),
    ];
    assert!(tiles.len() <= (GRID * GRID) as usize, "too many tiles");

    let mut atlas = image::RgbaImage::new(SIZE, SIZE);
    let mut manifest_tiles = String::new();
    for (index, (name, paint)) in tiles.iter().enumerate() {
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
            "    (name: \"{name}\", col: {column}, row: {row}),\n"
        ));
    }
    let directory = Path::new("assets/textures");
    fs::create_dir_all(directory).expect("cannot create assets/textures");
    atlas
        .save(directory.join("characters.png"))
        .expect("cannot write characters.png");
    let manifest = format!(
        "(image: \"characters.png\",\n tile_size: {TILE},\n tiles: [\n{manifest_tiles} ])\n"
    );
    fs::write(directory.join("characters.ron"), manifest).expect("cannot write characters.ron");

    let visor = atlas.get_pixel(2, 32);
    assert!(visor.0[2] > visor.0[0] + 40, "player visor lost its glow");
    let mask = atlas.get_pixel(64 + 30, 64 + 30);
    assert!(mask.0[0] > 150, "welder viewport lost its glow");
    let plate = atlas.get_pixel(4, 2 * 64 + 24);
    assert!(plate.0[0] > 120, "loader plating lost its yellow");
    println!(
        "wrote {} character tiles to assets/textures/characters.png",
        tiles.len()
    );
}
