#!/usr/bin/env python3
"""
Build the medication reference SQLite from Swissmedic AIPS data.

Supports both:
  - Legacy mediXML format (pre-August 2025): inline XML content
  - New MedicinalDocuments format (August 2025+): metadata + URLs to HTML/PDF

The generated file is published as a GitHub Release asset alongside a detached
minisign signature.  The app downloads both, verifies the signature with a
hardcoded public key, and uses the SQLite for local-only autocomplete searches.

Usage (run in CI after downloading AIPS XML):
    python scripts/build_medication_ref.py \
        --aips aips_de.xml \
        --out  medication_ref_de.sqlite \
        --version "2025-03"

Dependencies: lxml, requests, beautifulsoup4
"""

import argparse
import concurrent.futures
import json
import re
import sqlite3
import sys
import time
from pathlib import Path

try:
    from lxml import etree
except ImportError:
    print("ERROR: lxml is required.  Run: pip install lxml", file=sys.stderr)
    sys.exit(1)

try:
    import requests
    from bs4 import BeautifulSoup
except ImportError:
    requests = None
    BeautifulSoup = None


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

NS = {"n": "https://simisinfo.refdata.ch/MedicinalDocuments/1.0/"}

ATC_RE = re.compile(r"\b([A-Z]\d{2}[A-Z]{2}\d{2})\b")

# German SmPC section header patterns (matched against short lines only)
SECTION_PATTERNS = {
    "composition": re.compile(
        r"Zusammensetzung|Wirkstoffe?\s+und\s+Hilfsstoffe", re.I
    ),
    "indication": re.compile(
        r"Indikationen?\s*/?\s*Anwendungsm[öo]glichkeit|Therapeutische\s+Indikation",
        re.I,
    ),
    "contraindication": re.compile(r"Kontraindikation|Gegenanzeig", re.I),
    "side_effects": re.compile(r"Unerw[üu]nschte\s+Wirkung|Nebenwirkung", re.I),
    "pharmacodynamics": re.compile(
        r"Eigenschaften\s*/?\s*Wirkung|Pharmakodynami", re.I
    ),
}

# Max concurrent HTML downloads
MAX_WORKERS = 20
DOWNLOAD_TIMEOUT = 60
MAX_RETRIES = 3


# ---------------------------------------------------------------------------
# Format detection
# ---------------------------------------------------------------------------


def detect_format(xml_path: Path) -> tuple[str, etree._ElementTree]:
    parser = etree.XMLParser(recover=True, huge_tree=True)
    tree = etree.parse(str(xml_path), parser)
    root = tree.getroot()
    tag = root.tag.split("}")[-1] if "}" in root.tag else root.tag
    if tag == "MedicinalDocumentsBundles":
        return "new", tree
    return "legacy", tree


# ===================================================================
# LEGACY FORMAT  (pre-August 2025)
# ===================================================================


def text_or_none(element, xpath: str) -> str | None:
    nodes = element.xpath(xpath)
    if not nodes:
        return None
    raw = etree.tostring(nodes[0], method="text", encoding="unicode")
    text = re.sub(r"\s+", " ", raw).strip()
    return text or None


def parse_legacy(tree: etree._ElementTree) -> list[dict]:
    root = tree.getroot()
    substances: dict[str, dict] = {}

    for med in root.xpath("//medicalInformation[@type='fi' and @lang='de']"):
        atc = text_or_none(med, "atcCode")
        trade_name = text_or_none(med, "title")

        for subst_el in med.xpath(".//substances/substance"):
            name = text_or_none(subst_el, "name")
            if not name:
                continue
            key = name.lower()
            if key not in substances:
                substances[key] = {
                    "id": key,
                    "name_de": name,
                    "atc_code": atc,
                    "trade_names": [],
                    "indication": text_or_none(
                        med, ".//paragraph[@type='section5']"
                    ),
                    "side_effects": text_or_none(
                        med, ".//paragraph[@type='section4.8']"
                    ),
                    "contraindications": text_or_none(
                        med, ".//paragraph[@type='section4.3']"
                    ),
                }
            entry = substances[key]
            if trade_name and trade_name not in entry["trade_names"]:
                entry["trade_names"].append(trade_name)
            if atc and not entry["atc_code"]:
                entry["atc_code"] = atc

    return list(substances.values())


