import os
import wave

import numpy as np

SAMPLE_RATE = 44100
CHOSEN = [
    ("1911__A_42P", 0.94, "1911 .45, классический глубокий"),
    ("Bersa__F_41P", 0.44, "Bersa .380, резкий"),
    ("Walther_PPQ__X_31P", 1.08, "Walther PPQ 9mm, чёткий"),
    ("Smith_&_Wesson_642__V_22P", 0.67, "S&W 642, револьверный бас"),
    ("Ruger_Single_Six__S_11P", 0.60, "Ruger Single Six, громкий треск"),
]


def read(path):
    with wave.open(path) as file:
        frames = file.readframes(file.getnframes())
    return np.frombuffer(frames, dtype="<i2").astype(np.float32) / 32768.0


def write(path, signal):
    data = (np.clip(signal, -1.0, 1.0) * 32767.0).astype("<i2")
    with wave.open(path, "wb") as file:
        file.setnchannels(1)
        file.setsampwidth(2)
        file.setframerate(SAMPLE_RATE)
        file.writeframes(data.tobytes())


for index, (name, second, description) in enumerate(CHOSEN, start=1):
    source = read(os.path.join("art", "audio", "mono", name + ".wav"))
    start = max(0, int((second - 0.01) * SAMPLE_RATE))
    count = int(SAMPLE_RATE * 0.5)
    chunk = source[start : start + count]
    fade = np.ones(len(chunk))
    fade_length = int(SAMPLE_RATE * 0.08)
    fade[-fade_length:] = np.linspace(1.0, 0.0, fade_length) ** 2
    chunk = chunk * fade
    chunk = chunk / np.abs(chunk).max() * 0.95
    target = os.path.join("assets", "audio", f"shot_{index}.wav")
    write(target, chunk)
    rms = float(np.sqrt(np.mean(chunk**2)))
    print(f"shot_{index}: {description} | {len(chunk) / SAMPLE_RATE:.2f}s crest {np.abs(chunk).max() / rms:.1f}")
