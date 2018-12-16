We are using `xml` extension and not `toml` to enable automatic highlight detection by text editors.
And it works, even through those files is not valid XML.

As TOML:
```toml
title = "Minimal"
input = '''
<svg viewBox="0 0 1 1">
    <rect width="10" height="10"/>
</svg>
'''
output = '''
<svg
    width="1"
    height="1"
    viewBox="0 0 1 1"
    preserveAspectRatio="xMidYMid"
    xmlns:xlink="http://www.w3.org/1999/xlink"
    xmlns="http://www.w3.org/2000/svg"
    xmlns:usvg="https://github.com/RazrFalcon/usvg"
    usvg:version="0.1.0">
    <defs/>
    <path
        fill="#000000"
        fill-opacity="1"
        fill-rule="evenodd"
        stroke="none"
        d="M 0 0 L 10 0 L 10 10 L 0 10 Z"/>
</svg>
'''
```

As XML:
```xml
title = "Minimal"
input = '''
<svg viewBox="0 0 1 1">
    <rect width="10" height="10"/>
</svg>
'''
output = '''
<svg
    width="1"
    height="1"
    viewBox="0 0 1 1"
    preserveAspectRatio="xMidYMid"
    xmlns:xlink="http://www.w3.org/1999/xlink"
    xmlns="http://www.w3.org/2000/svg"
    xmlns:usvg="https://github.com/RazrFalcon/usvg"
    usvg:version="0.4.0">
    <defs/>
    <path
        fill="#000000"
        fill-opacity="1"
        fill-rule="evenodd"
        stroke="none"
        d="M 0 0 L 10 0 L 10 10 L 0 10 Z"/>
</svg>
'''

```
