#!/usr/bin/env python3
"""Refuse unless this tree's RFC mirror is exactly the corpus' own set for
this repository's ident prefix, byte-identical, at the pinned revision.

The set is derived from the corpus at the pin — every `rfc/<prefix>-*.md`
it carries — not from `provenance.json`, which is the import record and
names only the RFCs that were imported from a repository. An RFC born in
doxa carries no provenance entry and is mirrored all the same.

usage: doxa-mirror-check.py --doxa DIR --prefix P --path-template T --mirror G
       doxa-mirror-check.py --self-test    the check's own positive control
exit 0 clean, 1 findings or usage
"""
import argparse
import filecmp
import glob
import os
import shutil
import sys
import tempfile


def corpus_idents(doxa, prefix):
    rfc_dir = os.path.join(doxa, "rfc")
    if not os.path.isdir(rfc_dir):
        return None
    out = []
    for name in sorted(os.listdir(rfc_dir)):
        if name.startswith(prefix + "-") and name.endswith(".md"):
            out.append(name[: -len(".md")])
    return out


def check(doxa, prefix, path_template, mirror_glob):
    idents = corpus_idents(doxa, prefix)
    if idents is None:
        return [f"{os.path.join(doxa, 'rfc')} is absent — the corpus is unreadable at its pin, never a pass"]
    bad = []
    declared = set()
    for ident in idents:
        tail = ident[len(prefix) + 1:]
        local = path_template.format(tail=tail)
        declared.add(local)
        corpus = os.path.join(doxa, "rfc", f"{ident}.md")
        if not os.path.exists(local):
            bad.append(f"missing in mirror: {local} ({ident})")
        elif not filecmp.cmp(local, corpus, shallow=False):
            bad.append(f"diverges from corpus: {local} != rfc/{ident}.md")
    mirrored = sorted(glob.glob(mirror_glob))
    for f in mirrored:
        if f not in declared:
            bad.append(f"not carried by the corpus at the pin: {f} — the mirror is the corpus' set for the prefix {prefix}; delete the file or add the RFC to the corpus")
    if not declared and not mirrored:
        bad.append(f"nothing to check: the corpus carries no rfc/{prefix}-*.md and {mirror_glob} matches nothing")
    return bad


def report(bad, declared_count):
    if bad:
        print("FATAL: the RFC mirror is not the corpus' set at the pin — the mirror is read-only; edit the corpus, bump doxa.rev, refresh the mirror:")
        for b in bad:
            print("  " + b)
        return 1
    print(f"mirror ok: {declared_count} files byte-identical to the corpus at the pin")
    return 0


def plant(root, corpus_files, mirror_files):
    os.makedirs(os.path.join(root, "doxa", "rfc"), exist_ok=True)
    os.makedirs(os.path.join(root, "docs", "rfc"), exist_ok=True)
    for name, body in corpus_files.items():
        with open(os.path.join(root, "doxa", "rfc", name), "w", encoding="utf-8") as fh:
            fh.write(body)
    for name, body in mirror_files.items():
        with open(os.path.join(root, "docs", "rfc", name), "w", encoding="utf-8") as fh:
            fh.write(body)


def self_test():
    corpus = {
        "demo-001-first.md": "one\n",
        "demo-002-second.md": "two\n",
        "other-009-not-ours.md": "elsewhere\n",
    }
    cases = [
        ("the corpus' set mirrored byte-for-byte",
         corpus, {"001-first.md": "one\n", "002-second.md": "two\n"}, 0, ""),
        ("a corpus RFC absent from the mirror",
         dict(corpus, **{"demo-003-third.md": "three\n"}),
         {"001-first.md": "one\n", "002-second.md": "two\n"}, 1, "missing in mirror"),
        ("a mirrored file the corpus does not carry",
         corpus,
         {"001-first.md": "one\n", "002-second.md": "two\n", "004-extra.md": "four\n"},
         1, "not carried by the corpus"),
        ("a byte changed in a mirrored file",
         corpus, {"001-first.md": "one\n", "002-second.md": "two!\n"}, 1, "diverges from corpus"),
    ]
    failures = []
    cwd = os.getcwd()
    for name, corpus_files, mirror_files, want_rc, want_text in cases:
        root = tempfile.mkdtemp()
        try:
            plant(root, corpus_files, mirror_files)
            os.chdir(root)
            bad = check("doxa", "demo", "docs/rfc/{tail}.md", "docs/rfc/*.md")
            rc = 1 if bad else 0
            if rc != want_rc:
                failures.append(f"{name}: expected exit {want_rc}, got {rc} — {bad}")
            elif want_text and not any(want_text in b for b in bad):
                failures.append(f"{name}: expected a finding naming {want_text!r}, got {bad}")
        finally:
            os.chdir(cwd)
            shutil.rmtree(root, ignore_errors=True)
    if failures:
        print("doxa-mirror-check self-test FAILED — the check does not refuse what it claims to refuse:")
        for f in failures:
            print("  " + f)
        return 1
    print(f"doxa-mirror-check self-test ok: {len(cases)} plants, 1 clean and 3 refused")
    return 0


def main():
    if len(sys.argv) == 2 and sys.argv[1] == "--self-test":
        return self_test()
    ap = argparse.ArgumentParser(
        description="Refuses unless this tree's RFC mirror is exactly the corpus' set for its ident prefix, byte-identical, at the pinned revision: the corpus is the one source; the mirror is read-only — refreshed on a pin bump, never edited here."
    )
    ap.add_argument("--doxa", required=True, help="path of the doxa clone checked out at doxa.rev")
    ap.add_argument("--prefix", required=True, help="this repository's corpus ident prefix, e.g. graph-specs")
    ap.add_argument("--path-template", required=True, help="where an ident's tail lands in this tree, e.g. 'docs/rfc/{tail}.md'")
    ap.add_argument("--mirror", required=True, help="glob of the mirrored RFC files in this tree, e.g. 'docs/rfc/*.md'")
    ap.add_argument("--self-test", action="store_true", help="run the check's own positive control and exit")
    args = ap.parse_args()
    bad = check(args.doxa, args.prefix, args.path_template, args.mirror)
    idents = corpus_idents(args.doxa, args.prefix) or []
    return report(bad, len(idents))


if __name__ == "__main__":
    sys.exit(main())
