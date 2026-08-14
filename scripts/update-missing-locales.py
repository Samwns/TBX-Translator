#!/usr/bin/env python3
"""Fill only missing locale keys using the Portuguese locale as the source."""

import concurrent.futures
import json
import pathlib
import time
import urllib.parse
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
LOCALES = ROOT / "locales"
SEPARATOR_TOKEN = "\ue000\ue001\ue002"
SEPARATOR = f"\n{SEPARATOR_TOKEN}\n"


def translate_missing(path: pathlib.Path, source: dict[str, str]) -> tuple[str, int]:
    language = path.stem
    current = json.loads(path.read_text(encoding="utf-8"))
    missing = [key for key in source if key not in current]
    if not missing or language == "pt":
        return language, 0

    payload = SEPARATOR.join(source[key] for key in missing)
    query = urllib.parse.urlencode(
        {"client": "gtx", "sl": "pt", "tl": language, "dt": "t", "q": payload}
    )
    url = "https://translate.googleapis.com/translate_a/single?" + query
    last_error = None
    for attempt in range(3):
        try:
            request = urllib.request.Request(
                url, headers={"User-Agent": "TBX-Translator locale updater"}
            )
            with urllib.request.urlopen(request, timeout=30) as response:
                data = json.loads(response.read().decode("utf-8"))
            translated = "".join(segment[0] for segment in data[0] if segment[0])
            pieces = translated.split(SEPARATOR_TOKEN)
            if len(pieces) != len(missing):
                # A few language models rewrite even private-use separators.
                # Fall back to individual requests only for that language.
                pieces = []
                for key in missing:
                    single_query = urllib.parse.urlencode(
                        {"client": "gtx", "sl": "pt", "tl": language, "dt": "t", "q": source[key]}
                    )
                    single_request = urllib.request.Request(
                        "https://translate.googleapis.com/translate_a/single?" + single_query,
                        headers={"User-Agent": "TBX-Translator locale updater"},
                    )
                    with urllib.request.urlopen(single_request, timeout=30) as single_response:
                        single_data = json.loads(single_response.read().decode("utf-8"))
                    pieces.append("".join(segment[0] for segment in single_data[0] if segment[0]))
                    time.sleep(0.1)
            for key, value in zip(missing, pieces):
                current[key] = value.strip("\r\n ")
            path.write_text(
                json.dumps(current, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            return language, len(missing)
        except Exception as error:
            last_error = error
            time.sleep(0.75 * (2**attempt))
    raise RuntimeError(f"{language}: {last_error}")


def main() -> None:
    source = json.loads((LOCALES / "pt.json").read_text(encoding="utf-8"))
    paths = sorted(LOCALES.glob("*.json"))
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
        futures = [executor.submit(translate_missing, path, source) for path in paths]
        for future in concurrent.futures.as_completed(futures):
            try:
                language, count = future.result()
                if count:
                    print(f"{language}: {count} novas traduções")
            except Exception as error:
                print(f"aviso: {error}")


if __name__ == "__main__":
    main()
