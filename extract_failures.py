import re

with open("results.md", "r", encoding="utf-8") as f:
    content = f.read()

# Split by the separator "---"
sections = content.split("---")

failures = []
for sec in sections:
    if "Result: ❌ FAIL" in sec or "❌ FAIL" in sec:
        failures.append(sec.strip())

with open("failures.txt", "w", encoding="utf-8") as f:
    f.write(f"Found {len(failures)} failures:\n\n")
    for i, fail in enumerate(failures):
        f.write(f"=== FAILURE {i+1} ===\n")
        f.write(fail)
        f.write("\n\n" + "="*40 + "\n\n")

print(f"Extracted {len(failures)} failures to failures.txt")
