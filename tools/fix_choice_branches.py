#!/usr/bin/env python3
"""Post-process .bscript.ron to insert Halt+Jump between choice branches.

For each Choice with goto targets, finds the branch label sequence,
inserts Halt before the first branch, and Jump(convergence) before
each subsequent branch label so branches don't fall through into
each other but all converge on the common content after the choice.
"""

import sys
import os
import re

def parse_labels(content: str) -> list[tuple[int, str]]:
    """Return (line_index, name) for each Label command."""
    labels = []
    lines = content.split('\n')
    i = 0
    while i < len(lines):
        if lines[i].strip() == 'Label(' or lines[i].strip().startswith('Label('):
            j = i + 1
            while j < len(lines):
                ls = lines[j].strip()
                m = re.match(r'name:\s*"(\w+)"\s*,?\s*$', ls)
                if m:
                    labels.append((i, m.group(1)))
                    break
                if ls == '),' or ls.startswith(')'):
                    break
                j += 1
        i += 1
    return labels

def fix_file(path: str) -> bool:
    with open(path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Collection of all goto targets from all Choice commands
    goto_targets = {m.group(1) for m in re.finditer(r'goto:\s*Some\("(\w+)"\)', content)}
    if not goto_targets:
        return False

    labels = parse_labels(content)
    label_names = [name for _, name in labels]

    # Build map: for each goto target, find the convergence label
    # Convergence = first label after this target that is NOT a goto target
    # and is not a qjlabel (those are intermediate jump bookmarks within branches)
    convergences: dict[str, str] = {}
    # Also find the eventual common convergence (after ALL targets)
    all_target_indices = [i for i, name in enumerate(labels) if name in goto_targets]

    for i, (line_idx, target_name) in enumerate(labels):
        if target_name not in goto_targets:
            continue
        # Scan forward to find convergence (next non-target, non-qjlabel label)
        for j in range(i + 1, len(labels)):
            next_name = labels[j][1]
            if next_name not in goto_targets and not next_name.startswith('qjlabel'):
                convergences[target_name] = next_name
                break

    # Find consecutive runs of labels that are goto targets (skipping qjlabels in between)
    target_positions = []
    for i, (line_idx, name) in enumerate(labels):
        if name in goto_targets:
            target_positions.append((line_idx, name))

    # Group consecutive target positions into runs
    # Two targets are "consecutive" if there's no non-goto, non-qjlabel label between them
    runs = []
    current_run = [target_positions[0]] if target_positions else []
    for k in range(1, len(target_positions)):
        prev_name = target_positions[k - 1][1]
        curr_name = target_positions[k][1]
        prev_idx = label_names.index(prev_name)
        curr_idx = label_names.index(curr_name)
        # Check if any non-target, non-qjlabel label exists between them
        has_separator = False
        for between_idx in range(prev_idx + 1, curr_idx):
            between_name = label_names[between_idx]
            if between_name not in goto_targets and not between_name.startswith('qjlabel'):
                has_separator = True
                break
        if has_separator:
            runs.append(current_run)
            current_run = [target_positions[k]]
        else:
            current_run.append(target_positions[k])
    if current_run:
        runs.append(current_run)

    lines = content.split('\n')
    insertions = []

    for run in runs:
        if not run:
            continue

        # Find convergence for this run (the first non-target, non-qjlabel label after the last target in the run)
        last_target_name = run[-1][1]
        convergence = convergences.get(last_target_name)
        if not convergence:
            continue  # skip if no convergence found

        # Halt before the first branch label in this run
        first_line = run[0][0]
        indent = lines[first_line][:len(lines[first_line]) - len(lines[first_line].lstrip())]
        insertions.append((first_line, f'{indent}Halt,'))

        # Jump to convergence before each subsequent branch label (except the last)
        for line_idx, name in run[1:]:
            branch_indent = lines[line_idx][:len(lines[line_idx]) - len(lines[line_idx].lstrip())]
            insertions.append((line_idx, f'{branch_indent}Jump( target: "{convergence}" ),'))

    if not insertions:
        return False

    insertions.sort(key=lambda x: x[0], reverse=True)
    for line_idx, text in insertions:
        lines.insert(line_idx, text)

    new_content = '\n'.join(lines)
    if new_content == content:
        return False

    with open(path, 'w', encoding='utf-8') as f:
        f.write(new_content)
    return True

def main():
    root = sys.argv[1] if len(sys.argv) > 1 else 'assets/scripts'
    fixed = 0
    for fname in sorted(os.listdir(root)):
        if not fname.endswith('.bscript.ron'):
            continue
        path = os.path.join(root, fname)
        if fix_file(path):
            fixed += 1
            print(f'  fixed: {fname}')
    print(f'Fixed {fixed} files')

if __name__ == '__main__':
    main()
