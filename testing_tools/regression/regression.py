#!/usr/bin/env python3

import argparse
import os
import subprocess
import fnmatch


RESVG_URL = 'https://github.com/RazrFalcon/resvg'

# List of files that should not
CRASH_ALLOWED = [
    'e-svg-007.svg'
]


def build_prev_version():
    prev_resvg_dir = os.path.join(args.work_dir, 'resvg')
    if os.path.exists(prev_resvg_dir):
        print('Warning: previous resvg version already exists')
        return os.path.join(prev_resvg_dir, 'target/debug/rendersvg')

    subprocess.check_call(['git', 'clone', '--depth', '5', RESVG_URL, prev_resvg_dir])

    if args.use_prev_commit:
        # TODO: maybe there is a better way
        subprocess.check_call(['git', 'reset', '--hard', 'HEAD~1'], cwd=prev_resvg_dir)

    subprocess.check_call(['cargo', 'build', '--features', args.backend + '-backend'],
                          cwd=os.path.join(prev_resvg_dir, 'tools/rendersvg'))

    return os.path.join(prev_resvg_dir, 'target/debug/rendersvg')


def change_ext(path, suffix, new_ext):
    return os.path.splitext(path)[0] + suffix + '.' + new_ext


def render_svg(render_path, in_svg, out_png):
    # Render with zoom by default to test scaling.
    # Images may render differently depending on scale.
    return subprocess.call([render_path, '--backend', args.backend, '--zoom', '2', in_svg, out_png],
                           cwd=args.work_dir)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--backend', help='Sets resvg backend', choices=['qt', 'cairo'])
    parser.add_argument('--use-prev-commit', help='Use previous git commit as a reference',
                        action='store_true')
    parser.add_argument('in_dir', help='Sets input directory')
    parser.add_argument('work_dir', help='Sets working directory')
    args = parser.parse_args()

    render_path = os.path.abspath("../../target/debug/rendersvg")
    if not os.path.exists(render_path):
        raise RuntimeError('rendersvg executable not found')

    with open('allow-{}.txt'.format(args.backend), 'r') as f:
        allowed_files_list = f.read().splitlines()
    allowed_files_list.extend(CRASH_ALLOWED)

    prev_render_path = build_prev_version()

    files = os.listdir(args.in_dir)
    files = fnmatch.filter(files, '*.svg')
    files = sorted(files)
    for idx, file in enumerate(files):
        print('Test {} of {}: {}'.format(idx + 1, len(files), file))

        if file in allowed_files_list:
            continue

        svg_path = os.path.join(args.in_dir, file)
        png_path_prev = os.path.join(args.work_dir, change_ext(file, "_prev", 'png'))
        png_path_curr = os.path.join(args.work_dir, change_ext(file, "_curr", 'png'))
        diff_path = os.path.join(args.work_dir, change_ext(file, "_diff", 'png'))

        if render_svg(prev_render_path, svg_path, png_path_prev) != 0:
            continue

        if render_svg(render_path, svg_path, png_path_curr) != 0:
            print('Error: rendersvg returned non-zero exit status.')
            exit(1)

        try:
            diff_val = subprocess.check_output(['compare', '-metric', 'AE',
                                               png_path_prev, png_path_curr, diff_path],
                                               stderr=subprocess.STDOUT)
        except subprocess.CalledProcessError as e:
            print('Error: images are different: {}.'.format(e.stdout.decode('ascii')))
            exit(1)

        os.remove(png_path_prev)
        os.remove(png_path_curr)
        # Remove diff image from the previous run.
        if os.path.exists(diff_path):
            os.remove(diff_path)
