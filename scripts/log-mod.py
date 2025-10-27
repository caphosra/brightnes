#!/usr/bin/env python3

#
# Modifies log files to make them easier to read.
#
# More specifically, this script does the following modifications:
# - Converts log files from FCEUX format to Brightnes format.
# - Detects loops in log files.
#
# uv run ./scripts/log-mod.py <log-file>
#

import re
import sys
from pathlib import Path
from tqdm import tqdm
from typing import List

MAX_LOOP_INTERVAL = 10
MIN_LOOP_LEN = 5

log_file = Path(sys.argv[1])
output_file = log_file.with_suffix(".mod.log")

with open(log_file, 'r') as f:
    lines = f.readlines()

def format_lines(lines: List[str]) -> List[str]:
    print("--- Formatting lines ---")

    output = []
    modified_lines = 0

    # A **hand-made** regex pattern to match FCEUX log lines.
    log_pattern = re.compile(r"^\$([0-9A-F]+):\s[0-9A-F][0-9A-F]\s+([0-9A-F][0-9A-F])?\s+([0-9A-F][0-9A-F])?\s+([A-Z][A-Z][A-Z])\s*(\(?#?\$[0-9A-F][0-9A-F]([0-9A-F][0-9A-F])?\)?(,X|,Y)?\)?)?.*A:([0-9A-F][0-9A-F])\sX:([0-9A-F][0-9A-F])\sY:([0-9A-F][0-9A-F])\sS:([0-9A-F][0-9A-F])\sP:(\S+)\s*$")

    for line in tqdm(lines):
            matched = log_pattern.match(line)
            if matched:
                addr = matched.group(1)
                opcode = matched.group(4)
                operand = matched.group(5)
                operand = operand if operand else ""
                a_reg = matched.group(8)
                x_reg = matched.group(9)
                y_reg = matched.group(10)
                s_reg = matched.group(11)
                p_reg_txt = matched.group(12)

                # Convert status register from text to hex.
                #
                # Brightnes always set bit 5.
                p_reg = 1 << 5
                for i in range(8):
                    if p_reg_txt[7 - i].isupper():
                        p_reg |= (1 << i)

                inst = f"{opcode} {operand}".strip()
                output_line = f"${addr}: {inst:30} A={a_reg} X={x_reg} Y={y_reg} P={p_reg:02X} SP={s_reg}\n"
                output.append(output_line)

                modified_lines += 1
            else:
                output.append(line)

    print(f"[-] Formatted {modified_lines} lines from FCEUX format to Brightnes one.")
    return output

def remove_info(lines: List[str]) -> List[str]:
    print("--- Removing unneeded info ---")

    output = []

    log_pattern = re.compile(r"^\$[0-9A-F]+:")

    for line in tqdm(lines):
        if log_pattern.match(line):
            output.append(line)

    print(f"[-] Removed lines: {len(lines)} -> {len(output)}")
    return output

def detect_loops(lines: List[str]) -> List[str]:
    print("--- Detecting loops ---")

    output = []
    modified_loops = 0

    skip = 0
    for idx in tqdm(range(len(lines))):
        if skip > 0:
            # We are in the middle of a detected loop.
            skip -= 1
            continue

        # Assume that there are loops.
        loop_removed = False
        for interval in range(1, MAX_LOOP_INTERVAL + 1):
            target = idx + interval
            streak = 0
            while target + interval - 1 < len(lines):
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

                modified_loops += 1
                break
        if not loop_removed:
            output.append(lines[idx])

    print(f"[-] Detected {modified_loops} loops in the log.")
    print(f"[-] Reduced lines: {len(lines)} -> {len(output)}")

    return output

output = format_lines(lines)
output = remove_info(output)
output = detect_loops(output)

with open(output_file, 'w') as f:
    f.writelines(output)
