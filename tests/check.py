#!/usr/bin/env python3

import re
import os
import subprocess
from lxml import etree


def split_qname(name):
    if name[0] == '{':
        return name[1:].split('}')
    else:
        return [None, name]


def check_untracked_files(dir):
    output = subprocess.check_output(['git', 'ls-files', '--others', '--exclude-standard', dir])
    if not output:
        return

    output = output.decode('ascii')
    print('Untracked files:')
    print(output)
    raise ValueError('not all tests are added to the git')


def check_title():
    """
    Checks that element/attribute tests has unique titles and shorter than 60 symbols
    """

    files = sorted(os.listdir('svg/'))
    if '.directory' in files:
        files.remove('.directory')

    titles = {}
    for file in files:
        tag_name = re.sub('-[0-9]+\.svg', '', file)

        tree = etree.parse('svg/' + file)
        title = list(tree.getroot())[0].text

        if len(title) > 60:
            raise ValueError('{} has title longer than 60 symbols'.format(file))

        if title in titles:
            if titles[title][0] == tag_name:
                raise ValueError('{} and {} have the same title'.format(titles[title][1], file))

        titles[title] = (tag_name, file)


def check_node_ids():
    """
    Checks that all elements has an unique ID attribute.
    """

    files = sorted(os.listdir('svg/'))
    if '.directory' in files:
        files.remove('.directory')

    ignore_files = [
        'e-svg-031.svg',  # because of ENTITY
        'e-svg-032.svg',  # because of ENTITY
        'e-use-024.svg',  # intended duplicate
    ]

    ignore_tags = [
        'title',
        'desc',
        'stop',
        'feBlend',
        'feColorMatrix',
        'feComponentTransfer',
        'feComposite',
        'feConvolveMatrix',
        'feDiffuseLighting',
        'feDistantLight',
        'feFlood',
        'feFuncA',
        'feFuncB',
        'feFuncG',
        'feFuncR',
        'feGaussianBlur',
        'feImage',
        'feMerge',
        'feMergeNode',
        'feMorphology',
        'feOffset',
        'fePointLight',
        'feSpecularLighting',
        'feSpotLight',
        'feTile',
        'feTurbulence',
    ]

    for file in ignore_files:
        files.remove(file)

    for file in files:
        tree = etree.parse('svg/' + file)
        ids = set()

        for node in tree.getroot().iter():
            if node.tag is etree.Comment:
                continue

            # extract tag name without namespace
            _, tag = split_qname(node.tag)

            if tag not in ignore_tags:
                node_id = node.get('id')
                # ID must be set
                if not node_id:
                    raise ValueError('\'{}\' element in {} has no ID'
                                     .format(tag, file))
                else:
                    # Check that ID is unique
                    if node_id in ids:
                        raise ValueError('\'{}\' ID already exist in {}'
                                         .format(node_id, file))
                    else:
                        ids.add(node_id)


def check_line_width():
    allow = [
        'e-svg-004.svg',
        'e-svg-005.svg',
        'e-svg-007.svg',
        'e-svg-031.svg',
        'e-svg-032.svg',
        'a-fill-028.svg',
        'e-tspan-010.svg',
    ]

    files = sorted(os.listdir('svg/'))
    if '.directory' in files:
        files.remove('.directory')

    for file in allow:
        files.remove(file)

    for file in files:
        with open('svg/' + file, 'r') as f:
            for i, line in enumerate(f.read().splitlines()):
                if len(line) > 100:
                    raise ValueError('Line {} in {} is longer than 100 characters'.format(i, file))


def check_for_unused_xlink_ns():
    # In case when 'xlink:href' is present, but namespace is not set
    # the 'lxml' will raise an error.

    allow = [
        'e-svg-003.svg',
        'e-svg-032.svg',
    ]

    files = sorted(os.listdir('svg/'))
    if '.directory' in files:
        files.remove('.directory')

    for file in allow:
        files.remove(file)

    for file in files:
        tree = etree.parse('svg/' + file)

        has_href = False
        for node in tree.getroot().iter():
            if '{http://www.w3.org/1999/xlink}href' in node.attrib:
                has_href = True
                break

        if not has_href and 'xlink' in tree.getroot().nsmap:
            raise ValueError('{} has an unneeded xlink namespace'.format(file))


def main():
    check_title()
    check_node_ids()
    check_untracked_files('svg')
    check_line_width()
    check_for_unused_xlink_ns()


if __name__ == '__main__':
    try:
        main()
    except etree.ParseError as e:
        print('Error: {}.'.format(e))
        exit(1)
    except ValueError as e:
        print('Error: {}.'.format(e))
        exit(1)
