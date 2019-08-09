#!/usr/bin/env python3.6

import argparse
import os
import platform
import subprocess
from subprocess import run
from contextlib import contextmanager
from pathlib import Path


TESTS_URL = 'https://github.com/RazrFalcon/resvg-test-suite.git'
SKIA_BUILD_URL = 'https://github.com/RazrFalcon/resvg-skia-ci-build.git'


@contextmanager
def cd(path):
    old_dir = os.getcwd()
    os.chdir(old_dir + '/' + path)
    yield
    os.chdir(old_dir)


def regression_testing(backend):
    reg_work_dir = Path(work_dir) / ('workdir-' + backend)

    if not reg_work_dir.exists():
        os.mkdir(reg_work_dir)

    regression_args = ['cargo', 'run', '--release', '--',
                       '--backend', backend, tests_dir, reg_work_dir]
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

if 'TRAVIS_BUILD_DIR' in os.environ:
    local_test = False
    work_dir = Path('.').resolve()
    tests_dir = Path('./target/resvg-test-suite/svg').resolve()
else:
    local_test = True
    work_dir = '/tmp'
    tests_dir = Path('../resvg-test-suite/svg').resolve()

print('local_test:', local_test)
print('work_dir:', work_dir)
print('tests_dir:', tests_dir)

# clone tests on CI
if not local_test:
    run(['git', 'clone', TESTS_URL, '--depth', '1', './target/resvg-test-suite'], check=True)

# prepare skia on CI
if not local_test and 'RESVG_SKIA_BACKEND' in os.environ:
    run(['git', 'clone', SKIA_BUILD_URL, '--depth', '1'], check=True)
    os.environ['SKIA_DIR'] = str(Path('./resvg-skia-ci-build').resolve())
    os.environ['SKIA_LIB_DIR'] = str(Path('./resvg-skia-ci-build/bin').resolve())
    os.environ['LD_LIBRARY_PATH'] = str(Path('./resvg-skia-ci-build/bin').resolve())


if 'RESVG_QT_BACKEND' in os.environ:
    # build qt backend
    with cd('tools/rendersvg'):
        run(['cargo', 'build', '--release', '--features', 'qt-backend'], check=True)

    # regression testing of the qt backend
    if not args.no_regression:
        with cd('testing-tools/regression'):
            if not local_test:
                os.environ['QT_QPA_PLATFORM'] = 'offscreen'
                run(['sudo', 'ln', '-s', '/usr/share/fonts', '/opt/qt56/lib/fonts'], check=True)

            try:
                regression_testing('qt')
            except subprocess.CalledProcessError:
                exit(1)


if 'RESVG_CAIRO_BACKEND' in os.environ:
    # build cairo backend
    with cd('tools/rendersvg'):
        run(['cargo', 'build', '--release', '--features', 'cairo-backend'], check=True)

    # regression testing of the cairo backend
    if not args.no_regression:
        with cd('testing-tools/regression'):
            try:
                regression_testing('cairo')
            except subprocess.CalledProcessError:
                exit(1)


if 'RESVG_RAQOTE_BACKEND' in os.environ:
    # build raqote backend
    with cd('tools/rendersvg'):
        run(['cargo', 'build', '--release', '--features', 'raqote-backend'], check=True)

    # regression testing of the cairo backend
    if not args.no_regression:
        with cd('testing-tools/regression'):
            try:
                regression_testing('raqote')
            except subprocess.CalledProcessError:
                exit(1)


if 'RESVG_SKIA_BACKEND' in os.environ:
    # build skia backend
    with cd('tools/rendersvg'):
        run(['cargo', 'build', '--release', '--features', 'skia-backend'], check=True)

    # regression testing of the cairo backend
    if not args.no_regression:
        with cd('testing-tools/regression'):
            try:
                regression_testing('skia')
            except subprocess.CalledProcessError:
                exit(1)


# # try to build with all backends
# with cd('tools/rendersvg'):
#     run(['cargo', 'build', '--all-features'], check=True)
#
#
# # run tests and build examples
# run(['cargo', 'test', '--all-features'], check=True)


if 'RESVG_QT_BACKEND' in os.environ:
    # test Qt C-API
    #
    # build C-API for demo
    with cd('capi'):
        run(['cargo', 'build', '--features', 'qt-backend'], check=True)

    # run tests and build examples
    run(['cargo', 'test', '--features', 'qt-backend'], check=True)

    # test Qt C-API wrapper
    qmake_env = os.environ if local_test else dict(os.environ, QT_SELECT="5")

    with cd('capi/qtests'):
        defines = 'DEFINES+=LOCAL_BUILD' if local_test else ''
        run(['make', 'distclean'])
        run(['qmake', 'CONFIG+=debug', defines], env=qmake_env, check=True)
        run(['make'], check=True)
        run(['./tst_resvgqt'], env=dict(os.environ, LD_LIBRARY_PATH="../../target/debug"), check=True)

    with cd('tools/viewsvg'):
        run(['make', 'distclean'])
        run(['qmake', 'CONFIG+=debug'], env=qmake_env, check=True)
        run(['make'], check=True)


if 'RESVG_CAIRO_BACKEND' in os.environ:
    # build cairo C example
    #
    # build C-API for cairo-capi
    with cd('capi'):
        run(['cargo', 'build', '--features', 'cairo-backend'], check=True)

    # run tests and build examples
    run(['cargo', 'test', '--features', 'cairo-backend'], check=True)

    with cd('examples/cairo-capi'):
        run(['make', 'clean'], check=True)
        run(['make'], check=True)

    # build cairo-rs example
    with cd('examples/cairo-rs'):
        run(['cargo', 'build'], check=True)


if 'RESVG_RAQOTE_BACKEND' in os.environ:
    # run tests and build examples
    run(['cargo', 'test', '--release', '--features', 'raqote-backend'], check=True)


if 'RESVG_SKIA_BACKEND' in os.environ:
    # run tests and build examples
    run(['cargo', 'test', '--release', '--features', 'skia-backend'], check=True)


with cd('usvg'):
    run(['cargo', 'test', '--release'], check=True)
