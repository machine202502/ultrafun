import os
import urllib.parse
import urllib.request

BASE = "https://raw.githubusercontent.com/buddingmonkey/FreeFirearmsSFXLibrary/main/"
FILES = [
    "Prepared SFX/1911/A_34P.wav",
    "Prepared SFX/1911/A_42P.wav",
    "Prepared SFX/Bersa/F_41P.wav",
    "Prepared SFX/Bersa/F_47P.wav",
    "Prepared SFX/Walther PPQ/X_31P.wav",
    "Prepared SFX/Walther PPQ/X_39P.wav",
    "Prepared SFX/Smith & Wesson 642/V_22P.wav",
    "Prepared SFX/Smith & Wesson 642/V_27P.wav",
    "Prepared SFX/Ruger Mark III/R_30P.wav",
    "Prepared SFX/Ruger Single Six/S_11P.wav",
]
os.makedirs(os.path.join("art", "audio", "raw"), exist_ok=True)
for path in FILES:
    parts = path.split("/")
    name = parts[1].replace(" ", "_") + "__" + parts[2]
    target = os.path.join("art", "audio", "raw", name)
    url = BASE + urllib.parse.quote(path)
    try:
        urllib.request.urlretrieve(url, target)
        print(name, os.path.getsize(target))
    except Exception as error:
        print("FAILED", path, error)
