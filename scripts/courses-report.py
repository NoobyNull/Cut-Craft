"""Write docs/courses.md from the fetched courses (re/courses, see scripts/courses) and the
capability table (docs/courses-capabilities.tsv).

A video teaches a capability when its title matches the capability's title pattern; it uses
one when its captions match the transcript pattern. The gap list ranks every
capability Cut Craft lacks (no) or only partly has by how many videos use it.

    python3 -I scripts/courses-report.py
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COURSES = ROOT / "re" / "courses"
TABLE = ROOT / "docs" / "courses-capabilities.tsv"
OUT = ROOT / "docs" / "courses.md"
STATUS = {"yes": "yes", "partly": "partly", "no": "no", "out": "out of scope"}


def capabilities():
    rows = []
    for line in TABLE.read_text().splitlines():
        if not line or line.startswith("#") or line.startswith("key\t"):
            continue
        cells = line.split("\t") + [""] * 7
        key, name, status, evidence, title_re, text_re, issue = cells[:7]
        assert status in STATUS, f"{key}: status {status!r}"
        rows.append(
            {
                "key": key,
                "name": name,
                "status": status,
                "evidence": evidence,
                "title": re.compile(title_re, re.I) if title_re else None,
                "text": re.compile(text_re, re.I) if text_re else None,
                "issue": issue,
            }
        )
    return rows


def cell(text):
    return text.replace("|", "\\|")


def write_progress(caps, uses):
    """re/courses/progress.json and commits.json, for the progress dashboard: every
    capability in scope with its status, video count and the commit that closed its
    issue; and every commit on this branch that names an issue."""
    import re
    import subprocess

    log = subprocess.run(
        ["git", "log", "--format=%h%x1f%cI%x1f%s%x1f%b%x1e", "ffad34d..HEAD"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout
    commits = []
    for entry in filter(str.strip, log.split("\x1e")):
        sha, at, subject, body = (entry.strip("\n").split("\x1f") + [""] * 4)[:4]
        issues = sorted({int(n) for n in re.findall(r"#(\d+)", subject + " " + body)})
        commits.append({"sha": sha, "at": at, "subject": subject, "issues": ",".join(map(str, issues))})
    closed_by = {}
    for c in reversed(commits):
        for n in filter(None, c["issues"].split(",")):
            closed_by[n] = c
    rows = []
    for c in caps:
        if c["status"] == "out":
            continue
        fix = closed_by.get(c["issue"] or "")
        rows.append({
            "key": c["key"],
            "capability": c["name"],
            "status": c["status"],
            "videos": len(uses[c["key"]]),
            "issue": int(c["issue"]) if c["issue"] else None,
            "commit": fix["sha"] if fix else "",
            "done_at": fix["at"][:10] if fix else "",
            "evidence": c["evidence"],
        })
    (COURSES / "progress.json").write_text(json.dumps(rows, indent=1))
    (COURSES / "commits.json").write_text(json.dumps(commits, indent=1))


def main():
    caps = capabilities()
    index = json.loads((COURSES / "index.json").read_text())
    uses = {c["key"]: set() for c in caps}
    titles = {}
    rows = []
    for course in index:
        for video in course["videos"]:
            vid, title = video["id"], video["title"] or "(untitled)"
            titles[vid] = title
            path = COURSES / "text" / f"{vid}.txt"
            text = path.read_text() if path.exists() else ""
            counts = {
                c["key"]: len(c["text"].findall(text)) if c["text"] else 0 for c in caps
            }
            taught = [c for c in caps if c["title"] and c["title"].search(title)]
            for c in caps:
                if counts[c["key"]] >= 1 or c in taught:
                    uses[c["key"]].add(vid)
            if not taught:
                # no title rule: the three capabilities its captions lean on most
                ranked = sorted(
                    (c for c in caps if counts[c["key"]] >= 1),
                    key=lambda c: -counts[c["key"]],
                )
                taught = ranked[:3]
            note = "" if text else " (no captions)"
            link = f"[{cell(title)}](https://www.youtube.com/watch?v={vid}){note}"
            for c in taught or [None]:
                rows.append((course["course"], link, c))

    by_key = {c["key"]: c for c in caps}
    gaps = sorted(
        (c for c in caps if c["status"] in ("no", "partly")),
        key=lambda c: (-len(uses[c["key"]]), c["status"] != "no", c["key"]),
    )
    videos = sum(len(c["videos"]) for c in index)
    unique = len({v["id"] for c in index for v in c["videos"]})
    tally = {s: sum(c["status"] == s for c in caps) for s in STATUS}

    out = [
        "# What the vendor's video courses teach, and whether Cut Craft can do it",
        "",
        "Generated by `scripts/courses-report.py` from the course captions that `scripts/courses`",
        "fetches into `re/courses/` (gitignored) and the hand-checked capability table",
        "`docs/courses-capabilities.tsv`. Edit the table, then run both scripts again.",
        "",
        f"{len(index)} courses, {videos} course entries, {unique} distinct videos. Of "
        f"{len(caps)} capabilities: {tally['yes']} yes, {tally['partly']} partly, "
        f"{tally['no']} no, {tally['out']} out of scope (laser, the vendor's online services, file "
        "formats and gadget API, release overviews).",
        "",
        "A video *teaches* a capability when its title names it; videos whose titles name none",
        "(projects such as signs and plaques) list the three capabilities their captions lean on",
        "most. *Videos* counts every video whose captions match a capability's pattern.",
        "",
        "## Gaps, most used first",
        "",
        "| Rank | Capability | Cut Craft | Videos | Issue |",
        "|---|---|---|---|---|",
    ]
    for n, c in enumerate(gaps, 1):
        issue = f"#{c['issue']}" if c["issue"] else ""
        out.append(
            f"| {n} | {cell(c['name'])} | {STATUS[c['status']]}: {cell(c['evidence'])} "
            f"| {len(uses[c['key']])} | {issue} |"
        )
    out += [
        "",
        "## Every capability",
        "",
        "| Capability | Cut Craft | Evidence or what is missing | Videos |",
        "|---|---|---|---|",
    ]
    for c in caps:
        out.append(
            f"| {cell(c['name'])} | {STATUS[c['status']]} | {cell(c['evidence'])} "
            f"| {len(uses[c['key']])} |"
        )
    out += [
        "",
        "## Every course and video",
        "",
        "| Course | Video | Capability | Cut Craft |",
        "|---|---|---|---|",
    ]
    for course, link, c in rows:
        if c is None:
            out.append(f"| {cell(course)} | {link} | (none recognised) | |")
        else:
            out.append(
                f"| {cell(course)} | {link} | {cell(c['name'])} | {STATUS[c['status']]} |"
            )
    OUT.write_text("\n".join(out) + "\n")
    ranked = [
        {
            "rank": n,
            "key": c["key"],
            "name": c["name"],
            "status": c["status"],
            "evidence": c["evidence"],
            "issue": c["issue"],
            "videos": [{"id": v, "title": titles[v]} for v in sorted(uses[c["key"]])],
        }
        for n, c in enumerate(gaps, 1)
    ]
    (COURSES / "gaps.json").write_text(json.dumps(ranked, indent=1))
    write_progress(caps, uses)
    missing = [k for k, v in uses.items() if not v and by_key[k]["status"] != "out"]
    print(f"wrote {OUT.relative_to(ROOT)}: {len(rows)} rows, {len(gaps)} gaps")
    if missing:
        print("capabilities no video matched:", ", ".join(missing), file=sys.stderr)


if __name__ == "__main__":
    main()
