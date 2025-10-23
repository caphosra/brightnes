#!/usr/bin/env python3

#
# Removes loops from log files.
#
# $ ./log-loop-remover.py <log-file>
#

import sys
from pathlib import Path
from tqdm import tqdm

MAX_LOOP_INTERVAL = 10
MIN_LOOP_LEN = 5

log_file = Path(sys.argv[1])

with open(log_file, 'r') as f:
    lines = f.readlines()
    lines_len = len(lines)

output = []

skip = 0
for idx in tqdm(range(lines_len)):
    if skip > 0:
        # We are in the middle of a detected loop.
        skip -= 1
        continue

    # Assume that there are loops.
    loop_removed = False
    for interval in range(1, MAX_LOOP_INTERVAL + 1):
        target = idx + interval
        streak = 0
        while target + interval - 1 < lines_len:
            is_loop = True

            # Check if the next lines match the first lines.
            for offset in range(interval):
                if lines[idx + offset] != lines[target + offset]:
                    is_loop = False
                    break
            if is_loop:
                streak += 1
                target += interval
            else:
                break
        if streak >= MIN_LOOP_LEN:
            # Found a loop.
            for offset in range(interval):
                output.append(lines[idx + offset])
            for offset in range(interval):
                output.append(f"... loop {streak} times ...\n")

            skip += (streak + 1) * interval - 1
            loop_removed = True
            break
    if not loop_removed:
        output.append(lines[idx])

with open(log_file.with_suffix('.cleaned.log'), 'w') as f:
    f.writelines(output)
