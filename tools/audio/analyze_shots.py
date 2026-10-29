import glob
import os
import wave

import numpy as np

SAMPLE_RATE = 44100


def read(path):
    with wave.open(path) as file:
        frames = file.readframes(file.getnframes())
    return np.frombuffer(frames, dtype="<i2").astype(np.float32) / 32768.0


def onsets(signal, threshold=0.18, gap_seconds=0.18):
    window = int(SAMPLE_RATE * 0.004)
    envelope = np.convolve(np.abs(signal), np.ones(window) / window, mode="same")
    limit = envelope.max() * threshold
    gap = int(SAMPLE_RATE * gap_seconds)
    found = []
    index = 0
    while index < len(envelope):
        if envelope[index] >= limit:
            found.append(index)
            index += gap
        else:
            index += 1
    return found, envelope


def describe(signal, start):
    end = min(start + int(SAMPLE_RATE * 0.5), len(signal))
    chunk = signal[start:end]
    peak = float(np.abs(chunk).max())
    rms = float(np.sqrt(np.mean(chunk**2)))
    spectrum = np.abs(np.fft.rfft(chunk))
    frequencies = np.fft.rfftfreq(len(chunk), 1.0 / SAMPLE_RATE)
    centroid = float((spectrum * frequencies).sum() / spectrum.sum())
    return peak, peak / rms if rms > 0 else 0.0, centroid


for path in sorted(glob.glob("art/audio/mono/*.wav")):
    signal = read(path)
    starts, envelope = onsets(signal)
    name = os.path.basename(path).replace(".wav", "")
    best = []
    for start in starts:
        peak, crest, centroid = describe(signal, start)
        if peak < 0.2:
            continue
        best.append((peak, crest, centroid, start / SAMPLE_RATE))
    best.sort(key=lambda item: -item[0])
    top = best[:4]
    print(name, f"{len(signal) / SAMPLE_RATE:.1f}s", len(best), "loud:")
    for peak, crest, centroid, second in top:
        print(f"   t={second:.2f}s peak {peak:.2f} crest {crest:.1f} centroid {centroid:.0f}Hz")
