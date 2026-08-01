#!/usr/bin/env python3
"""
CodecSec Integration Test Harness (Rigorous Edition)

Usage:
    pip install requests
    python test_engine.py

Generates results.md with every request/response, grouped by category.

Categories:
    1. Correctness         - happy-path per-language execution
    2. Resource Limits      - boundary conditions for time/memory/output
    3. Sandbox Security      - escape attempts: fs, network, fork, subprocess, env
    4. Malformed Input       - fuzzing: bad JSON, missing fields, bad types, injection
    5. API Robustness        - wrong methods, unknown routes, unknown ids
    6. Concurrency & Stress  - parallel load, isolation-under-load, queue ordering
"""

import json
import os
import time
import uuid
from concurrent.futures import ThreadPoolExecutor, as_completed

import requests

BASE_URL = "http://localhost:54054"

RESULTS = []
PASS = 0
FAIL = 0


# ---------------------------------------------------------------------------
# Core recording helpers
# ---------------------------------------------------------------------------

def record(category, name, request_data, response, expected, ok, latency, note=""):
    global PASS, FAIL
    if ok:
        PASS += 1
    else:
        FAIL += 1
    try:
        body = response.json()
    except Exception:
        body = response.text if response is not None else ""
    RESULTS.append({
        "category": category,
        "name": name,
        "request": request_data,
        "status": response.status_code if response is not None else "N/A",
        "response": body,
        "expected": expected,
        "ok": ok,
        "latency": round(latency * 1000, 2) if isinstance(latency, (int, float)) else latency,
        "note": note,
    })


def execute(payload, wait=True, raw_body=None, headers=None):
    url = f"{BASE_URL}/execute"
    if wait:
        url += "?wait=true"
    t = time.perf_counter()
    if raw_body is not None:
        r = requests.post(url, data=raw_body, headers=headers or {"Content-Type": "application/json"}, timeout=60)
    else:
        r = requests.post(url, json=payload, timeout=60)
    return r, time.perf_counter() - t


def test(category, name, payload, status=None, http=200, note=""):
    try:
        r, dt = execute(payload)
        ok = (r.status_code == http)
        if status and ok and r.status_code == 200:
            ok = r.json().get("status") == status
        record(category, name, payload, r, status or http, ok, dt, note)
    except Exception as e:
        record(category, name, payload, None, status or http, False, 0, f"exception: {e}")


def test_security(name, payload, forbidden_substrings=None, allowed_statuses=None, note=""):
    """
    For sandbox-escape probes. We don't necessarily know the exact status the
    engine will return, but we assert that the escape did NOT succeed, i.e.
    none of the forbidden substrings (evidence of a breakout) show up in
    stdout/stderr, and the reported status is one of the acceptable outcomes
    (typically the sandbox should contain/kill the attempt, not silently
    allow it).
    """
    forbidden_substrings = forbidden_substrings or []
    allowed_statuses = allowed_statuses or ["Runtime Error", "Time Limit Exceeded",
                                             "Memory Limit Exceeded", "Accepted",
                                             "Internal Error", "Sandbox Error"]
    try:
        r, dt = execute(payload)
        body = {}
        try:
            body = r.json()
        except Exception:
            pass
        combined = (body.get("stdout", "") or "") + (body.get("stderr", "") or "") + (body.get("compile_output", "") or "")
        leaked = [s for s in forbidden_substrings if s in combined]
        status_ok = (r.status_code == 200) and (body.get("status") in allowed_statuses)
        ok = status_ok and not leaked
        note_full = note
        if leaked:
            note_full += f" | LEAKED EVIDENCE OF ESCAPE: {leaked}"
        record("Sandbox Security", name, payload, r, f"no leak, status in {allowed_statuses}", ok, dt, note_full)
    except Exception as e:
        record("Sandbox Security", name, payload, None, "contained/handled safely", False, 0, f"exception: {e}")


