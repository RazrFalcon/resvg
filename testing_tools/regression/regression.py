#!/usr/bin/env python3.6

import argparse
import os
import subprocess as proc
from subprocess import run
import fnmatch
from pathlib import Path


RESVG_URL = 'https://github.com/RazrFalcon/resvg'

# List of files that should not
CRASH_ALLOWED = [
    'e-svg-007.svg'
]


def build_prev_version() -> Path:
    prev_resvg_dir = args.work_dir / 'resvg'
    if prev_resvg_dir.exists():
        print('Warning: previous resvg version already exists')
        return prev_resvg_dir / 'target/debug/rendersvg'

    run(['git', 'clone', '--depth', '5', RESVG_URL, prev_resvg_dir], check=True)

    if args.use_prev_commit:
        # TODO: maybe there is a better way
        run(['git', 'reset', '--hard', 'HEAD~1'], cwd=prev_resvg_dir, check=True)

    run(['cargo', 'build', '--features', args.backend + '-backend'],
        cwd=prev_resvg_dir / 'tools/rendersvg', check=True)

    return prev_resvg_dir / 'target/debug/rendersvg'


def change_ext(path: str, suffix: str, new_ext: str) -> str:
    return Path(path).stem + suffix + '.' + new_ext


def render_svg(render_path, in_svg, out_png):
    # Render with zoom by default to test scaling.
    # Images may render differently depending on scale.
    return run([render_path, '--backend', args.backend, '--zoom', '2', in_svg, out_png],
               cwd=args.work_dir).returncode


def load_last_pos():
    path = args.work_dir / 'pos.txt'
    if path.exists():
        with open(path, 'r') as f:
            return int(f.read().splitlines()[0])
    return 0


def save_last_pos(pos: int):
    path = args.work_dir / 'pos.txt'
    with open(path, 'w') as out:
        out.write(str(pos) + '\n')


def rm_file(file_path: Path):
    if file_path.exists():
        os.remove(file_path)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--backend', help='Sets resvg backend', choices=['qt', 'cairo'])
    parser.add_argument('--use-prev-commit', help='Use previous git commit as a reference',
                        action='store_true')
    parser.add_argument('in_dir', type=Path, help='Sets input directory')
    parser.add_argument('work_dir', type=Path, help='Sets working directory')
    args = parser.parse_args()

    render_path = Path('../../target/debug/rendersvg').resolve()
    if not render_path.exists():
        raise RuntimeError('rendersvg executable not found')

    with open('allow-{}.txt'.format(args.backend), 'r') as f:
        allowed_files_list = f.read().splitlines()
    allowed_files_list.extend(CRASH_ALLOWED)

    prev_render_path = build_prev_version()

    start_idx = load_last_pos()
    files = os.listdir(args.in_dir)
    files = fnmatch.filter(files, '*.svg')
    files = sorted(files)
    for idx, file in enumerate(files):
        svg_path = args.in_dir / file
        png_path_prev = args.work_dir / change_ext(file, '_prev', 'png')
        png_path_curr = args.work_dir / change_ext(file, '_curr', 'png')
        diff_path = args.work_dir / change_ext(file, '_diff', 'png')

        # remove leftovers
        rm_file(png_path_prev)
        rm_file(png_path_curr)
        rm_file(diff_path)

        if idx < start_idx:
            continue

        print('Test {} of {}: {}'.format(idx + 1, len(files), file))

        if file in allowed_files_list:
            continue

        if render_svg(prev_render_path, svg_path, png_path_prev) != 0:
            continue

        if render_svg(render_path, svg_path, png_path_curr) != 0:
            print('Error: rendersvg returned non-zero exit status.')
            save_last_pos(idx)
            exit(1)

        try:
            diff_val = run(['compare', '-metric', 'AE', png_path_prev, png_path_curr, diff_path],
                           check=True, stdout=proc.PIPE, stderr=proc.STDOUT).stdout
        except proc.CalledProcessError as e:
            print('Error: images are different by {} pixels.'.format(e.stdout.decode('ascii')))
            save_last_pos(idx)
            exit(1)

        rm_file(png_path_prev)
        rm_file(png_path_curr)
        rm_file(diff_path)

    save_last_pos(0)
