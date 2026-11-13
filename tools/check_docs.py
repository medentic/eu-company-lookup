"""Validate repository Markdown links, or links within a Cargo package file list.

External URLs are not fetched. Source files and directories must exist; Markdown
fragments must resolve to headings. GitHub links into this repository are checked
against the checkout. Relative package links may not point to excluded files.
"""
import argparse
from pathlib import Path
import re
import subprocess
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY_PATH = '/medentic/eu-company-lookup/blob/main/'
LINK = re.compile(r'\[[^\]]+\]\(([^)]+)\)')


def prose(text):
    return re.sub(r'^```[^\n]*\n.*?^```\s*$', '', text, flags=re.M | re.S)


def anchors(text):
    counts = {}
    result = set()
    for heading in re.findall(r'^#{1,6}\s+(.+?)\s*#*$', prose(text), re.M):
        slug = re.sub(r'[^\w\- ]', '', heading.lower()).replace(' ', '-')
        count = counts.get(slug, 0)
        counts[slug] = count + 1
        result.add(f'{slug}-{count}' if count else slug)
    return result


def check(root, files, packaged=False):
    root = root.resolve()
    files = set(files)
    errors = []
    for name in sorted(files):
        if not name.endswith('.md'):
            continue
        path = root / name
        if not path.is_file():
            errors.append(f'{name}: missing Markdown file')
            continue
        text = path.read_text(encoding='utf-8')
        if re.search(r'/(?:Users|home)/[^\s]+', text):
            errors.append(f'{name}: contains a machine-specific home path')
        for raw in LINK.findall(prose(text)):
            target = urlsplit(raw.strip('<>'))
            repository_url = (target.scheme == 'https' and target.netloc == 'github.com'
                              and target.path.startswith(REPOSITORY_PATH))
            if repository_url:
                dest = (root / unquote(target.path[len(REPOSITORY_PATH):])).resolve()
            elif target.scheme or target.netloc:
                continue
            else:
                dest = (path.parent / unquote(target.path)).resolve() if target.path else path.resolve()
            if not dest.is_relative_to(root):
                errors.append(f'{name}: link leaves repository: {raw}')
                continue
            relative = dest.relative_to(root).as_posix()
            if not dest.exists() or (packaged and not repository_url and relative not in files):
                errors.append(f'{name}: missing link target: {raw}')
            elif target.fragment and dest.suffix == '.md':
                if unquote(target.fragment) not in anchors(dest.read_text(encoding='utf-8')):
                    errors.append(f'{name}: missing heading: {raw}')
    return errors


def repository_files():
    listing = subprocess.check_output(
        ['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'],
        cwd=ROOT,
    ).decode().split('\0')
    return {name for name in listing if name and (ROOT / name).is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--file-list', type=Path, help='Cargo package --list output')
    args = parser.parse_args()
    files = set(args.file_list.read_text().splitlines()) if args.file_list else repository_files()
    errors = check(ROOT, files, packaged=args.file_list is not None)
    if errors:
        raise SystemExit('\n'.join(errors))
    print(f'Markdown links verified in {sum(p.endswith(".md") for p in files)} files.')


if __name__ == '__main__':
    main()