# ===================================================================
# NEW FORMAT  (August 2025+)
# ===================================================================


def parse_new_xml(tree: etree._ElementTree) -> list[dict]:
    """
    Parse the new MedicinalDocuments XML:
      1. Collect metadata + HTML URLs for German SmPC bundles
      2. Download HTML files concurrently
      3. Parse each HTML to extract substances, ATC, clinical sections
      4. Deduplicate by substance name
    """
    if requests is None or BeautifulSoup is None:
        print(
            "ERROR: requests and beautifulsoup4 are required for the new AIPS "
            "format.  Run:  pip install requests beautifulsoup4",
            file=sys.stderr,
        )
        sys.exit(1)

    root = tree.getroot()

    # --- Step 1: collect bundles ----------------------------------------
    bundles: list[dict] = []
    for el in root.findall("n:MedicinalDocumentsBundle", NS):
        if el.findtext("n:Type", "", NS) != "SmPC":
            continue

        auth_numbers = [
            n.text.strip()
            for n in el.findall("n:RegulatedAuthorization/n:Identifier", NS)
            if n.text
        ]
        holder = (el.findtext("n:Holder/n:Name", "", NS) or "").strip()

        for doc in el.findall("n:AttachedDocument", NS):
            lang = (doc.findtext("n:Language", "", NS) or "").strip()
            if lang != "de":
                continue
            desc = (doc.findtext("n:Description", "", NS) or "").strip()
            html_url = None
            for ref in doc.findall("n:DocumentReference", NS):
                ct = (ref.findtext("n:ContentType", "", NS) or "").strip()
                if ct == "text/html":
                    html_url = (ref.findtext("n:Url", "", NS) or "").strip()
                    break
            if html_url and desc:
                bundles.append(
                    {
                        "auth_numbers": auth_numbers,
                        "holder": holder,
                        "trade_name": desc,
                        "html_url": html_url,
                    }
                )
            break  # only the first German AttachedDocument

    print(f"Found {len(bundles)} German SmPC bundles to download.")

    # --- Step 2: download & parse in parallel ---------------------------
    substances: dict[str, dict] = {}
    failed = 0
    session = requests.Session()
    adapter = requests.adapters.HTTPAdapter(
        pool_connections=MAX_WORKERS, pool_maxsize=MAX_WORKERS, max_retries=0
    )
    session.mount("https://", adapter)

    def _process(bundle: dict) -> list[dict] | None:
        for attempt in range(MAX_RETRIES):
            try:
                r = session.get(bundle["html_url"], timeout=DOWNLOAD_TIMEOUT)
                r.raise_for_status()
                r.encoding = r.apparent_encoding or "utf-8"
                return _parse_smpc_html(r.text, bundle)
            except Exception:
                if attempt < MAX_RETRIES - 1:
                    time.sleep(1.5**attempt)
        return None

    with concurrent.futures.ThreadPoolExecutor(max_workers=MAX_WORKERS) as pool:
        future_map = {pool.submit(_process, b): b for b in bundles}
        done_count = 0
        for future in concurrent.futures.as_completed(future_map):
            done_count += 1
            if done_count % 500 == 0 or done_count == len(bundles):
                print(f"  … {done_count}/{len(bundles)} processed")

            result = future.result()
            if result is None:
                failed += 1
                bundle = future_map[future]
                print(f"  WARN: failed to download {bundle['html_url']}")
                continue

            for rec in result:
                key = rec["id"]
                if key not in substances:
                    substances[key] = rec
                else:
                    existing = substances[key]
                    tn = rec.get("_trade_name")
                    if tn and tn not in existing["trade_names"]:
                        existing["trade_names"].append(tn)
                    if rec.get("atc_code") and not existing.get("atc_code"):
                        existing["atc_code"] = rec["atc_code"]
                    # Merge richer clinical text if current is empty
                    for field in ("indication", "side_effects", "contraindications"):
                        if not existing.get(field) and rec.get(field):
                            existing[field] = rec[field]

    if failed:
        print(f"  WARNING: {failed}/{len(bundles)} HTML downloads failed.")
    print(f"  Extracted {len(substances)} unique substances.")

    # Remove internal helper key before returning
    for rec in substances.values():
        rec.pop("_trade_name", None)

    return list(substances.values())


