# usvg C API

This is not a real C API, but rather a collection of common files
that used by backends C API implementations.

This solution, based on symlinks, is far from perfect, but this allows us
to substantially reduce the code duplication in C API implementations.
Also, since each backend has it's own vendored archive, it makes publishing a bit easier.
