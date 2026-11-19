"""Package a staged native mod as a flat Horizon .hmod archive."""
from argparse import ArgumentParser
from pathlib import Path
from tempfile import TemporaryDirectory
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

from stage_sdk import ROOT, stage


def package(project: Path, output: Path) -> None:
    for name in ("horizon-mod.toml", "Cargo.toml", "Cargo.lock", "src/lib.rs"):
        if not (project / name).is_file():
            raise ValueError(f"Missing native package file {name}")
    manifest = (project / "Cargo.toml").read_text(encoding="utf-8")
    if ".horizon/sdk/api/crates/horizon-mod-api" not in manifest:
        raise ValueError("Package a plain SDK stage without --api-source")

    roots = {"horizon-mod.toml", "Cargo.toml", "Cargo.lock", "src", "assets", "data",
             "LICENSE", "LICENSE-StorageDrawers.txt"}
    output.parent.mkdir(parents=True, exist_ok=True)
    scratch = output.with_suffix(".hmod.tmp")
    with ZipFile(scratch, "w", compression=ZIP_DEFLATED, compresslevel=6) as archive:
        for file in sorted(project.rglob("*")):
            relative = file.relative_to(project)
            if file.is_file() and relative.parts[0] in roots:
                info = ZipInfo(relative.as_posix())
                archive.writestr(info, file.read_bytes(), compress_type=ZIP_DEFLATED)
    with ZipFile(scratch) as archive:
        if bad := archive.testzip():
            raise ValueError(f"Package member did not verify: {bad}")
    scratch.replace(output)


if __name__ == "__main__":
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--staged-project", type=Path)
    args = parser.parse_args()
    if args.staged_project:
        package(args.staged_project.resolve(), args.output.resolve())
    else:
        scratch = ROOT / "scratch"
        scratch.mkdir(exist_ok=True)
        with TemporaryDirectory(dir=scratch, prefix="sdk-package-") as tmp:
            package(stage(Path(tmp)), args.output.resolve())
    print(args.output.resolve())