# ---------------------------------------------------------------------------
# HTML parsing helpers
# ---------------------------------------------------------------------------


def _parse_smpc_html(html: str, bundle: dict) -> list[dict]:
    """Extract substance records from one SmPC HTML document."""
    soup = BeautifulSoup(html, "html.parser")
    full_text = soup.get_text("\n")

    sections = _extract_sections(full_text)

    # --- ATC code -------------------------------------------------------
    atc = None
    pharma = sections.get("pharmacodynamics", "")
    m = ATC_RE.search(pharma)
    if m:
        atc = m.group(1)
    else:
        m = ATC_RE.search(full_text)
        if m:
            atc = m.group(1)

    # --- Substance names ------------------------------------------------
    substance_names = _extract_substances(sections.get("composition", ""))
    if not substance_names:
        # Fallback: use trade name (before first comma)
        substance_names = [bundle["trade_name"].split(",")[0].strip()]

    trade_name = bundle["trade_name"].split(",")[0].strip()

    # --- Build records --------------------------------------------------
    records: list[dict] = []
    for sname in substance_names:
        key = sname.lower().strip()
        if not key or len(key) < 2:
            continue
        records.append(
            {
                "id": key,
                "name_de": sname.strip(),
                "atc_code": atc,
                "trade_names": [trade_name],
                "_trade_name": trade_name,
                "indication": _truncate(sections.get("indication"), 2000),
                "side_effects": _truncate(sections.get("side_effects"), 3000),
                "contraindications": _truncate(
                    sections.get("contraindication"), 2000
                ),
            }
        )
    return records


def _extract_sections(text: str) -> dict[str, str]:
    """Split SmPC full text into named sections by detecting headers."""
    lines = text.split("\n")
    sections: dict[str, str] = {}
    current: str | None = None
    buf: list[str] = []

    for line in lines:
        stripped = line.strip()
        if not stripped:
            if current:
                buf.append("")
            continue

        # Only treat short lines as potential section headers
        matched = None
        if len(stripped) < 120:
            for name, pattern in SECTION_PATTERNS.items():
                if pattern.search(stripped):
                    matched = name
                    break

        if matched:
            if current:
                sections[current] = "\n".join(buf).strip()
            current = matched
            buf = []
        elif current:
            buf.append(stripped)

    if current:
        sections[current] = "\n".join(buf).strip()

    return sections


def _extract_substances(composition_text: str) -> list[str]:
    """Best-effort extraction of active substance names from the composition section."""
    if not composition_text:
        return []

    names: list[str] = []

    # Pattern: "Wirkstoff(e): SubstanceName ..."
    m = re.search(
        r"Wirkstoff(?:e|\(e\))?\s*:\s*(.+?)(?:\n\s*\n|Hilfsstoff|$)",
        composition_text,
        re.I | re.DOTALL,
    )
    if m:
        raw = m.group(1).strip()
        for part in re.split(r"[,;]\s*", raw):
            # Strip dosage info  (e.g. "123.4 mg", "0,05 g", etc.)
            part = re.sub(
                r"\s+\d[\d.,]*\s*(?:mg|g|ml|µg|IE|UI|mmol|Mio\.?)(?:\b|$).*",
                "",
                part,
                flags=re.I,
            )
            part = part.strip().rstrip(".")
            if part and len(part) > 2:
                names.append(part)
        return names

    # Fallback: look for "Praeparatio" lines (Latin compound names)
    for line in composition_text.split("\n"):
        line = line.strip()
        if re.match(r"^[A-Z][a-z].*(?:um|is|as|us|ol)\b", line):
            # Likely a Latin substance name
            name = re.sub(
                r"\s+\d[\d.,]*\s*(?:mg|g|ml|µg|IE|UI|mmol).*",
                "",
                line,
                flags=re.I,
            ).strip()
            if name and len(name) > 3:
                names.append(name)
                break  # take only first match for safety

    return names


