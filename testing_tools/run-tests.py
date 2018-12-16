#!/usr/bin/env python3.6

import argparse
import os
import platform
import subprocess as proc
from subprocess import run
from contextlib import contextmanager
from pathlib import Path


TESTS_URL = 'https://github.com/RazrFalcon/resvg-test-suite.git'


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

    regression_args = ['./regression.py', tests_dir, reg_work_dir, '--backend', backend]
    if not local_test:
        regression_args.append('--use-prev-commit')

    run(regression_args, check=True)


if platform.system() != 'Linux':
    print('Error: this script is Linux only.')
    exit(1)

parser = argparse.ArgumentParser()
parser.add_argument('--no-regression', help='Do not run regression testing', action='store_true')
args = parser.parse_args()

if os.getcwd().endswith('testing_tools'):
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


if 'RESVG_QT_BACKEND' in os.environ:
    # build qt backend
    with cd('tools/rendersvg'):
        run(['cargo', 'build', '--features', 'qt-backend'], check=True)

    # regression testing of the qt backend
    if not args.no_regression:
        with cd('testing_tools/regression'):
            if not local_test:
                os.environ['QT_QPA_PLATFORM'] = 'offscreen'
                run(['sudo', 'ln', '-s', '/usr/share/fonts', '/opt/qt56/lib/fonts'], check=True)

            try:
                regression_testing('qt')
            except proc.CalledProcessError:
                exit(1)


if 'RESVG_CAIRO_BACKEND' in os.environ:
    # build cairo backend
    with cd('tools/rendersvg'):
        run(['cargo', 'build', '--features', 'cairo-backend'], check=True)

    # regression testing of the cairo backend
    if not args.no_regression:
        with cd('testing_tools/regression'):
            try:
                regression_testing('cairo')
            except proc.CalledProcessError:
                exit(1)


# try to build with all backends
with cd('tools/rendersvg'):
    run(['cargo', 'build', '--features', 'cairo-backend qt-backend'], check=True)


# run tests and build examples
run(['cargo', 'test', '--all-features'], check=True)
run(['cargo', 'test', '--features', 'cairo-backend'], check=True)
run(['cargo', 'test', '--features', 'qt-backend'], check=True)


if 'RESVG_QT_BACKEND' in os.environ:
    # test Qt C-API
    #
    # build C-API for demo
    with cd('capi'):
        run(['cargo', 'build', '--features', 'qt-backend'], check=True)

    # test Qt C-API wrapper
    qmake_env = os.environ if local_test else dict(os.environ, QT_SELECT="5")

    with cd('capi/qtests'):
        defines = 'DEFINES+=LOCAL_BUILD' if local_test else ''
        run(['make', 'distclean'])
        run(['qmake', 'CONFIG+=debug', defines], env=qmake_env, check=True)
        run(['make'], check=True)
        run(['./tst_resvgqt'], env=dict(os.environ, LD_LIBRARY_PATH="../../target/debug"), check=True)

    with cd('examples/qt-demo'):
        run(['make', 'distclean'])
        run(['qmake', 'CONFIG+=debug'], env=qmake_env, check=True)
        run(['make'], check=True)

    with cd('examples/resvg-vs-qtsvg'):
        run(['make', 'distclean'])
        run(['qmake', 'CONFIG+=debug'], env=qmake_env, check=True)
        run(['make'], check=True)


if 'RESVG_CAIRO_BACKEND' in os.environ:
    # build cairo C example
    #
    # build C-API for cairo-capi
    with cd('capi'):
        run(['cargo', 'build', '--features', 'cairo-backend'], check=True)

    with cd('examples/cairo-capi'):
        run(['make', 'clean'], check=True)
        run(['make'], check=True)

    # build cairo-rs example
    with cd('examples/cairo-rs'):
        run(['cargo', 'build'], check=True)


if 'USVG_TESTING' in os.environ:
    with cd('usvg'):
        run(['cargo', 'test'], check=True)

    # usvg/testing_tools/regression.py uses tools/usvg
    with cd('tools/usvg'):
        run(['cargo', 'build'], check=True)

    with cd('usvg/testing_tools'):
        run(['./regression.py', '--ci-mode', '../../target/resvg-test-suite/svg',
             '../../target/test-suite-temp'], check=True)
