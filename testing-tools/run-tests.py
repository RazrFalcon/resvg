#!/usr/bin/env python3

# Should work on Python >= 3.5

import argparse
import os
import platform
import subprocess
from subprocess import run
from contextlib import contextmanager


SKIA_BUILD_URL = 'https://github.com/RazrFalcon/resvg-skia-ci-build.git'


@contextmanager
def cd(path):
    old_dir = os.getcwd()
    os.chdir(old_dir + '/' + path)
    yield
    os.chdir(old_dir)


def regression_testing(backend):
    reg_work_dir = work_dir + '/' + ('workdir-' + backend)

    if not os.path.exists(reg_work_dir):
        os.mkdir(reg_work_dir)

    regression_args = ['cargo', 'run', '--release', '--', '--backend', backend, reg_work_dir]

    # Use a master branch for pull requests.
    if not local_test and os.environ['TRAVIS_BRANCH'] == 'master':
        regression_args.append('--use-prev-commit')

    run(regression_args, check=True)


if platform.system() != 'Linux':
    print('Error: this script is Linux only.')
    exit(1)

parser = argparse.ArgumentParser()
parser.add_argument('--no-regression', help='Do not run regression testing', action='store_true')
args = parser.parse_args()

if os.getcwd().endswith('testing-tools'):
    os.chdir('..')

if not os.path.exists('./target'):
    os.mkdir('./target')

local_test = 'TRAVIS_BUILD_DIR' not in os.environ
work_dir = os.path.abspath('.')
tests_dir = os.path.abspath('./svg-tests/svg')

print('local_test:', local_test)
print('work_dir:', work_dir)
print('tests_dir:', tests_dir)

if 'RESVG_QT_BACKEND' in os.environ:
    # build qt backend
    with cd('resvg-qt'):
        run(['cargo', 'build', '--release'], check=True)

    # regression testing of the qt backend
    if not args.no_regression:
        with cd('testing-tools/regression'):
            if not local_test:
                os.environ['QT_QPA_PLATFORM'] = 'offscreen'
                run(['sudo', 'ln', '-s', '/usr/share/fonts', '/opt/qt56/lib/fonts'], check=True)

            regression_testing('qt')

    # build examples
    with cd('resvg-qt'):
        run(['cargo', 'test'], check=True)

    # test Qt C-API
    #
    # build C-API for demo
    with cd('resvg-qt/c-api'):
        run(['cargo', 'build'], check=True)

    # test Qt C-API wrapper
    qmake_env = os.environ if local_test else dict(os.environ, QT_SELECT="5")

    # with cd('capi/qtests'):
    #     defines = 'DEFINES+=LOCAL_BUILD' if local_test else ''
    #     run(['make', 'distclean'])
    #     run(['qmake', 'CONFIG+=debug', defines], env=qmake_env, check=True)
    #     run(['make'], check=True)
    #     run(['./tst_resvgqt'], env=dict(os.environ, LD_LIBRARY_PATH="../../target/debug"), check=True)

    with cd('resvg-qt/examples/viewsvg'):
        run(['make', 'distclean'])
        run(['qmake', 'CONFIG+=debug'], env=qmake_env, check=True)
        run(['make'], check=True)


if 'RESVG_CAIRO_BACKEND' in os.environ:
    # build cairo backend
    with cd('resvg-cairo'):
        run(['cargo', 'build', '--release'], check=True)

    # regression testing of the cairo backend
    if not args.no_regression:
        with cd('testing-tools/regression'):
            regression_testing('cairo')

    # build examples
    with cd('resvg-cairo'):
        run(['cargo', 'test'], check=True)

    with cd('resvg-cairo/examples/gtk-ui-rs'):
        run(['cargo', 'build'], check=True)

    # build C-API for gtk-ui-c
    with cd('resvg-cairo/c-api'):
        run(['cargo', 'build', '--release'], check=True)

    with cd('resvg-cairo/examples/gtk-ui-c'):
        run(['make', 'clean'], check=True)
        run(['make'], check=True)


if 'RESVG_SKIA_BACKEND' in os.environ:
    # prepare skia on CI
    if not local_test:
        run(['git', 'clone', SKIA_BUILD_URL, '--depth', '1'], check=True)
        os.environ['SKIA_DIR'] = os.path.abspath('./resvg-skia-ci-build')
        os.environ['SKIA_LIB_DIR'] = os.path.abspath('./resvg-skia-ci-build/bin')
        os.environ['LD_LIBRARY_PATH'] = os.path.abspath('./resvg-skia-ci-build/bin')

    # build skia backend
    with cd('resvg-skia'):
        run(['cargo', 'build', '--release'], check=True)

    # regression testing of the cairo backend
    if not args.no_regression:
        with cd('testing-tools/regression'):
            regression_testing('skia')

    # check C-API
    with cd('resvg-skia/c-api'):
        run(['cargo', 'build'], check=True)

    # build examples
    with cd('resvg-skia'):
        run(['cargo', 'test'], check=True)


if 'RESVG_RAQOTE_BACKEND' in os.environ:
    # build raqote backend
    with cd('resvg-raqote'):
        run(['cargo', 'build', '--release'], check=True)

    # regression testing of the cairo backend
    if not args.no_regression:
        with cd('testing-tools/regression'):
            regression_testing('raqote')

    # build examples
    with cd('resvg-raqote'):
        run(['cargo', 'test'], check=True)


if 'USVG_TESTING' in os.environ:
    with cd('usvg'):
        # build without the `text` feature first
        run(['cargo', 'build', '--no-default-features'], check=True)
        # test with the `text` feature
        run(['cargo', 'test'], check=True)
