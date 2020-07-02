#!/usr/bin/env python3

# Should work on Python >= 3.5

import os
import platform
from subprocess import run


SKIA_BUILD_URL = 'https://github.com/RazrFalcon/resvg-skia-ci-build.git'

if platform.system() != 'Linux':
    print('Error: this script is Linux only.')
    exit(1)

local_test = 'TRAVIS_BUILD_DIR' not in os.environ

# force embedded harfbuzz build
os.environ['HARFBUZZ_SYS_NO_PKG_CONFIG'] = '1'

# prepare skia on CI
if not local_test:
    run(['git', 'clone', SKIA_BUILD_URL, '--depth', '1'], check=True)
    os.environ['SKIA_DIR'] = os.path.abspath('./resvg-skia-ci-build')
    os.environ['SKIA_LIB_DIR'] = os.path.abspath('./resvg-skia-ci-build/bin')
    os.environ['LD_LIBRARY_PATH'] = os.path.abspath('./resvg-skia-ci-build/bin')

# run tests
run(['cargo', 'test', '--release'], check=True)

# build C-API
os.chdir('c-api')
run(['cargo', 'build'], check=True)
os.chdir('..')

# test usvg
os.chdir('usvg')
# build without the `text` feature first
run(['cargo', 'build', '--no-default-features'], check=True)
# build with default features
run(['cargo', 'build'], check=True)
# test with default features
run(['cargo', 'test'], check=True)
os.chdir('..')
