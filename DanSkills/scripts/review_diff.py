#!/usr/bin/env python3
import subprocess, sys, re

def git(*args):
    return subprocess.check_output(["git", *args], text=True)

def main():
    try:
        diff = git("diff", "--cached")
        if not diff.strip():
            diff = git("diff", "HEAD~1...HEAD")
    except subprocess.CalledProcessError:
        diff = ""
    out = []
    out.append("## Reviewer automático (heurístico)\n")
    if not diff.strip():
        out.append("- Sem diff detectado.\n")
    else:
        warns = 0
        for i, line in enumerate(diff.splitlines(), 1):
            if re.search(r"\bprint\(", line):
                out.append(f"- Linha {i}: evitar `print()` em código de produção.\n"); warns+=1
            if "TODO" in line:
                out.append(f"- Linha {i}: resolver/registrar TODO antes do merge.\n"); warns+=1
            m = re.search(r"^\+def\s+([a-zA-Z_][a-zA-Z0-9_]*)\(", line)
            if m:
                fn = m.group(1)
                prefix = (os.environ.get("REQUIRED_FN_PREFIX") or "bill_")
                if not fn.startswith(prefix):
                    out.append(f"- Linha {i}: função `{fn}` não segue prefixo `{prefix}`.\n"); warns+=1
        if warns == 0:
            out.append("- Nenhum problema heurístico encontrado ✅\n")
    sys.stdout.write("".join(out))

if __name__ == "__main__":
    import os
    main()
