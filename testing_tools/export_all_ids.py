#!/usr/bin/env python3.6

import os
import subprocess
import argparse

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('svg', help='Input file')
    parser.add_argument('out_dir', help='Output directory')
    parser.add_argument('-b, --backend', dest='backend', help='Sets backend', required=True)
    parser.add_argument('-z, --zoom', dest='zoom', help='Sets zoom', default=1)
    args = parser.parse_args()

    if not os.path.exists(args.out_dir):
        os.mkdir(args.out_dir)

    resvg = '../tools/rendersvg/target/debug/rendersvg'

    subprocess.check_call(['cargo', 'build', '--all-features'],
                          cwd='../tools/rendersvg/')

    out = subprocess.check_output([
        resvg,
        args.svg,
        '--query-all',
        '--backend', args.backend,
        '-z', args.zoom
    ]).decode('utf-8')

    for line in out.splitlines():
        node_id = line.split(',')[0]
        out_png = os.path.join(args.out_dir, node_id + '.png')
        subprocess.check_call([
            resvg,
            args.svg,
            out_png,
            '--export-id', node_id,
            '--backend', args.backend,
            '-z', args.zoom
        ])
