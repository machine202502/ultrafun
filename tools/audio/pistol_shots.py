import os
import wave

import numpy as np

SAMPLE_RATE = 44100
OUTPUT_DIRECTORY = os.path.join("assets", "audio")


def noise(rng, count, low_hz, high_hz):
    spectrum = np.fft.rfft(rng.uniform(-1.0, 1.0, count))
    frequencies = np.fft.rfftfreq(count, 1.0 / SAMPLE_RATE)
    keep = (frequencies >= low_hz) & (frequencies <= high_hz)
    signal = np.fft.irfft(spectrum * keep, count)
    peak = np.abs(signal).max()
    return signal / peak if peak > 0.0 else signal


def envelope(count, attack, decay):
    time = np.arange(count) / SAMPLE_RATE
    return (1.0 - np.exp(-time / attack)) * np.exp(-time / decay)


def sweep(count, start_hz, end_hz, decay, bend=1.0):
    time = np.arange(count) / SAMPLE_RATE
    shape = (1.0 - time / (count / SAMPLE_RATE)) ** bend
    frequency = start_hz + (end_hz - start_hz) * (1.0 - shape)
    phase = 2.0 * np.pi * np.cumsum(frequency) / SAMPLE_RATE
    return np.sin(phase) * np.exp(-time / decay)


def filtered(signal, low_hz, high_hz):
    spectrum = np.fft.rfft(signal)
    frequencies = np.fft.rfftfreq(len(signal), 1.0 / SAMPLE_RATE)
    keep = (frequencies >= low_hz) & (frequencies <= high_hz)
    return np.fft.irfft(spectrum * keep, len(signal))


def room(signal, taps):
    out = np.zeros(len(signal) + int(SAMPLE_RATE * 0.1))
    out[: len(signal)] += signal
    for delay, gain in taps:
        offset = int(SAMPLE_RATE * delay)
        out[offset : offset + len(signal)] += filtered(signal, 120.0, 5000.0) * gain
    return out


def build(seed, duration, boom, body, pow_, crack, tail, ring, saturation):
    rng = np.random.default_rng(seed)
    count = int(SAMPLE_RATE * duration)
    time = np.arange(count) / SAMPLE_RATE

    boom_signal = sweep(count, boom[0], boom[1], boom[2], 2.5) * 0.75
    body_signal = noise(rng, count, 120.0, 900.0) * envelope(count, 0.0015, body)
    pow_signal = noise(rng, count, 300.0, 2600.0) * envelope(count, 0.0008, pow_)
    crack_signal = noise(rng, count, 2600.0, 12000.0) * envelope(count, 0.0003, crack)

    tail_signal = np.zeros(count)
    tail_count = count - int(SAMPLE_RATE * 0.012)
    if tail_count > 0:
        tail_signal[int(SAMPLE_RATE * 0.012) :] = noise(rng, tail_count, 120.0, 2400.0) * envelope(
            tail_count, 0.004, tail
        )

    ring_signal = np.zeros(count)
    if ring > 0.0:
        for frequency, gain, decay in (
            (1500.0, 1.0, 0.1),
            (2600.0, 0.6, 0.07),
            (3900.0, 0.35, 0.05),
        ):
            ring_signal += np.sin(2.0 * np.pi * frequency * time) * np.exp(-time / decay) * gain
        ring_signal *= ring

    mix = boom_signal * 0.5 + body_signal * 1.0 + pow_signal * 1.0 + crack_signal * 0.8
    mix += tail_signal * 0.45 + ring_signal * 0.3
    mix = filtered(mix, 35.0, 11000.0)
    mix = mix / np.abs(mix).max()
    mix = np.tanh(mix * saturation)
    mix += tail_signal * 0.25
    fade = np.clip((len(mix) - np.arange(len(mix))) / (SAMPLE_RATE * 0.02), 0.0, 1.0)
    mix *= fade
    peak = np.abs(mix).max()
    return mix / peak * 0.95

VARIANTS = {
    "shot_1": dict(
        seed=101,
        duration=0.42,
        boom=(150.0, 46.0, 0.07),
        body=0.06,
        pow_=0.025,
        crack=0.01,
        tail=0.16,
        ring=0.0,
        saturation=1.4,
    ),
    "shot_2": dict(
        seed=211,
        duration=0.55,
        boom=(120.0, 38.0, 0.1),
        body=0.1,
        pow_=0.035,
        crack=0.014,
        tail=0.26,
        ring=0.0,
        saturation=1.8,
    ),
    "shot_3": dict(
        seed=307,
        duration=0.45,
        boom=(170.0, 55.0, 0.06),
        body=0.045,
        pow_=0.02,
        crack=0.007,
        tail=0.14,
        ring=0.9,
        saturation=1.5,
    ),
    "shot_4": dict(
        seed=401,
        duration=0.65,
        boom=(140.0, 40.0, 0.12),
        body=0.12,
        pow_=0.04,
        crack=0.018,
        tail=0.34,
        ring=0.5,
        saturation=2.3,
    ),
    "shot_5": dict(
        seed=509,
        duration=0.5,
        boom=(95.0, 32.0, 0.09),
        body=0.08,
        pow_=0.03,
        crack=0.009,
        tail=0.2,
        ring=0.3,
        saturation=1.9,
    ),
}

def write(path, samples):
    data = (np.clip(samples, -1.0, 1.0) * 32767.0).astype("<i2")
    with wave.open(path, "wb") as file:
        file.setnchannels(1)
        file.setsampwidth(2)
        file.setframerate(SAMPLE_RATE)
        file.writeframes(data.tobytes())


def main():
    os.makedirs(OUTPUT_DIRECTORY, exist_ok=True)
    for name, settings in VARIANTS.items():
        samples = build(**settings)
        write(os.path.join(OUTPUT_DIRECTORY, f"{name}.wav"), samples)
        spectrum = np.abs(np.fft.rfft(samples)) ** 2
        frequencies = np.fft.rfftfreq(len(samples), 1.0 / SAMPLE_RATE)
        total = spectrum.sum()
        low = spectrum[frequencies < 200.0].sum() / total
        mid = spectrum[(frequencies >= 200.0) & (frequencies < 1500.0)].sum() / total
        high = spectrum[frequencies >= 1500.0].sum() / total
        rms = float(np.sqrt(np.mean(samples**2)))
        crest = float(np.abs(samples).max() / rms)
        print(
            f"{name}: {len(samples) / SAMPLE_RATE:.2f}s crest {crest:.1f} "
            f"low {low:.2f} mid {mid:.2f} high {high:.2f}"
        )


if __name__ == "__main__":
    main()