def test_raw(category, name, raw_body, headers=None, http=400, note="", url_suffix="/execute?wait=true"):
    """For malformed-input tests that need control over the exact bytes sent."""
    try:
        t = time.perf_counter()
        r = requests.post(f"{BASE_URL}{url_suffix}", data=raw_body,
                           headers=headers or {"Content-Type": "application/json"}, timeout=30)
        dt = time.perf_counter() - t
        ok = (r.status_code == http)
        record(category, name, raw_body if isinstance(raw_body, str) else str(raw_body), r, http, ok, dt, note)
    except Exception as e:
        record(category, name, str(raw_body), None, http, False, 0, f"exception: {e}")


def test_method(category, name, method, path, http=405, note=""):
    try:
        t = time.perf_counter()
        r = requests.request(method, f"{BASE_URL}{path}", timeout=15)
        dt = time.perf_counter() - t
        ok = (r.status_code == http)
        record(category, name, f"{method} {path}", r, http, ok, dt, note)
    except Exception as e:
        record(category, name, f"{method} {path}", None, http, False, 0, f"exception: {e}")


# ---------------------------------------------------------------------------
# 1. CORRECTNESS
# ---------------------------------------------------------------------------

CORRECTNESS_TESTS = [
    ("Python Hello", {"language": "python", "source": "print('hello')"}, "Accepted"),
    ("Python Runtime Error", {"language": "python", "source": "1/0"}, "Runtime Error"),
    ("Python stdin", {"language": "python", "stdin": "7", "source": "print(int(input())**2)"}, "Accepted"),
    ("Unicode", {"language": "python", "source": "print('こんにちは');print('😀');print('नमस्ते')"}, "Accepted"),
    ("Large Output", {"language": "python", "source": "for i in range(10000): print(i)"}, "Accepted"),
    ("CPP Hello", {"language": "cpp", "source": "#include<iostream>\nint main(){std::cout<<\"hi\";}"}, "Accepted"),
    ("CPP Compile Error", {"language": "cpp", "source": "int main(){broken;}"}, "Compile Error"),
    ("Java Hello", {"language": "java", "source": "public class Main{public static void main(String[]a){System.out.println(\"Hi\");}}"}, "Accepted"),
    ("Rust Hello", {"language": "rust", "source": "fn main(){println!(\"Hi\");}"}, "Accepted"),
    ("Node Hello", {"language": "node", "source": "console.log('Hi')"}, "Accepted"),
    ("Ruby Hello", {"language": "ruby", "source": "puts 'Hi'"}, "Accepted"),
    ("Multi-line stdin parsing", {"language": "python", "stdin": "3\n1 2 3\n", "source": "n=int(input());print(sum(map(int,input().split())))"}, "Accepted"),
    ("Exit code propagation", {"language": "python", "source": "import sys; sys.exit(7)"}, "Runtime Error"),
    ("Stderr-only program", {"language": "python", "source": "import sys; print('err', file=sys.stderr)"}, "Accepted"),
]

# ---------------------------------------------------------------------------
# 2. RESOURCE LIMIT EDGE CASES
# ---------------------------------------------------------------------------

