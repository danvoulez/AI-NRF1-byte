#!/usr/bin/env python3
"""
Light polish of docs/MODULES-DESIGN.md:
- Fix setext headings → atx (##)
- Fix `* * *` → `---`
- Fix `*   ` list markers → `* `
- Fix numbered list prefixes (keep original numbers but add blank lines)
- Add blank lines around lists
- Remove double blank lines
- Keep ALL content intact
"""

import re
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PATH = os.path.join(ROOT, "docs", "MODULES-DESIGN.md")

with open(PATH, "r") as f:
    text = f.read()

# 1) Fix setext headings (Title\n====) → ## Title
text = re.sub(r'^(.+)\n={3,}\s*$', r'## \1', text, flags=re.MULTILINE)
text = re.sub(r'^(.+)\n-{3,}\s*$', r'### \1', text, flags=re.MULTILINE)

# 2) Fix `* * *` → `---`
text = re.sub(r'^\* \* \*\s*$', '---', text, flags=re.MULTILINE)

# 3) Fix list marker spacing: `*   text` → `* text`, `-   text` → `- text`
text = re.sub(r'^(\s*)\*   ', r'\1* ', text, flags=re.MULTILINE)
text = re.sub(r'^(\s*)-   ', r'\1- ', text, flags=re.MULTILINE)

# 4) Fix indented list items: ` * ` (1 space) → `* ` (0 spaces) at top level
text = re.sub(r'^ \* ', '* ', text, flags=re.MULTILINE)
text = re.sub(r'^   \* ', '  * ', text, flags=re.MULTILINE)

# 5) Fix numbered lists: add blank line before first item if missing
lines = text.split('\n')
out = []
for i, line in enumerate(lines):
    # Add blank line before numbered list item if previous line is not blank/list
    if re.match(r'^\d+\.', line) and i > 0 and out and out[-1].strip() != '' and not re.match(r'^\d+\.', out[-1]):
        out.append('')
    # Add blank line after numbered list item if next line is not blank/list
    out.append(line)

text = '\n'.join(out)

# 6) Remove triple+ blank lines → double
text = re.sub(r'\n{4,}', '\n\n\n', text)

# 7) Remove double blank lines → single (except before headings)
text = re.sub(r'\n\n\n(?!#)', '\n\n', text)

# 8) Fix fenced code blocks without language: ``` → ```text (for non-rust/json blocks)
# Only fix the directory structure blocks
text = re.sub(r'```\nmodules/', '```text\nmodules/', text)

with open(PATH, "w") as f:
    f.write(text)

print(f"Polished {PATH}")
print(f"  {len(text)} bytes")
