"""Score blinded listener trials for Phase 6. No estimated MOS or speaker labels."""
import argparse
import json
from pathlib import Path
import re
from statistics import mean
import unicodedata


def words(text):
    return re.findall(r"[^\W_]+", unicodedata.normalize("NFC", text).casefold())


def word_errors(reference, transcript):
    scoring = list(range(len(transcript) + 1))
    for row, source in enumerate(reference, 1):
        following = [row]
        for col, item in enumerate(transcript, 1):
            following.append(min(
                scoring[col] + 1, following[-1] + 1, scoring[col - 1] + (source != item)
            ))
        scoring = following
    return scoring[-1]


def score(key, trials):
    if not key or not trials:
        raise ValueError("Provide a blind sample key and completed listening trials")
    aggregates = {}
    seen = set()
    for trial in trials:
        clip = key[trial["sampleId"]]
        identity = (trial["listenerId"], trial["sampleId"])
        if identity in seen:
            raise ValueError("Duplicate trial from the same listener")
        seen.add(identity)
        reference = words(clip["reference"])
        rating = trial["naturalness"]
        if not reference or type(rating) is not int or not 1 <= rating <= 5:
            raise ValueError("Reference transcript and integer naturalness from 1 to 5 are required")
        if clip["profileId"] not in ("wind-assassin", "jade-guardian"):
            raise ValueError("Unknown target identity")
        if trial["guess"] not in ("wind-assassin", "jade-guardian", "unsure"):
            raise ValueError("Unrecognized blind identity guess")
        group = aggregates.setdefault((clip["profileId"], clip["language"]), [])
        group.append((word_errors(reference, words(trial["transcript"])), len(reference),
                      rating, trial["guess"] == clip["profileId"], trial["listenerId"]))
    return [
        dict(profileId=profile, language=language, trials=len(results),
             listeners=len({row[4] for row in results}),
             wordErrorRate=sum(row[0] for row in results) / sum(row[1] for row in results),
             meanNaturalness=mean(row[2] for row in results),
             identificationAccuracy=mean(row[3] for row in results))
        for (profile, language), results in sorted(aggregates.items())
    ]


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("key", type=Path)
    parser.add_argument("trials", type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(score(json.loads(args.key.read_text("utf-8")),
                               json.loads(args.trials.read_text("utf-8"))), indent=2))
    except (ValueError, KeyError, OSError) as error:
        parser.exit(1, f"Voice benchmark: {error}\n")