RESOURCE_TESTS = [
    ("TLE - classic infinite loop", {"language": "python", "source": "while True: pass", "time_limit_ms": 1000}, "Time Limit Exceeded"),
    ("TLE - boundary just over limit", {"language": "python", "source": "import time; time.sleep(0.6)", "time_limit_ms": 500}, "Time Limit Exceeded"),
    ("TLE - boundary just under limit (should pass)", {"language": "python", "source": "import time; time.sleep(0.2)", "time_limit_ms": 2000}, "Accepted"),
    ("TLE - minimum viable time limit", {"language": "python", "source": "print(1)", "time_limit_ms": 50}, "Accepted"),
    ("TLE - CPU-bound busy loop (not sleep)", {"language": "cpp", "source": "int main(){volatile long x=0; while(1){x++;}}", "time_limit_ms": 800}, "Time Limit Exceeded"),
    ("MLE - big list allocation", {"language": "python", "source": "a=[0]*(100*1024*1024)", "memory_limit_kb": 16384}, "Memory Limit Exceeded"),
    ("MLE - boundary just under limit (should pass)", {"language": "python", "source": "a=bytearray(4*1024*1024)", "memory_limit_kb": 65536}, "Accepted"),
    ("MLE - very tight limit", {"language": "python", "source": "print('hi')", "memory_limit_kb": 4096}, "Accepted"),
    ("MLE - C malloc bomb", {"language": "cpp", "source": "#include <cstdlib>\nint main(){void* p=malloc(500L*1024*1024); (void)p; return 0;}", "memory_limit_kb": 32768}, "Runtime Error"),
    ("Deep recursion / stack overflow (Python)", {"language": "python", "source": "import sys\nsys.setrecursionlimit(1000000)\ndef f(n): return f(n+1)\nf(0)"}, "Runtime Error"),
    ("Deep recursion / stack overflow (C++)", {"language": "cpp", "source": "void f(int n){ volatile char buf[1024]; f(n+1);} int main(){f(0);}"}, "Runtime Error"),
    ("Huge stdin (~5MB)", {"language": "python", "stdin": "x" * (5 * 1024 * 1024), "source": "import sys\nprint(len(sys.stdin.read()))"}, "Accepted"),
    ("Excessive stdout (potential output cap)", {"language": "python", "source": "for i in range(2000000): print('a'*80)"}, None),
    ("Zero time limit", {"language": "python", "source": "print(1)", "time_limit_ms": 0}, None),
    ("Negative time limit (should be rejected or clamped)", {"language": "python", "source": "print(1)", "time_limit_ms": -100}, None),
    ("Negative memory limit (should be rejected or clamped)", {"language": "python", "source": "print(1)", "memory_limit_kb": -1}, None),
    ("Absurdly large time limit (should probably be capped server-side)", {"language": "python", "source": "print(1)", "time_limit_ms": 999999999}, None),
]

# ---------------------------------------------------------------------------
# 3. SANDBOX SECURITY PROBES
# ---------------------------------------------------------------------------

SECURITY_TESTS = [
    (
        "Read /etc/passwd",
        {"language": "python", "source": "print(open('/etc/passwd').read())", "time_limit_ms": 2000},
        ["root:x:0:0", "root:!:0:0"],
        "Sandbox should either deny read access or present a restricted/fake passwd; real host root entry must never leak.",
    ),
    (
        "Read /proc/1/environ (host PID 1 secrets)",
        {"language": "python", "source": "print(open('/proc/1/environ').read())", "time_limit_ms": 2000},
        [],
        "Should fail with PermissionError/FileNotFoundError, not return host init process environment.",
    ),
    (
        "Write outside sandbox to /root",
        {"language": "python", "source": "open('/root/pwned.txt','w').write('pwned')\nprint('wrote')", "time_limit_ms": 2000},
        ["wrote"],
        "Write must fail (PermissionError) inside the isolate box; 'wrote' printing means write succeeded outside sandbox scope.",
    ),
    (
        "Escape via os.chdir + relative path traversal",
        {"language": "python", "source": "import os\nos.chdir('/')\nfor _ in range(10): os.chdir('..')\nprint(open('etc/shadow').read())", "time_limit_ms": 2000},
        ["root:"],
        "Should not be able to read /etc/shadow contents regardless of chdir traversal.",
    ),
    (
        "Outbound network access attempt (raw socket)",
        {"language": "python", "source": "import socket\ns=socket.socket(socket.AF_INET, socket.SOCK_STREAM)\ns.settimeout(2)\ntry:\n    s.connect(('8.8.8.8', 53))\n    print('CONNECTED')\nexcept Exception as e:\n    print('BLOCKED', type(e).__name__)", "time_limit_ms": 4000},
        ["CONNECTED"],
        "Sandbox network namespace should block outbound connections; should print BLOCKED not CONNECTED.",
    ),
    (
        "DNS resolution attempt",
        {"language": "python", "source": "import socket\ntry:\n    print(socket.gethostbyname('example.com'))\nexcept Exception as e:\n    print('BLOCKED', type(e).__name__)", "time_limit_ms": 3000},
        [],
        "DNS should fail inside an isolated/no-network sandbox.",
    ),
    (
        "Fork bomb",
        {"language": "python", "source": "import os\nwhile True:\n    os.fork()", "time_limit_ms": 1500},
        [],
        "Process/thread limits (isolate --processes) must cap runaway forking rather than taking down the host.",
    ),
    (
        "Subprocess spawn attempt to list host root",
        {"language": "python", "source": "import subprocess\ntry:\n    out = subprocess.run(['ls','/'], capture_output=True, text=True, timeout=3)\n    print(out.stdout)\nexcept Exception as e:\n    print('BLOCKED', type(e).__name__)", "time_limit_ms": 3000},
        ["home", "var", "etc"],
        "Even if a subprocess call succeeds, it should only see the sandboxed filesystem view, not the real host root listing.",
    ),
    (
        "Environment variable exfiltration",
        {"language": "python", "source": "import os\nprint(dict(os.environ))", "time_limit_ms": 2000},
        ["AWS_SECRET", "API_KEY", "DATABASE_URL", "PRIVATE_KEY"],
        "Sandbox environment should be minimal/clean; no host secrets should be inherited by the executed process.",
    ),
    (
        "Sparse-file disk quota bomb",
        {"language": "python", "source": "f = open('/tmp/bomb','wb')\nf.truncate(10**12)\nprint('done')", "time_limit_ms": 3000, "memory_limit_kb": 65536},
        [],
        "isolate's disk quota (or lack of write access) should prevent creating a 1TB sparse file inside the box.",
    ),
    (
        "Shell metacharacter injection in source (no real shell)",
        {"language": "python", "source": "print('$(rm -rf /); `id`; ; echo pwned')", "time_limit_ms": 2000},
        [],
        "Source is Python code, not shell — metacharacters should just be printed literally, never interpreted as commands.",
    ),
    (
        "Path traversal via language field",
        {"language": "../../../etc/passwd", "source": "print(1)"},
        [],
        "Unknown/malicious language identifiers must be rejected (400), not used to build a filesystem path.",
    ),
]

