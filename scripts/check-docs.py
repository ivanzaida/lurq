"""Check internal links and anchors after building the documentation site."""

from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urljoin, urlparse


SITE = "https://ivanzaida.github.io"
BASE = "/lurq/"
DIST = Path(__file__).resolve().parents[1] / "docs" / "dist"


class Page(HTMLParser):
    def __init__(self, source):
        super().__init__()
        self.links = []
        self.ids = set()
        self.feed(source)

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            self.ids.add(attrs["id"])
        if tag == "a" and attrs.get("href"):
            self.links.append(attrs["href"])


def main():
    if not (DIST / "index.html").is_file():
        raise SystemExit("Build the documentation first: cd docs && yarn build")

    pages = {
        path.relative_to(DIST).as_posix(): Page(path.read_text(encoding="utf-8"))
        for path in DIST.rglob("*.html")
    }
    failures = []
    checked = 0
    for name, page in pages.items():
        route = name.removesuffix("index.html")
        for href in page.links:
            target = urlparse(urljoin(SITE + BASE + route, href))
            if target.netloc != urlparse(SITE).netloc or not target.path.startswith(BASE):
                continue
            checked += 1
            relative = unquote(target.path[len(BASE):])
            if not relative or relative.endswith("/"):
                relative += "index.html"
            destination = (DIST / relative).resolve()
            if not destination.is_relative_to(DIST) or not destination.is_file():
                failures.append(f"{name}: {href} (missing target)")
            elif target.fragment and relative in pages:
                if unquote(target.fragment) not in pages[relative].ids:
                    failures.append(f"{name}: {href} (missing anchor)")

    for failure in failures:
        print(failure)
    print(f"Checked {checked} internal links across {len(pages)} HTML pages; {len(failures)} failures.")
    return bool(failures)


if __name__ == "__main__":
    raise SystemExit(main())
