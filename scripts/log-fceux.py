#!/usr/bin/env python3

#
# This script is intended to convert a FCEUX log file into a Brightnes log file.
#
# Since a logging format of Brightnes is different from that of FCEUX, a well-known emulator,
# it is difficult to correlate logs between the two.
#

import sys
import re
from pathlib import Path
from tqdm import tqdm

log_path = Path(sys.argv[1])
output_path = log_path.with_suffix(".mod.log")

# A **hand-made** regex pattern to match FCEUX log lines.
log_pattern = re.compile(r"^\$([0-9A-F]+):\s[0-9A-F][0-9A-F]\s+([0-9A-F][0-9A-F])?\s+([0-9A-F][0-9A-F])?\s+([A-Z][A-Z][A-Z])\s*(\(?#?\$[0-9A-F][0-9A-F]([0-9A-F][0-9A-F])?\)?(,X|,Y)?\)?)?.*A:([0-9A-F][0-9A-F])\sX:([0-9A-F][0-9A-F])\sY:([0-9A-F][0-9A-F])\sS:([0-9A-F][0-9A-F])\sP:(\S+)\s*$")

with open(log_path, "r") as log:
    with open(output_path, "w") as output:
        for line in tqdm(log.readlines()):
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

                p_reg = 0
                for i in range(8):
                    if p_reg_txt[7 - i].isupper():
                        p_reg |= (1 << i)

                inst = f"{opcode} {operand}".strip()
                output_line = f"${addr}: {inst:30} A={a_reg} X={x_reg} Y={y_reg} P={p_reg:02X} SP={s_reg}\n"
                output.write(output_line)
            else:
                print(f"[!] Not matched: {line}")