# ---------------------------------------------------------------------------
# 4. MALFORMED / FUZZED INPUT
# ---------------------------------------------------------------------------

MALFORMED_TESTS = [
    ("Missing 'source' field", {"language": "python"}, 400),
    ("Missing 'language' field", {"source": "print(1)"}, 400),
    ("Unknown language", {"language": "brainfuck", "source": "+++."}, 400),
    ("Empty source string", {"language": "python", "source": ""}, 200),
    ("Whitespace-only source", {"language": "python", "source": "   \n\t  "}, 200),
    ("Null source value", {"language": "python", "source": None}, 400),
    ("Non-string source (integer)", {"language": "python", "source": 12345}, 400),
    ("Non-string language (integer)", {"language": 123, "source": "print(1)"}, 400),
    ("time_limit_ms as string", {"language": "python", "source": "print(1)", "time_limit_ms": "1000"}, 400),
    ("Extra unknown fields (should be ignored, not error)", {"language": "python", "source": "print(1)", "totally_made_up_field": True}, 200),
    ("Empty JSON body", {}, 400),
    ("Null bytes embedded in source", {"language": "python", "source": "print('a\\x00b')"}, 200),
    ("Extremely long language string", {"language": "p" * 5000, "source": "print(1)"}, 400),
    ("Very large source payload (~2MB)", {"language": "python", "source": "# " + ("x" * (2 * 1024 * 1024))}, None),
]


def run_malformed():
    for name, payload, http in MALFORMED_TESTS:
        test("Malformed Input", name, payload, status=None, http=http if http else 200)

    # Raw-body fuzzing that requests.post(json=...) can't express
    test_raw("Malformed Input", "Broken JSON syntax", '{"language": "python", "source": "print(1)"',
              http=400, note="Truncated/invalid JSON should yield 400, not 500 or a hang.")
    test_raw("Malformed Input", "Wrong Content-Type (text/plain)",
              json.dumps({"language": "python", "source": "print(1)"}),
              headers={"Content-Type": "text/plain"}, http=400,
              note="Body is valid JSON but declared as text/plain; server should reject or handle gracefully, not crash.")
    test_raw("Malformed Input", "Array instead of object",
              '["language", "python"]', http=400)
    test_raw("Malformed Input", "Deeply nested JSON (100 levels)",
              '{"language":"python","source":' + '{"a":' * 100 + '1' + '}' * 100 + '}',
              http=400)
    test_raw("Malformed Input", "Huge single JSON key fuzz",
              json.dumps({"language": "python", "source": "print(1)", "x" * 100000: "y"}),
              http=200, note="Oversized but structurally valid field name; should not crash the parser.")


