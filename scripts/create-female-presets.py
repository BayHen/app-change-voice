"""Deterministic authored DSP profiles; names describe tonal starting points, not speaker identities."""
import json
from pathlib import Path
rows = [
    ("Natural Female",2.0,1.2,-0.5,0.2,0.15,1.15,"Balanced resonance with restrained lift."),
    ("Soft Female",1.4,1.0,0.8,-1.8,0.35,1.4,"Warm lower mids and softened sibilants."),
    ("Sweet Female",3.0,1.8,-0.8,0.7,0.25,1.3,"Light resonance and gentle presence."),
    ("Cute Girl",4.5,2.4,-1.8,0.9,0.3,1.25,"Stylized higher register; not a natural-voice claim."),
    ("Young Woman",2.6,1.5,-0.6,0.4,0.2,1.2,"Moderate lift with open upper mids."),
    ("Mature Female",0.8,0.5,1.2,-0.4,0.2,1.6,"Lower register, weight and controlled dynamics."),
    ("Deep Female",-1.0,0.4,2.0,-0.8,0.15,1.45,"Low pitch with independently lifted resonance."),
    ("Elegant Female",1.6,1.3,0.4,0.3,0.3,1.35,"Restrained pitch, clear resonance, light leveling."),
    ("Gentle Female",1.2,0.8,1.0,-1.0,0.4,1.25,"Subtle lift and soft high frequencies."),
    ("Bright Female",2.8,1.7,-1.0,1.6,0.45,1.3,"Presence lift with sibilant control."),
    ("Anime Girl",5.0,2.8,-2.0,1.2,0.35,1.4,"Expressive character register; intentionally stylized."),
    ("Kawaii Voice",5.7,3.2,-2.5,0.5,0.45,1.35,"Small bright character resonance; intentionally stylized."),
    ("Korean Style",2.1,1.1,0.2,0.8,0.35,1.5,"Smooth pop-style tone only; does not change accent or language."),
    ("Japanese Style",3.2,2.0,-0.7,0.3,0.3,1.3,"Light character tone only; accent is not generated."),
    ("Vietnamese Female",1.0,0.6,0.0,0.0,0.1,1.1,"Conservative tonal starting point; Vietnamese tones and accents are untested."),
    ("Female Streamer",2.3,1.3,-1.2,0.9,0.4,1.8,"Forward conversational tone with light leveling."),
    ("Female Narrator",1.1,0.7,1.4,0.2,0.3,1.9,"Weighted low mids and even spoken dynamics."),
    ("Female ASMR",0.5,0.9,1.6,-2.2,0.55,1.05,"Soft intimate tone; no fabricated whisper or breathing."),
    ("Female Singer",2.4,1.6,0.6,0.6,0.2,1.15,"Wide-dynamics singing starting point; not a singing model."),
    ("Female Studio",1.8,1.4,-0.2,0.5,0.5,2.0,"Controlled sibilance and studio-style leveling."),
]
presets=[]
for name,pitch,formant,bass,treble,deess,ratio,description in rows:
    presets.append(dict(schemaVersion=2,id="female-"+name.lower().replace("female ","").replace(" ","-"),name=name,
        description=description+" Classic DSP approximation; result depends on the input voice.",category="Female",
        pitchSemitones=pitch,formantSemitones=formant,gain={"makeupDb":0},effects=[],builtIn=True,
        parameters=dict(inputGainPercent=100,outputGainPercent=100,bassDb=bass,trebleDb=treble,
            noiseSuppression=False,suppressionStrength=0.5,noiseGate=False,gateThresholdDbfs=-52,
            limiterEnabled=True,limiterCeilingDbfs=-1,effectBypassed=False,voiceIntensity=1,
            deEssStrength=deess,compressionRatio=ratio)))
destination=Path(__file__).resolve().parents[1]/"src-tauri/presets/female.json"
destination.write_text(json.dumps(presets,indent=2)+"\n",encoding="utf8")
print(destination)
