from pathlib import Path
from collections import defaultdict
from argparse import ArgumentParser

# Unfortunately, LLVM special case lists cannot express the concept of mutually
# exclusive categories. As a consequence, it is not possible to change the
# value assigned to a specific regex, for example from "discard" to "custom",
# in an easy way. The only viable solution is to parse the original ABI list
# and generate a new one modifying only the relevant categories.


def main(args):
    original_abilist = parse_special_case_list(args.original_abilist)
    project_abilist = parse_special_case_list(args.project_abilist)

    for entity_type in project_abilist:
        for category in project_abilist[entity_type]:
            for regex in project_abilist[entity_type][category]:

                # Only uninstrumented is not mutually exclusive
                if not category == 'uninstrumented':
                    original_abilist[entity_type]['discard'].discard(regex)
                    original_abilist[entity_type]['functional'].discard(regex)
                    original_abilist[entity_type]['custom'].discard(regex)

                original_abilist[entity_type][category].add(regex)

    unparsed_list = unparse_special_case_list(original_abilist)

    if args.output_path is not None:
        with open(args.output_path, 'w') as output_abilist_file:
            for line in unparsed_list:
                output_abilist_file.write(f'{line}\n')
    else:
        for line in unparsed_list:
            print(line)


def unparse_special_case_list(parsed_list):
    unparsed_list = []

    for entity_type in parsed_list:
        for category in parsed_list[entity_type]:
            for regex in parsed_list[entity_type][category]:
                unparsed_list.append(f'{entity_type}:{regex}={category}')

    return unparsed_list


def parse_special_case_list(special_case_file_path):
    parsed_list = {}

    parsed_list['src'] = defaultdict(set)
    parsed_list['fun'] = defaultdict(set)

    with open(special_case_file_path) as special_case_file:
        for line in special_case_file:
            stripped_line = line.strip()
            if stripped_line == '' or stripped_line.startswith('#'):
                continue

            column_split = stripped_line.split(':')
            equal_split = column_split[1].split('=')

            entity_type = column_split[0]
            regex = equal_split[0]
            category = equal_split[1]

            if not (entity_type == 'fun' or entity_type == 'src'):
                print(f'Unexpected entity type in line: {line}')
                continue

            if not (category == 'discard' or category == 'functional'
                    or category == 'custom' or category == 'uninstrumented'):
                print(f'Unexpected category in line: {line}')
                continue

            parsed_list[entity_type][category].add(regex)

    return parsed_list


if __name__ == '__main__':
    parser = ArgumentParser()
    parser.add_argument('original_abilist', type=Path)
    parser.add_argument('project_abilist', type=Path)
    parser.add_argument('-o', '--output-path', type=Path)

    main(parser.parse_args())
