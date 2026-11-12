use std::fs;
use std::path::Path;

const SAMPLE_RATE: u32 = 22050;
const SECONDS: u32 = 32;

fn write_wav(path: &std::path::PathBuf, samples: &[f32]) {
    let mut pcm = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        let value = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        pcm.extend_from_slice(&value.to_le_bytes());
    }
    let data_len = pcm.len() as u32;
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(&pcm);
    fs::write(path, wav).expect("cannot write wav");
}

fn hash(index: u32) -> f32 {
    let mut h = index.wrapping_mul(0x9e3779b1).wrapping_add(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

fn one_pole(state: &mut f32, input: f32, cutoff_hz: f32) -> f32 {
    let alpha = 1.0 - (-2.0 * std::f32::consts::PI * cutoff_hz / SAMPLE_RATE as f32).exp();
    *state += alpha * (input - *state);
    *state
}

fn calm_music() -> Vec<f32> {
    let total = (SAMPLE_RATE * SECONDS) as usize;
    let mut samples = vec![0.0f32; total];
    let mut drone_a = 0.0f32;
    let mut drone_b = 0.0f32;
    let mut sub = 0.0f32;
    let mut hiss_state = 0.0f32;
    for (index, sample) in samples.iter_mut().enumerate() {
        let time = index as f32 / SAMPLE_RATE as f32;
        let a_frequency = 55.0;
        drone_a += a_frequency / SAMPLE_RATE as f32;
        drone_b += (a_frequency * 1.005) / SAMPLE_RATE as f32;
        sub += (a_frequency / 2.0) / SAMPLE_RATE as f32;
        let saw_a = (drone_a.fract() * 2.0 - 1.0) * 0.5;
        let saw_b = (drone_b.fract() * 2.0 - 1.0) * 0.5;
        let sine_sub = (sub * std::f32::consts::TAU).sin() * 0.5;
        let swell = 0.6 + 0.4 * (time * std::f32::consts::TAU / 16.0).sin();
        let noise = hash(index as u32) * 2.0 - 1.0;
        let hiss = one_pole(&mut hiss_state, noise, 900.0) * 0.12;
        *sample = (saw_a + saw_b) * 0.16 * swell + sine_sub * 0.14 + hiss;
    }
    let mut drop: f32;
    let mut drop_frequency: f32;
    for beat in 0..16u32 {
        let start = (beat as usize * total) / 16 + (hash(beat + 700) as usize) % 900;
        drop_frequency = 1900.0 + hash(beat + 900) * 900.0;
        drop = 1.0;
        for offset in 0..(SAMPLE_RATE as usize / 12) {
            let index = start + offset;
            if index >= total {
                break;
            }
            drop *= 0.9993 - offset as f32 * 0.00001;
            drop_frequency *= 1.0 - 0.00012;
            samples[index] +=
                (drop_frequency * offset as f32 / SAMPLE_RATE as f32 * std::f32::consts::TAU).sin()
                    * drop
                    * 0.35;
        }
    }
    samples
}

fn battle_music() -> Vec<f32> {
    let total = (SAMPLE_RATE * SECONDS) as usize;
    let mut samples = vec![0.0f32; total];
    let beats_per_second = 132.0 / 60.0;
    let beat_length = SAMPLE_RATE as f32 / beats_per_second;
    let mut bass = 0.0f32;
    let mut hat_state = 0.0f32;
    for (index, sample) in samples.iter_mut().enumerate() {
        let beat = index as f32 / beat_length;
        let eighth = (beat * 2.0).fract();
        let time = index as f32 / SAMPLE_RATE as f32;
        bass += 55.0 / SAMPLE_RATE as f32;
        let bass_wave = bass.fract() * 2.0 - 1.0;
        let gate = if eighth < 0.55 { 1.0 } else { 0.0 };
        let kick_phase = beat.fract();
        let kick = if kick_phase < 0.25 {
            let phase = kick_phase / 0.25;
            (phase * 40.0).sin() * (1.0 - phase) * 1.2
        } else {
            0.0
        };
        let noise = hash(index as u32 ^ 0x5f3759df) * 2.0 - 1.0;
        let hat = one_pole(&mut hat_state, noise, 6000.0)
            * if (beat * 2.0).floor() as u32 % 2 == 1 {
                0.5
            } else {
                0.15
            };
        let stab = if (beat.floor() as u32) % 8 == 7 {
            (110.0 * time * std::f32::consts::TAU).sin() * 0.2
        } else {
            0.0
        };
        *sample = bass_wave * 0.22 * gate + kick * 0.7 + hat * 0.3 + stab;
    }
    samples
}

fn main() {
    let directory = Path::new("assets/audio");
    fs::create_dir_all(directory).expect("cannot create assets/audio");
    let calm_path = directory.join("music_calm.wav");
    let battle_path = directory.join("music_battle.wav");
    let calm = calm_music();
    let battle = battle_music();
    write_wav(&calm_path, &calm);
    write_wav(&battle_path, &battle);
    let calm_peak = calm
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    let battle_peak = battle
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    assert!(calm_peak > 0.2, "calm layer is silent");
    assert!(battle_peak > 0.5, "battle layer is too quiet");
    let calm_energy: f32 = calm.iter().map(|sample| sample.abs()).sum();
    let battle_energy: f32 = battle.iter().map(|sample| sample.abs()).sum();
    assert!(
        battle_energy > calm_energy,
        "battle layer must be denser than calm"
    );
    println!("wrote music_calm.wav and music_battle.wav ({SECONDS}s loops)");
}
