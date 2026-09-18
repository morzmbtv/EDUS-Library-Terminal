#!/usr/bin/env python3
"""Create a portable ZIP while explicitly retaining Unix execute attributes."""
from __future__ import annotations
import argparse
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

EXECUTABLES = {
    'EDUS Библиотека — тест.desktop',
    'launch/launcher',
    'launch/terminal_test_server.py',
}

def add_file(archive: ZipFile, source: Path, name: str) -> None:
    info = ZipInfo(name)
    # Mark the attributes as Unix metadata; otherwise a ZIP created on Windows
    # may expose the high mode bits but Linux extractors can ignore them.
    info.create_system = 3
    info.compress_type = ZIP_DEFLATED
    relative_name = name.split('/', 1)[1] if '/' in name else name
    info.external_attr = ((0o100755 if relative_name in EXECUTABLES else 0o100644) << 16)
    archive.writestr(info, source.read_bytes())

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--app', required=True, type=Path)
    parser.add_argument('--portable', required=True, type=Path)
    parser.add_argument('--server', required=True, type=Path)
    parser.add_argument('--hardware', required=True, type=Path)
    parser.add_argument('--version', required=True)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    root = 'EDUS-Library-Test'
    if not args.app.is_dir() or not args.portable.is_dir() or not args.server.is_file():
        raise SystemExit('Portable source files are missing.')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with ZipFile(args.output, 'w', ZIP_DEFLATED) as archive:
        add_file(archive, args.portable / 'EDUS Библиотека — тест.desktop', f'{root}/EDUS Библиотека — тест.desktop')
        add_file(archive, args.portable / 'ПРОЧИТАЙТЕ.txt', f'{root}/ПРОЧИТАЙТЕ.txt')
        add_file(archive, args.hardware, f'{root}/HARDWARE_TEST.md')
        version = ZipInfo(f'{root}/VERSION')
        version.create_system = 3
        version.compress_type = ZIP_DEFLATED
        version.external_attr = 0o100644 << 16
        archive.writestr(version, f'{args.version}-terminal-test-portable\n'.encode('utf-8'))
        for file in sorted(args.portable.rglob('*')):
            if file.is_file() and file.name not in {'EDUS Библиотека — тест.desktop', 'ПРОЧИТАЙТЕ.txt'}:
                add_file(archive, file, f'{root}/{file.relative_to(args.portable).as_posix()}')
        add_file(archive, args.server, f'{root}/launch/terminal_test_server.py')
        for file in sorted(args.app.rglob('*')):
            if file.is_file(): add_file(archive, file, f'{root}/app/{file.relative_to(args.app).as_posix()}')
    with ZipFile(args.output) as archive:
        required = {f'{root}/EDUS Библиотека — тест.desktop', f'{root}/launch/launcher', f'{root}/launch/terminal_test_server.py', f'{root}/app/index.html'}
        names = set(archive.namelist())
        if not required.issubset(names): raise SystemExit('Portable ZIP validation failed: required files are missing.')
        for name in EXECUTABLES:
            mode = (archive.getinfo(f'{root}/{name}').external_attr >> 16) & 0o777
            if mode != 0o755: raise SystemExit(f'Portable ZIP validation failed: {name} is not executable.')
    print(f'Portable ZIP prepared: {args.output}')
    return 0
if __name__ == '__main__': raise SystemExit(main())
