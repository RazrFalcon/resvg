#!/usr/bin/env python3.6

import argparse
import csv
import fnmatch
import hashlib
import os
import subprocess as proc
from pathlib import Path
from shutil import copyfile
from subprocess import run


# List of files that should be skipped.
CRASH_ALLOWED = [
    'e-svg-007.svg',  # non-UTF8 encoding
    'e-svg-034.svg',  # invalid size
    'e-svg-035.svg',  # invalid size
    'e-svg-036.svg',  # invalid size
]

CACHE_FILENAME = 'cache.csv'


def change_ext(path, suffix, new_ext):
    return Path(path).stem + suffix + '.' + new_ext


def render_svg(in_svg, out_png):
    run(['node', 'svgrender.js', in_svg, out_png, '200'],
        check=True, cwd='chrome-svgrender')


def load_last_pos():
    path = args.work_dir / 'pos.txt'
    if path.exists():
        with open(path, 'r') as f:
            return int(f.read().splitlines()[0])
    return 0


def save_last_pos(pos):
    path = args.work_dir / 'pos.txt'
    with open(path, 'w') as out:
        out.write(str(pos) + '\n')


def rm_file(file_path):
    if file_path.exists():
        os.remove(file_path)


def remove_artifacts():
    rm_file(svg_copy_path)
    rm_file(svg_path_usvg)
    rm_file(png_path_orig)
    rm_file(png_path_usvg)
    rm_file(diff_path)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--ci-mode', action='store_true', help='Enables the CI mode')
    parser.add_argument('--dpi', type=int, default=96, help='Sets the DPI')
    parser.add_argument('svg_dir', type=Path, help='Sets an input directory with SVG files')
    parser.add_argument('work_dir', type=Path, help='Sets the working directory')
    args = parser.parse_args()

    if not args.work_dir.exists():
        os.mkdir(args.work_dir)

    files = os.listdir(args.svg_dir)
    files = fnmatch.filter(files, '*.svg')
    files = sorted(files)

    allowed_ae = {}
    with open('allow.csv') as f:
        for row in csv.reader(f):
            allowed_ae[row[0]] = int(row[1])

    cache = {}
    if os.path.exists(CACHE_FILENAME):
        with open(CACHE_FILENAME) as f:
            for row in csv.reader(f):
                cache[row[0]] = row[1]

    start_idx = load_last_pos()
    last_idx = 0  # will be set only on error
    for idx, file in enumerate(files):
        svg_path = args.svg_dir / file

        # The output file must have the same name as an original.
        # Otherwise tests like e-image-034.svg will break.
        svg_path_usvg = args.work_dir / file

        svg_copy_path = args.work_dir / change_ext(file, '_orig', 'svg')
        png_path_orig = args.work_dir / change_ext(file, '_orig', 'png')
        png_path_usvg = args.work_dir / change_ext(file, '_usvg', 'png')
        diff_path = args.work_dir / change_ext(file, '_diff', 'png')

        remove_artifacts()

        if idx < start_idx:
            continue

        print('Test {} of {}: {}'.format(idx + 1, len(files), file))

        if file in CRASH_ALLOWED:
            continue

        try:
            run(['../../target/debug/usvg', svg_path, svg_path_usvg, '--dpi', str(args.dpi)],
                check=True)
        except proc.CalledProcessError as e:
            print('Error: usvg crashed.')
            last_idx = idx
            break

        with open(svg_path_usvg, 'rb') as f:
            md5 = hashlib.md5()
            md5.update(f.read())
            md5hash = md5.hexdigest()
            md5hash = md5hash[:8]  # 8 values is enough for us

        # If the md5 hash of the simplified SVG was not changed
        # that there is no need to render and compare raster images
        # because it's very expensive.
        if md5hash == cache.get(svg_path.stem, ''):
            rm_file(svg_path_usvg)
            continue
        elif args.ci_mode:
            # md5 check can't fail in the CI mode.
            print('Error: md5 hash mismatch: {} != {}.'
                  .format(md5hash, cache.get(svg_path.stem, '')))

            # Print the simplified file content.
            with open(svg_path_usvg) as f:
                print(f.read())

            last_idx = idx
            break

        render_svg(svg_path, png_path_orig)
        render_svg(svg_path_usvg, png_path_usvg)

        try:
            run(['compare', '-metric', 'AE', '-fuzz', '1%',
                 png_path_orig, png_path_usvg, diff_path],
                check=True, stdout=proc.PIPE, stderr=proc.STDOUT)
        except proc.CalledProcessError as e:
            ae = int(e.stdout.decode('ascii'))
            if ae > 20 and ae != allowed_ae.get(svg_path.stem, 0):
                print('Error: images are different by {} pixels.'.format(ae))

                # copy original svg on error
                copyfile(svg_path, svg_copy_path)

                last_idx = idx
                break

        # Update md5 hash on OK.
        cache[svg_path.stem] = md5hash

        remove_artifacts()

    save_last_pos(last_idx)

    # Update cache file.
    with open(CACHE_FILENAME, 'w') as f:
        writer = csv.writer(f)
        for key, value in cache.items():
            writer.writerow([key, value])

    if last_idx != 0:
        exit(1)
