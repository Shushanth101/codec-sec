import re

with open("failures.txt", "r", encoding="utf-8") as f:
    lines = f.readlines()

clean_lines = []
for line in lines:
    if len(line) > 1000:
        clean_lines.append(line[:500] + "... [TRUNCATED] ...\n")
    else:
        clean_lines.append(line)

with open("failures_clean.txt", "w", encoding="utf-8") as f:
    f.writelines(clean_lines)

print("Cleaned up failures to failures_clean.txt")