# ---------------------------------------------------------------------------
# SQLite writer  (unchanged schema for app compatibility)
# ---------------------------------------------------------------------------


def write_sqlite(records: list[dict], out_path: Path, version: str) -> None:
    if out_path.exists():
        out_path.unlink()

    conn = sqlite3.connect(str(out_path))
    conn.execute("PRAGMA journal_mode = DELETE;")
    conn.execute("PRAGMA synchronous = FULL;")

    conn.executescript(
        """
        CREATE TABLE substances (
            id                TEXT PRIMARY KEY NOT NULL,
            name_de           TEXT NOT NULL,
            atc_code          TEXT,
            trade_names       TEXT,
            indication        TEXT,
            side_effects      TEXT,
            contraindications TEXT,
            source_version    TEXT
        );

        CREATE VIRTUAL TABLE substances_fts USING fts5(
            name_de,
            trade_names,
            content='substances',
            content_rowid='rowid',
            tokenize='unicode61 remove_diacritics 1'
        );
    """
    )

    for rec in records:
        conn.execute(
            """INSERT INTO substances
               (id, name_de, atc_code, trade_names,
                indication, side_effects, contraindications, source_version)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)""",
            (
                rec["id"],
                rec["name_de"],
                rec.get("atc_code"),
                json.dumps(rec.get("trade_names", []), ensure_ascii=False),
                _truncate(rec.get("indication"), 2000),
                _truncate(rec.get("side_effects"), 3000),
                _truncate(rec.get("contraindications"), 2000),
                version,
            ),
        )

    conn.execute(
        """INSERT INTO substances_fts(rowid, name_de, trade_names)
           SELECT rowid, name_de, COALESCE(trade_names, '') FROM substances"""
    )

    conn.commit()
    conn.execute("PRAGMA optimize;")
    conn.execute("VACUUM;")
    conn.close()

    size_kb = out_path.stat().st_size // 1024
    print(
        f"Wrote {len(records)} substances to '{out_path}' "
        f"({size_kb} KB, version={version})"
    )


def _truncate(text: str | None, max_len: int) -> str | None:
    if text is None:
        return None
    if len(text) <= max_len:
        return text
    return text[:max_len].rsplit(" ", 1)[0] + "…"


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--aips", required=True, help="Path to AIPS XML file (e.g. aips_de.xml)"
    )
    ap.add_argument(
        "--out", default="medication_ref_de.sqlite", help="Output SQLite path"
    )
    ap.add_argument(
        "--version", required=True, help="Source version string (e.g. 2025-03)"
    )
    args = ap.parse_args()

    aips_path = Path(args.aips)
    if not aips_path.exists():
        print(f"ERROR: AIPS XML not found: {aips_path}", file=sys.stderr)
        sys.exit(1)

    fmt, tree = detect_format(aips_path)
    print(f"Detected format: {fmt}")

    if fmt == "new":
        records = parse_new_xml(tree)
    else:
        records = parse_legacy(tree)

    print(f"Found {len(records)} unique substances.")

    if len(records) < 50:
        print("ERROR: Too few substances — something likely went wrong.", file=sys.stderr)
        sys.exit(1)

    write_sqlite(records, Path(args.out), args.version)


if __name__ == "__main__":
    main()