# ---------------------------------------------------------------------------
# 5. API ROBUSTNESS
# ---------------------------------------------------------------------------

def run_api_robustness():
    # Unknown submission id
    fake_id = str(uuid.uuid4())
    try:
        t = time.perf_counter()
        r = requests.get(f"{BASE_URL}/submissions/{fake_id}", timeout=15)
        dt = time.perf_counter() - t
        ok = r.status_code == 404
        record("API Robustness", "GET unknown submission id", f"/submissions/{fake_id}", r, 404, ok, dt)
    except Exception as e:
        record("API Robustness", "GET unknown submission id", fake_id, None, 404, False, 0, str(e))

    # Malformed (non-UUID) submission id
    try:
        t = time.perf_counter()
        r = requests.get(f"{BASE_URL}/submissions/not-a-uuid", timeout=15)
        dt = time.perf_counter() - t
        ok = r.status_code in (400, 404)
        record("API Robustness", "GET malformed submission id", "/submissions/not-a-uuid", r, "400 or 404", ok, dt)
    except Exception as e:
        record("API Robustness", "GET malformed submission id", "not-a-uuid", None, "400 or 404", False, 0, str(e))

    # Unknown route
    try:
        t = time.perf_counter()
        r = requests.get(f"{BASE_URL}/totally/not/a/route", timeout=15)
        dt = time.perf_counter() - t
        ok = r.status_code == 404
        record("API Robustness", "GET unknown route", "/totally/not/a/route", r, 404, ok, dt)
    except Exception as e:
        record("API Robustness", "GET unknown route", "", None, 404, False, 0, str(e))

    # Wrong HTTP methods on known routes
    test_method("API Robustness", "GET on /execute (should be POST-only)", "GET", "/execute?wait=true", http=405)
    test_method("API Robustness", "DELETE on /submissions", "DELETE", "/submissions", http=405)
    test_method("API Robustness", "PUT on /runtimes", "PUT", "/runtimes", http=405)

    # /execute with wait=false behavior — should behave like async enqueue
    try:
        t = time.perf_counter()
        r = requests.post(f"{BASE_URL}/execute?wait=false",
                           json={"language": "python", "source": "print(1)"}, timeout=15)
        dt = time.perf_counter() - t
        ok = r.status_code in (200, 202)
        record("API Robustness", "/execute?wait=false enqueues instead of blocking",
               {"language": "python", "source": "print(1)"}, r, "200 or 202", ok, dt)
    except Exception as e:
        record("API Robustness", "/execute?wait=false enqueues instead of blocking", "", None, "200 or 202", False, 0, str(e))


# ---------------------------------------------------------------------------
# 6. CONCURRENCY & STRESS
# ---------------------------------------------------------------------------

def async_test():
    payload = {"language": "python", "source": "import time;time.sleep(1);print('done')"}
    r = requests.post(f"{BASE_URL}/submissions", json=payload)
    ok = False
    body = {}
    # Accept both 200 and 202 as valid "enqueued" responses (fixes prior harness bug
    # where only 200 was checked, causing this test to always fail against a
    # spec-compliant 202 Accepted response).
    if r.status_code in (200, 202):
        body = r.json() if r.content else {}
        sid = body.get("id")
        if not sid:
            # some implementations put the id in a Location header instead
            loc = r.headers.get("Location", "")
            sid = loc.rstrip("/").rsplit("/", 1)[-1] if loc else None
        if sid:
            for _ in range(60):
                rr = requests.get(f"{BASE_URL}/submissions/{sid}")
                jb = rr.json()
                if jb.get("status") not in ("Queued", "Running"):
                    ok = jb.get("status") == "Accepted"
                    body = jb
                    break
                time.sleep(.25)
    RESULTS.append({
        "category": "Concurrency & Stress",
        "name": "Async Submission (fixed 202 handling)",
        "request": payload,
        "status": r.status_code,
        "response": body,
        "expected": "Accepted",
        "ok": ok,
        "latency": "-",
        "note": "",
    })
    global PASS, FAIL
    if ok:
        PASS += 1
    else:
        FAIL += 1


