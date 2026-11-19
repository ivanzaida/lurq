"""Builds the gallery as a page of a browser, into target/gallery-web.

    python scripts/gallery-web.py            build
    python scripts/gallery-web.py --serve    build, then serve on http://127.0.0.1:8123
    python scripts/gallery-web.py --dev      an unoptimised build: quicker to build, slow to run

Needs the wasm32 target and wasm-bindgen's CLI at the version of the `wasm-bindgen` crate in Cargo.lock:

    rustup target add wasm32-unknown-unknown
    cargo install wasm-bindgen-cli --version <that version> --locked

The page fetches its fonts from fonts/ beside it, since a page cannot read the system's. They are the gallery's
own, crates/gallery/web/fonts: Inter and JetBrains Mono, under the SIL Open Font License, which a page put on a
server may ship. `--fonts DIR` copies another directory that has the font files and an index.txt (see
crates/gallery/src/web.rs). `--system-fonts`, on Windows, copies the system's Segoe UI and Consolas, so the page
looks as the desktop gallery does there: those are Microsoft's and are not to be published.

The build is cargo's `web` profile (the workspace's Cargo.toml), chosen for what a page downloads; `--dev` is an
unoptimised one.
"""

import argparse
import functools
import http.server
import os
import pathlib
import re
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "target" / "gallery-web"
PAGE = ROOT / "crates" / "gallery" / "web"
TARGET = "wasm32-unknown-unknown"
# Files of the gallery's that its pages fetch beside the page, as a desktop reads them from its disk: where they
# are under crates/gallery, and where under the page.
PAGE_FILES = [
    ("assets/media/track.m4a", "media/track.m4a"),
]

# The fonts the desktop gallery draws with on Windows, and the symbols Segoe UI leaves to another font.
WINDOWS_FONTS = [
    "segoeui.ttf",
    "segoeuii.ttf",
    "seguisb.ttf",
    "segoeuib.ttf",
    "segoeuiz.ttf",
    "consola.ttf",
    "consolab.ttf",
    "seguisym.ttf",
]
WINDOWS_FAMILIES = {"sans-serif": "Segoe UI", "monospace": "Consolas"}


def run(*command):
    print("$", " ".join(str(part) for part in command), flush=True)
    subprocess.run(command, cwd=ROOT, check=True)


def locked_wasm_bindgen():
    lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
    found = re.search(r'name = "wasm-bindgen"\nversion = "([^"]+)"', lock)
    return found.group(1) if found else None


def check_tools():
    cli = shutil.which("wasm-bindgen")
    wanted = locked_wasm_bindgen()
    if cli is None:
        sys.exit(f"wasm-bindgen is not installed: cargo install wasm-bindgen-cli --version {wanted} --locked")
    have = subprocess.run([cli, "--version"], capture_output=True, text=True, check=True).stdout.split()[-1]
    if wanted and have != wanted:
        sys.exit(
            f"wasm-bindgen {have} is installed and Cargo.lock has {wanted}: "
            f"cargo install wasm-bindgen-cli --version {wanted} --locked"
        )


def build(dev):
    profile = "dev" if dev else "web"
    run("cargo", "build", "-p", "gallery", "--target", TARGET, "--profile", profile)
    wasm = ROOT / "target" / TARGET / ("debug" if dev else profile) / "gallery.wasm"
    OUT.mkdir(parents=True, exist_ok=True)
    # The names of the module's functions are for a debugger: 4 MB a page does not need.
    names = [] if dev else ["--remove-name-section"]
    run("wasm-bindgen", "--target", "web", "--no-typescript", *names, "--out-dir", OUT, "--out-name", "gallery", wasm)
    for file in PAGE.iterdir():
        if file.is_file():
            shutil.copy2(file, OUT / file.name)
    for source, there in PAGE_FILES:
        (OUT / there).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(PAGE.parent / source, OUT / there)


def copy_fonts(source, system):
    fonts = OUT / "fonts"
    if fonts.exists():
        shutil.rmtree(fonts)
    if not system:
        source = source or PAGE / "fonts"
        if not (source / "index.txt").is_file():
            sys.exit(f"{source} has no index.txt")
        shutil.copytree(source, fonts)
        return
    if os.name != "nt":
        sys.exit("--system-fonts knows only Windows' own")
    system = pathlib.Path(os.environ.get("WINDIR", r"C:\Windows")) / "Fonts"
    fonts.mkdir(parents=True)
    lines = [f"{generic} = {family}" for generic, family in WINDOWS_FAMILIES.items()]
    for name in WINDOWS_FONTS:
        if (system / name).is_file():
            shutil.copy2(system / name, fonts / name)
            lines.append(name)
    (fonts / "index.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")


def serve(port):
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(OUT))
    handler.extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm"}
    with http.server.ThreadingHTTPServer(("127.0.0.1", port), handler) as server:
        print(f"http://127.0.0.1:{port}/", flush=True)
        server.serve_forever()


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--dev", action="store_true", help="an unoptimised build")
    parser.add_argument("--fonts", type=pathlib.Path, help="a directory of fonts with an index.txt")
    parser.add_argument("--system-fonts", action="store_true", help="Windows' Segoe UI and Consolas, not to publish")
    parser.add_argument("--serve", action="store_true", help="serve the page after building it")
    parser.add_argument("--no-build", action="store_true", help="serve what is already built")
    parser.add_argument("--port", type=int, default=8123)
    args = parser.parse_args()
    if not args.no_build:
        check_tools()
        build(args.dev)
        copy_fonts(args.fonts, args.system_fonts)
    if args.serve or args.no_build:
        serve(args.port)


if __name__ == "__main__":
    main()
