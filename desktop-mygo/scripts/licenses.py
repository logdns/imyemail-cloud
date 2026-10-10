import json
import pathlib
import sys

metadata = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
destination = pathlib.Path(sys.argv[2])
packages = {package["id"]: package for package in metadata["packages"]}
nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
pending = [package["id"] for package in metadata["packages"] if package["name"] == "chck-cli"]
visited = set()
notices = []
while pending:
    package_id = pending.pop()
    if package_id in visited:
        continue
    visited.add(package_id)
    pending.extend(nodes[package_id]["dependencies"])
    package = packages[package_id]
    if package.get("source") is None:
        continue
    root = pathlib.Path(package["manifest_path"]).parent
    label = package["name"] + "-" + package["version"]
    notices.append(label + " | " + str(package.get("license")) + " | " + str(package.get("repository")))
    for source in sorted(root.iterdir()):
        if source.is_file() and not source.is_symlink() and source.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE")):
            output = destination / (label + "-" + source.name)
            output.write_bytes(source.read_bytes())
    license_file = package.get("license_file")
    if license_file:
        source = root / license_file
        if source.is_file() and not source.is_symlink() and source.resolve().is_relative_to(root.resolve()):
            (destination / (label + "-license-file.txt")).write_bytes(source.read_bytes())
(destination / "rust-dependencies.txt").write_text("\n".join(sorted(notices)) + "\n", encoding="utf-8")
