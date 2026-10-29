use std::f32::consts::TAU;
use std::path::Path;

const SAMPLE_RATE: u32 = 44_100;
const PEAK: f32 = 0.9;

type Generator = fn() -> Vec<f32>;

struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

struct Sweep {
    phase: f32,
}

impl Sweep {
    fn new() -> Self {
        Self { phase: 0.0 }
    }

    fn sine(&mut self, frequency: f32) -> f32 {
        self.phase = (self.phase + frequency / SAMPLE_RATE as f32).fract();
        (self.phase * TAU).sin()
    }

    fn saw(&mut self, frequency: f32) -> f32 {
        self.phase = (self.phase + frequency / SAMPLE_RATE as f32).fract();
        self.phase * 2.0 - 1.0
    }
}

struct LowPass(f32);

impl LowPass {
    fn apply(&mut self, input: f32, amount: f32) -> f32 {
        self.0 += (input - self.0) * amount;
        self.0
    }
}

fn decay(time: f32, seconds: f32) -> f32 {
    (-time / seconds).exp()
}

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

fn render(seconds: f32, mut sample: impl FnMut(f32, f32) -> f32) -> Vec<f32> {
    let count = (seconds * SAMPLE_RATE as f32) as usize;
    let mut samples: Vec<f32> = (0..count)
        .map(|index| {
            let time = index as f32 / SAMPLE_RATE as f32;
            sample(time, time / seconds)
        })
        .collect();
    let loudest = samples
        .iter()
        .fold(0.0_f32, |peak, value| peak.max(value.abs()));
    if loudest > 0.0 {
        for value in &mut samples {
            *value *= PEAK / loudest;
        }
    }
    samples
}

fn shot() -> Vec<f32> {
    let mut noise = Noise(1);
    let mut body = Sweep::new();
    render(0.14, |time, progress| {
        noise.next() * decay(time, 0.02) * 0.7
            + body.sine(lerp(180.0, 60.0, progress)) * decay(time, 0.05) * 0.8
    })
}

fn impact() -> Vec<f32> {
    let mut noise = Noise(2);
    let mut previous = 0.0;
    render(0.06, |time, _| {
        let current = noise.next();
        let crack = current - previous;
        previous = current;
        crack * decay(time, 0.01)
    })
}

fn enemy_hit() -> Vec<f32> {
    let mut noise = Noise(3);
    let mut filter = LowPass(0.0);
    let mut body = Sweep::new();
    render(0.1, |time, progress| {
        body.sine(lerp(120.0, 80.0, progress)) * decay(time, 0.03)
            + filter.apply(noise.next(), 0.2) * decay(time, 0.02) * 0.8
    })
}

fn enemy_killed() -> Vec<f32> {
    let mut noise = Noise(4);
    let mut filter = LowPass(0.0);
    let mut body = Sweep::new();
    render(0.35, |time, progress| {
        filter.apply(noise.next(), 0.3) * decay(time, 0.12)
            + body.saw(lerp(300.0, 80.0, progress)) * decay(time, 0.15) * 0.4
    })
}

fn part_collected() -> Vec<f32> {
    let mut tone = Sweep::new();
    render(0.09, |time, progress| {
        tone.sine(lerp(600.0, 1400.0, progress)) * decay(time, 0.05)
    })
}

fn player_damaged() -> Vec<f32> {
    let mut noise = Noise(5);
    let mut buzz = Sweep::new();
    render(0.22, |time, _| {
        buzz.saw(90.0) * decay(time, 0.1) + noise.next() * decay(time, 0.05) * 0.4
    })
}

fn jump() -> Vec<f32> {
    let mut tone = Sweep::new();
    render(0.07, |time, progress| {
        tone.sine(lerp(250.0, 450.0, progress)) * decay(time, 0.03)
    })
}

fn dash() -> Vec<f32> {
    let mut noise = Noise(6);
    let mut filter = LowPass(0.0);
    render(0.22, |_, progress| {
        let cutoff = lerp(0.05, 0.6, progress);
        filter.apply(noise.next(), cutoff) * (progress * std::f32::consts::PI).sin()
    })
}

fn slam() -> Vec<f32> {
    let mut noise = Noise(7);
    let mut filter = LowPass(0.0);
    let mut body = Sweep::new();
    render(0.5, |time, progress| {
        body.sine(lerp(90.0, 35.0, progress)) * decay(time, 0.18)
            + filter.apply(noise.next(), 0.15) * decay(time, 0.06) * 1.5
    })
}

fn door() -> Vec<f32> {
    let mut noise = Noise(8);
    let mut partials = [220.0, 563.0, 1101.0, 1764.0].map(|frequency| (Sweep::new(), frequency));
    let decays = [0.4, 0.25, 0.15, 0.1];
    render(0.8, |time, _| {
        let ring: f32 = partials
            .iter_mut()
            .zip(decays)
            .map(|((sweep, frequency), seconds)| sweep.sine(*frequency) * decay(time, seconds))
            .sum();
        ring + noise.next() * decay(time, 0.01)
    })
}

fn chime() -> Vec<f32> {
    let mut notes = [(0.0, 523.25), (0.12, 659.25), (0.24, 783.99)]
        .map(|(start, frequency)| (start, frequency, Sweep::new()));
    render(0.9, |time, _| {
        notes
            .iter_mut()
            .filter(|(start, _, _)| time >= *start)
            .map(|(start, frequency, sweep)| sweep.sine(*frequency) * decay(time - *start, 0.35))
            .sum()
    })
}

fn radio() -> Vec<f32> {
    let mut noise = Noise(9);
    let mut gate = Sweep::new();
    render(0.12, |time, _| {
        let square = gate.sine(1200.0).signum();
        (noise.next() * 0.6 + square * 0.4) * decay(time, 0.05)
    })
}

fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_size = samples.len() as u32 * 2;
    let mut bytes = Vec::with_capacity(44 + data_size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for sample in samples {
        let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn main() -> std::io::Result<()> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio");
    std::fs::create_dir_all(&directory)?;
    let sounds: [(&str, Generator); 12] = [
        ("shot", shot),
        ("impact", impact),
        ("enemy_hit", enemy_hit),
        ("enemy_killed", enemy_killed),
        ("part_collected", part_collected),
        ("player_damaged", player_damaged),
        ("jump", jump),
        ("dash", dash),
        ("slam", slam),
        ("door", door),
        ("chime", chime),
        ("radio", radio),
    ];
    for (name, generate) in sounds {
        let path = directory.join(format!("{name}.wav"));
        std::fs::write(&path, wav_bytes(&generate()))?;
        println!("{}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_matches_sample_count() {
        let bytes = wav_bytes(&[0.0; 100]);
        assert_eq!(bytes.len(), 44 + 200);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            36 + 200
        );
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 200);
    }

    #[test]
    fn generation_is_deterministic() {
        assert_eq!(shot(), shot());
        assert_eq!(door(), door());
    }
}
