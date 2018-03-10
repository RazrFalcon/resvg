#!/usr/bin/env python3

import os
import platform
import subprocess as proc
import argparse
from contextlib import contextmanager


TESTS_URL = 'https://github.com/RazrFalcon/resvg-test-suite.git'


@contextmanager
def cd(path):
    old_dir = os.getcwd()
    os.chdir(old_dir + '/' + path)
    yield
    os.chdir(old_dir)


def regression_testing(backend):
    regress_dir = work_dir + '/workdir-' + backend
    os.mkdir(regress_dir)

    cargo_args = [
        'cargo', 'run', '--release', '--',
        tests_dir, regress_dir, '--backend', backend,
    ]

    if not local_test:
        cargo_args.append('--use-prev-commit')

    proc.check_call(cargo_args)


if platform.system() != 'Linux':
    print('Error: this script is Linux only.')
    exit(1)

parser = argparse.ArgumentParser()
parser.add_argument('--no-regression', help='Do not run regression testing',
                    action='store_true')
args = parser.parse_args()

if os.getcwd().endswith('scripts'):
    os.chdir('..')

if 'TRAVIS_BUILD_DIR' in os.environ:
    local_test = False
    work_dir = os.path.abspath('.')
    tests_dir = os.path.abspath('./resvg-test-suite/svg')
else:
    local_test = True
    work_dir = '/tmp'
    tests_dir = os.path.abspath('../resvg-test-suite/svg')

print('local_test:', local_test)
print('work_dir:', work_dir)
print('tests_dir:', tests_dir)

# clone tests on CI
if not local_test:
    proc.check_call(['git', 'clone', TESTS_URL, '--depth', '1'])


# build qt backend
with cd('tools/rendersvg'):
    proc.check_call(['cargo', 'build', '--features', 'qt-backend'])

# regression testing of the qt backend
if not args.no_regression:
    with cd('testing_tools/regression'):
        if not local_test:
            os.environ['QT_QPA_PLATFORM'] = 'offscreen'
            proc.check_call(['sudo', 'ln', '-s', '/usr/share/fonts', '/opt/qt56/lib/fonts'])

        regression_testing('qt')


# build cairo backend
with cd('tools/rendersvg'):
    proc.check_call(['cargo', 'build', '--features', 'cairo-backend'])

# regression testing of the cairo backend
if not args.no_regression:
    with cd('testing_tools/regression'):
        regression_testing('cairo')


# try to build with all backends
with cd('tools/rendersvg'):
    proc.check_call(['cargo', 'build', '--features', 'cairo-backend qt-backend'])


# build cargo examples
proc.check_call(['cargo', 'test', '--all-features'])
proc.check_call(['cargo', 'test', '--features', 'cairo-backend'])
proc.check_call(['cargo', 'test', '--features', 'qt-backend'])


# rendersvg unit tests
#
# run only locally, because bboxes depend on freetype settings
if local_test:
    with cd('tools/rendersvg'):
        proc.check_call(['cargo', 'test', '--features', 'qt-backend'])
        proc.check_call(['cargo', 'test', '--features', 'cairo-backend'])


# build demo
#
# build C-API for demo
with cd('capi'):
    proc.check_call(['cargo', 'build', '--features', 'qt-backend'])

with cd('demo'):
    proc.call(['make', 'distclean'])
    proc.check_call(['qmake', 'CONFIG+=debug'], env=dict(os.environ, QT_SELECT="5"))
    proc.check_call(['make'])


# build cairo C example
#
# build C-API for cairo-capi
with cd('capi'):
    proc.check_call(['cargo', 'build', '--features', 'cairo-backend'])

with cd('examples/cairo-capi'):
    proc.call(['make', 'clean'])
    proc.check_call(['make'])


# build cairo-rs example
with cd('examples/cairo-rs'):
    proc.check_call(['cargo', 'build'])
