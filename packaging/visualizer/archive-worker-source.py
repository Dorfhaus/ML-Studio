"""Archive the patched AudioVisualizer worker and its pinned build dependencies."""
import argparse
from pathlib import Path
import subprocess
import sys
import zipfile

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
from private_build import public_source_bytes, require_public_artifact

DEPENDENCIES = {
    "juce": "51d11a2be6d5c97ccf12b4e5e827006e19f0555a",
    "clap-juce-extensions": "7adee3a1bd4684d4caa5601100e364abccff4b4d",
    "tinygltf": "14ba27113ebd507ebeb7cba8ecaeab8df3921b27",
}
UPSTREAM = "18758d720c1d08cdd982352604e30e6260d146fe"

def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args])

def archive(z, root, prefix, extra=()):
    names = git(root, "ls-files", "--recurse-submodules", "-z").decode().strip("\0").split("\0")
    for name in sorted(set(names).union(extra)):
        if name:
            member = prefix + name
            z.writestr(member, public_source_bytes(member, (root / name).read_bytes()))

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("upstream", type=Path)
    parser.add_argument("deps", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    upstream = args.upstream.resolve()
    if git(upstream, "rev-parse", "HEAD").decode().strip() != UPSTREAM:
        raise SystemExit("Wrong AudioVisualizer checkout")
    expected_changes = {" M CMakeLists.txt", " M src/render/Renderer.cpp", " M src/render/Renderer.h", "?? src/OfflineRender.cpp"}
    if set(git(upstream, "status", "--porcelain").decode().splitlines()) != expected_changes:
        raise SystemExit("Unexpected changes in the renderer checkout")
    git(upstream, "apply", "--reverse", "--check", "--unidiff-zero", str(HERE / "offline-render.patch"))
    if (upstream / "src/OfflineRender.cpp").read_text().rstrip() != (HERE / "OfflineRender.cpp").read_text().rstrip():
        raise SystemExit("Renderer entry point differs from the release source")
    for name, commit in DEPENDENCIES.items():
        dep = args.deps / (name + "-src")
        if git(dep, "rev-parse", "HEAD").decode().strip() != commit or git(dep, "status", "--porcelain"):
            raise SystemExit("Unexpected source in " + name)
        if any(line[:1] != b" " for line in git(dep, "submodule", "status", "--recursive").splitlines()):
            raise SystemExit("Uninitialized or changed submodule in " + name)
    with zipfile.ZipFile(args.output, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        archive(z, upstream, "AudioVisualizer/", ["src/OfflineRender.cpp"])
        for name in DEPENDENCIES:
            archive(z, args.deps / (name + "-src"), "deps/" + name + "/")
        z.writestr("README.md", (HERE / "README.md").read_bytes())
    require_public_artifact(args.output)

if __name__ == "__main__":
    main()
