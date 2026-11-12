import math
import random
import struct
import wave
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "assets/audio"
RATE = 44100


def save(name, seconds, sample):
    count = round(seconds * RATE)
    values = [sample(i / RATE, i / count) for i in range(count)]
    peak = max(abs(value) for value in values) or 1
    data = b"".join(struct.pack("<h", round(max(-1, min(1, value / peak * 0.65)) * 32767)) for value in values)
    with wave.open(str(ROOT / (name + ".wav")), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(RATE)
        output.writeframes(data)


noise = random.Random(1681)
save("rivet_hot", 0.28, lambda t, p: (math.sin(2 * math.pi * (190 * t - 210 * t * t)) * 0.65 + noise.uniform(-1, 1) * math.exp(-t * 70)) * math.exp(-t * 19))
save("sand_shot", 0.32, lambda t, p: (noise.uniform(-1, 1) * math.exp(-t * 13) + math.sin(2 * math.pi * 65 * t) * math.exp(-t * 26)) * min(1, t * 1000))
save("air_blast", 0.3, lambda t, p: noise.uniform(-1, 1) * math.sin(math.pi * p) * math.exp(-t * 5))
save("plasma_discharge", 0.45, lambda t, p: (math.sin(2 * math.pi * (750 * t - 550 * t * t)) + noise.uniform(-0.5, 0.5)) * math.exp(-t * 14) * min(1, t * 1000))
save("plasma_loop", 0.2, lambda t, p: math.sin(2 * math.pi * 160 * t) * 0.6 + math.sin(2 * math.pi * 480 * t) * 0.2 + math.sin(2 * math.pi * 1100 * t) * 0.1)
save("charge_loop", 0.2, lambda t, p: math.sin(2 * math.pi * 300 * t) * 0.6 + math.sin(2 * math.pi * 600 * t) * 0.15)
print("Generated six original combat sounds")
