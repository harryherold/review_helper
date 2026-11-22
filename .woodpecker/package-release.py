import hashlib
import os
import re
import sys
import tarfile
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile


ARCHIVE_NAMES = {
    "x86_64-unknown-linux-gnu": "review-helper-Linux-gnu-x86_64",
    "x86_64-apple-darwin": "review-helper-macOS-x86_64",
    "x86_64-pc-windows-msvc": "review-helper-Windows-msvc-x86_64",
}


def release_notes(changelog, tag):
    headings = list(re.finditer(r"^##[ \t]+([^\n]+)", changelog, re.MULTILINE))
    version = re.escape(tag)
    heading = rf"(?:\[{version}\]|{version})(?:\s|$)"
    for index, match in enumerate(headings):
        if re.match(heading, match[1]):
            end = headings[index + 1].start() if index + 1 < len(headings) else len(changelog)
            notes = changelog[match.end():end].strip().removesuffix("\n---").rstrip()
            if notes:
                return notes
    return "See CHANGELOG.md in the release archive."


def main():
    target = sys.argv[1]
    windows = target.endswith("-windows-msvc")
    executable = "review-helper.exe" if windows else "review-helper"
    binary = Path("target") / target / "release" / executable
    files = [binary, Path("README.md"), Path("CHANGELOG.md")]
    for path in files:
        if not path.is_file():
            raise FileNotFoundError(path)

    dist = Path("dist")
    dist.mkdir(exist_ok=True)
    archive_name = ARCHIVE_NAMES[target]
    if windows:
        archive_path = dist / f"{archive_name}.zip"
        with ZipFile(archive_path, "w", compression=ZIP_DEFLATED) as archive:
            for path in files:
                archive.write(path, path.name)
    else:
        archive_path = dist / f"{archive_name}.tar.gz"
        with tarfile.open(archive_path, "w:gz") as archive:
            for path in files:
                archive.add(path, path.name)

    checksum = hashlib.sha256()
    with archive_path.open("rb") as archived:
        for chunk in iter(lambda: archived.read(1024 * 1024), b""):
            checksum.update(chunk)
    checksum_path = archive_path.with_name(f"{archive_path.name}.sha256")
    checksum_path.write_bytes(f"{checksum.hexdigest()}  {archive_path.name}\n".encode("ascii"))

    changelog = Path("CHANGELOG.md").read_text(encoding="utf-8")
    notes = release_notes(changelog, os.environ["CI_COMMIT_TAG"])
    (dist / "release-notes.md").write_text(notes + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
