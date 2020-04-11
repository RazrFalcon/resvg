project('resvg', 'c', version: '0.9.0')

cargo = find_program('cargo', required: true)

cargo_script = find_program('scripts/build.sh')

lib = 'libresvg.so.0.'+meson.project_version()

target = custom_target('resvg',
                       build_by_default: true,
                       install: true,
                       install_dir: lib,
                       console: true,
                       output: 'libresvg.so.0.'+meson.project_version(),
                       command: [cargo_script, '@SOURCE_ROOT@', '@OUTPUT@', meson.build_root(), get_option('buildtype'), get_option('backends')])

if get_option('backends') == 'both' or get_option('backends') == 'qt'
    install_headers(['capi/include/resvg.h', 'capi/include/ResvgQt.h'], subdir: 'resvg')
endif

pkg = import('pkgconfig')

pkg.generate(libraries: '-L${libdir} -lresvg',
             name: 'resvg',
             description: 'An SVG rendering library',
             subdirs: 'resvg')

meson.add_install_script('scripts/link.sh', join_paths(get_option('prefix'), get_option('libdir')), lib)