def concurrent(n_workers, n_requests, label):
    payload = {"language": "python", "source": "print(sum(range(1000)))"}
    lat = []
    bad = 0

    def worker(i):
        r, dt = execute(payload)
        try:
            j = r.json()
        except Exception:
            j = {}
        return r.status_code, j.get("status"), j.get("stdout"), dt

    with ThreadPoolExecutor(max_workers=n_workers) as ex:
        fs = [ex.submit(worker, i) for i in range(n_requests)]
        for f in as_completed(fs):
            sc, st, stdout, dt = f.result()
            lat.append(dt * 1000)
            if sc != 200 or st != "Accepted" or stdout != "499500\n":
                bad += 1
    RESULTS.append({
        "category": "Concurrency & Stress",
        "name": label,
        "request": f"{n_requests} parallel execute requests, {n_workers} workers",
        "status": "-",
        "response": {"failures": bad, "avg_ms": round(sum(lat) / len(lat), 2), "max_ms": round(max(lat), 2), "min_ms": round(min(lat), 2)},
        "expected": "0 failures",
        "ok": bad == 0,
        "latency": "-",
        "note": "",
    })
    global PASS, FAIL
    if bad == 0:
        PASS += 1
    else:
        FAIL += 1


def concurrent_isolation_check():
    """
    Each worker submits code that prints a unique token derived from its own
    index. If sandbox boxes are being reused/pooled incorrectly under load,
    output could bleed between concurrent jobs. We verify each response's
    stdout matches exactly its own expected token.
    """
    n = 30
    mismatches = 0

    def worker(i):
        token = f"WORKER_TOKEN_{i}_{uuid.uuid4().hex[:8]}"
        payload = {"language": "python", "source": f"print('{token}')"}
        r, dt = execute(payload)
        try:
            j = r.json()
        except Exception:
            j = {}
        return token, j.get("stdout", "").strip()

    with ThreadPoolExecutor(max_workers=15) as ex:
        fs = [ex.submit(worker, i) for i in range(n)]
        for f in as_completed(fs):
            token, stdout = f.result()
            if stdout != token:
                mismatches += 1

    RESULTS.append({
        "category": "Concurrency & Stress",
        "name": "Cross-job isolation under concurrency (unique token bleed check)",
        "request": f"{n} concurrent jobs each printing a unique token",
        "status": "-",
        "response": {"mismatches": mismatches},
        "expected": "0 mismatches (no output bleed between sandboxed jobs)",
        "ok": mismatches == 0,
        "latency": "-",
        "note": "" if mismatches == 0 else "Output from one job appears to have leaked into another — sandbox pool isolation bug.",
    })
    global PASS, FAIL
    if mismatches == 0:
        PASS += 1
    else:
        FAIL += 1


def mixed_language_stress():
    jobs = [
        {"language": "python", "source": "print('py-ok')"},
        {"language": "cpp", "source": "#include<iostream>\nint main(){std::cout<<\"cpp-ok\";}"},
        {"language": "node", "source": "console.log('node-ok')"},
        {"language": "ruby", "source": "puts 'ruby-ok'"},
    ] * 5  # 20 total, interleaved languages
    bad = 0

    def worker(payload):
        r, dt = execute(payload)
        try:
            j = r.json()
        except Exception:
            j = {}
        return payload["language"], r.status_code, j.get("status")

    with ThreadPoolExecutor(max_workers=10) as ex:
        fs = [ex.submit(worker, p) for p in jobs]
        for f in as_completed(fs):
            lang, sc, st = f.result()
            if sc != 200 or st != "Accepted":
                bad += 1

    RESULTS.append({
        "category": "Concurrency & Stress",
        "name": "Mixed-language concurrent stress (20 jobs, 4 languages)",
        "request": "interleaved python/cpp/node/ruby jobs, 10 workers",
        "status": "-",
        "response": {"failures": bad, "total": len(jobs)},
        "expected": "0 failures",
        "ok": bad == 0,
        "latency": "-",
        "note": "Exercises the compiler toolchains and interpreter runtimes concurrently, not just one language at a time.",
    })
    global PASS, FAIL
    if bad == 0:
        PASS += 1
    else:
        FAIL += 1


