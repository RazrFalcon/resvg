#!/usr/bin/env python3

import csv
import argparse
from enum import Enum


class Result(Enum):
    Ok = 0
    Error = 1
    Crashed = 2
    OutOfScope = 3


def csv_file(value):
    if not value.endswith('.csv'):
        raise argparse.ArgumentTypeError('not a csv')
    return value


def html_file(value):
    if not value.endswith('.html'):
        raise argparse.ArgumentTypeError('not an html')
    return value


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('input_file', help='an input file', type=csv_file)
    parser.add_argument('output_file', help='an output file', type=html_file)
    parser.add_argument('--print-stats', help='print stats', action='store_true')
    parser.add_argument('--with-out-of-scope', help='print out of scope files',
                        action='store_true')
    args = parser.parse_args()

    rows = []
    with open(args.input_file) as f:
        rows = list(csv.reader(f, delimiter=',', quotechar='"'))

    with open(args.output_file, 'w') as f:
        f.write("<html>\n")

        f.write("<style>\n")
        with open("style.css") as style_file:
            f.write(style_file.read())
        f.write("</style>\n")

        f.write("<table>\n")

        f.write("<tr>\n")
        f.write("<th></th>\n")
        for item in rows[0][0:-1]:
            f.write("<th class=\"vertical\"><div>{}</div></th>\n".format(item))

        f.write("</tr>\n")

        for row in rows[1:]:
            is_out_of_scope = int(row[0]) == Result.OutOfScope.value
            if not args.with_out_of_scope and is_out_of_scope:
                continue

            f.write("<tr>\n")
            f.write("<td align=\"right\">{}&nbsp;</td>\n".format(row[-1]))

            for item in row[0:-1]:
                res = Result(int(item))

                f.write("<td class=\"{}\"></td>\n".format(res.name))
            f.write("</tr>\n")

        f.write("</table>\n")

        f.write("</html>\n")


    if args.print_stats:
        table = [[0]*3 for i in range(5)]

        for row in rows[1:]:
            for i, cell in enumerate(row[0:-1]):
                if int(cell) == Result.Ok.value:
                    table[i][0] += 1
                elif int(cell) == Result.Error.value:
                    table[i][1] += 1
                elif int(cell) == Result.Crashed.value:
                    table[i][2] += 1


        print(table)