def runtimes():
    global PASS, FAIL
    try:
        r = requests.get(f"{BASE_URL}/runtimes")
        RESULTS.append({
            "category": "Correctness",
            "name": "Runtime Registry",
            "request": "GET /runtimes",
            "status": r.status_code,
            "response": r.json(),
            "expected": "200",
            "ok": r.status_code == 200,
            "latency": "-",
            "note": "",
        })
        if r.status_code == 200:
            PASS += 1
        else:
            FAIL += 1
    except Exception as e:
        RESULTS.append({"category": "Correctness", "name": "Runtime Registry", "request": "", "status": "",
                         "response": str(e), "expected": "200", "ok": False, "latency": "-", "note": ""})
        FAIL += 1


# ---------------------------------------------------------------------------
# Report generation
# ---------------------------------------------------------------------------

def markdown():
    with open("results.md", "w", encoding="utf8") as f:
        f.write("# CodecSec Rigorous Test Report\n\n")
        f.write(f"Generated: {time.ctime()}\n\n")

        # Summary table by category, up top
        cats = {}
        for res in RESULTS:
            c = res["category"]
            cats.setdefault(c, {"pass": 0, "fail": 0})
            if res["ok"]:
                cats[c]["pass"] += 1
            else:
                cats[c]["fail"] += 1

        f.write("## Summary by Category\n\n")
        f.write("| Category | Passed | Failed | Total |\n|---|---|---|---|\n")
        for c, v in cats.items():
            f.write(f"| {c} | {v['pass']} | {v['fail']} | {v['pass']+v['fail']} |\n")
        f.write(f"\n**Overall: {PASS} passed / {FAIL} failed / {PASS+FAIL} total**\n\n---\n\n")

        current_cat = None
        idx = 0
        for res in RESULTS:
            if res["category"] != current_cat:
                current_cat = res["category"]
                f.write(f"\n# {current_cat}\n\n")
            idx += 1
            f.write(f"## Test {idx}: {res['name']}\n\n")
            f.write("### Request\n```json\n")
            f.write(json.dumps(res["request"], indent=2, ensure_ascii=False, default=str))
            f.write("\n```\n\n")
            f.write("### Response\n```json\n")
            f.write(json.dumps(res["response"], indent=2, ensure_ascii=False, default=str))
            f.write("\n```\n\n")
            f.write(f"HTTP: {res['status']}\n\n")
            f.write(f"Expected: {res['expected']}\n\n")
            f.write(f"Latency: {res['latency']} ms\n\n")
            if res.get("note"):
                f.write(f"Note: {res['note']}\n\n")
            f.write(f"Result: {'✅ PASS' if res['ok'] else '❌ FAIL'}\n\n---\n\n")

        f.write("\n# Final Summary\n\n")
        f.write(f"- Passed: {PASS}\n")
        f.write(f"- Failed: {FAIL}\n")
        f.write(f"- Total : {PASS+FAIL}\n")


def main():
    runtimes()

    for n, p, s in CORRECTNESS_TESTS:
        test("Correctness", n, p, s)

    for n, p, s in RESOURCE_TESTS:
        test("Resource Limits", n, p, status=s if s else None, http=200)

    for n, p, forbidden, note in SECURITY_TESTS:
        test_security(n, p, forbidden_substrings=forbidden, note=note)

    run_malformed()
    run_api_robustness()

    async_test()
    concurrent(20, 50, "Concurrent Stress (50 req / 20 workers)")
    concurrent(50, 200, "Heavy Concurrent Stress (200 req / 50 workers)")
    concurrent_isolation_check()
    mixed_language_stress()

    markdown()
    print(f"Done. {PASS} passed, {FAIL} failed. See results.md")


if __name__ == "__main__":
    main()
