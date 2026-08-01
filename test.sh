#!/bin/bash
set -eo pipefail

HOST="http://localhost:54054"

echo "=== CodecSec Verification Suite ==="

# Helper function to print success/failure
print_result() {
    local status=$1
    local name=$2
    if [ "$status" = "success" ]; then
        echo -e "[\e[32mPASS\e[0m] $name"
    else
        echo -e "[\e[31mFAIL\e[0m] $name"
        exit 1
    fi
}

echo "Testing GET /runtimes..."
RUNTIMES_RESP=$(curl -s -f "$HOST/runtimes")
if echo "$RUNTIMES_RESP" | grep -q "cpp" && echo "$RUNTIMES_RESP" | grep -q "python"; then
    print_result "success" "GET /runtimes returned cpp and python"
else
    print_result "fail" "GET /runtimes did not return expected list"
fi

# 1. C test
echo "Testing C (Synchronous)..."
C_RESP=$(curl -s -f -X POST "$HOST/execute?wait=true" \
  -H "Content-Type: application/json" \
  -d '{
    "language": "c",
    "source": "#include <stdio.h>\nint main() { printf(\"Hello from C\\n\"); return 0; }"
  }')
if echo "$C_RESP" | grep -q "Hello from C" && echo "$C_RESP" | grep -q '"status":"Accepted"'; then
    print_result "success" "C Execution"
else
    echo "C response: $C_RESP"
    print_result "fail" "C Execution"
fi

# 2. C++ test
echo "Testing C++ (Synchronous)..."
CPP_RESP=$(curl -s -f -X POST "$HOST/execute?wait=true" \
  -H "Content-Type: application/json" \
  -d '{
    "language": "cpp",
    "source": "#include <iostream>\nint main() { std::cout << \"Hello from C++\" << std::endl; return 0; }"
  }')
if echo "$CPP_RESP" | grep -q "Hello from C++" && echo "$CPP_RESP" | grep -q '"status":"Accepted"'; then
    print_result "success" "C++ Execution"
else
    echo "C++ response: $CPP_RESP"
    print_result "fail" "C++ Execution"
fi

# 3. Java test
echo "Testing Java (Synchronous)..."
JAVA_RESP=$(curl -s -f -X POST "$HOST/execute?wait=true" \
  -H "Content-Type: application/json" \
  -d '{
    "language": "java",
    "source": "public class Main {\n  public static void main(String[] args) {\n    System.out.println(\"Hello from Java\");\n  }\n}"
  }')
if echo "$JAVA_RESP" | grep -q "Hello from Java" && echo "$JAVA_RESP" | grep -q '"status":"Accepted"'; then
    print_result "success" "Java Execution"
else
    echo "Java response: $JAVA_RESP"
    print_result "fail" "Java Execution"
fi

# 4. Python test
echo "Testing Python (Synchronous)..."
PY_RESP=$(curl -s -f -X POST "$HOST/execute?wait=true" \
  -H "Content-Type: application/json" \
  -d '{
    "language": "python",
    "source": "print(\"Hello from Python\")"
  }')
if echo "$PY_RESP" | grep -q "Hello from Python" && echo "$PY_RESP" | grep -q '"status":"Accepted"'; then
    print_result "success" "Python Execution"
else
    echo "Python response: $PY_RESP"
    print_result "fail" "Python Execution"
fi

# 5. Node.js test
echo "Testing Node.js (Synchronous)..."
NODE_RESP=$(curl -s -f -X POST "$HOST/execute?wait=true" \
  -H "Content-Type: application/json" \
  -d '{
    "language": "node",
    "source": "console.log(\"Hello from Node.js\")"
  }')
if echo "$NODE_RESP" | grep -q "Hello from Node.js" && echo "$NODE_RESP" | grep -q '"status":"Accepted"'; then
    print_result "success" "Node.js Execution"
else
    echo "Node response: $NODE_RESP"
    print_result "fail" "Node.js Execution"
fi

# 6. Rust test
echo "Testing Rust (Synchronous)..."
RUST_RESP=$(curl -s -f -X POST "$HOST/execute?wait=true" \
  -H "Content-Type: application/json" \
  -d '{
    "language": "rust",
    "source": "fn main() { println!(\"Hello from Rust\"); }"
  }')
if echo "$RUST_RESP" | grep -q "Hello from Rust" && echo "$RUST_RESP" | grep -q '"status":"Accepted"'; then
    print_result "success" "Rust Execution"
else
    echo "Rust response: $RUST_RESP"
    print_result "fail" "Rust Execution"
fi

# 7. Ruby test
echo "Testing Ruby (Synchronous)..."
RUBY_RESP=$(curl -s -f -X POST "$HOST/execute?wait=true" \
  -H "Content-Type: application/json" \
  -d '{
    "language": "ruby",
    "source": "puts \"Hello from Ruby\""
  }')
if echo "$RUBY_RESP" | grep -q "Hello from Ruby" && echo "$RUBY_RESP" | grep -q '"status":"Accepted"'; then
    print_result "success" "Ruby Execution"
else
    echo "Ruby response: $RUBY_RESP"
    print_result "fail" "Ruby Execution"
fi

# 8. Asynchronous execution test
echo "Testing Asynchronous submission (POST /submissions -> GET /submissions/{id})..."
ASYNC_POST=$(curl -s -f -X POST "$HOST/submissions" \
  -H "Content-Type: application/json" \
  -d '{
    "language": "python",
    "source": "import time\ntime.sleep(1)\nprint(\"Async Completed\")"
  }')
SUB_ID=$(echo "$ASYNC_POST" | grep -oP '"id":"\K[^"]+')

if [ -n "$SUB_ID" ]; then
    echo "Enqueued submission ID: $SUB_ID"
    # Poll for result
    for i in {1..10}; do
        ASYNC_GET=$(curl -s -f "$HOST/submissions/$SUB_ID")
        STATUS=$(echo "$ASYNC_GET" | grep -oP '"status":"\K[^"]+')
        echo "Poll $i: Status is $STATUS"
        if [ "$STATUS" = "Accepted" ]; then
            break
        fi
        sleep 0.5
    done
    if echo "$ASYNC_GET" | grep -q "Async Completed"; then
        print_result "success" "Asynchronous Execution and Polling"
    else
        echo "Final get result: $ASYNC_GET"
        print_result "fail" "Asynchronous Execution and Polling"
    fi
else
    echo "Async POST response: $ASYNC_POST"
    print_result "fail" "Enqueuing Asynchronous Job"
fi

# 9. Time Limit Exceeded test
echo "Testing Time Limit Exceeded (TLE)..."
TLE_RESP=$(curl -s -f -X POST "$HOST/execute?wait=true" \
  -H "Content-Type: application/json" \
  -d '{
    "language": "python",
    "source": "import time\nwhile True: time.sleep(0.1)",
    "time_limit_ms": 1000
  }')
if echo "$TLE_RESP" | grep -q '"status":"Time Limit Exceeded"'; then
    print_result "success" "Time Limit Exceeded (TLE) enforcement"
else
    echo "TLE response: $TLE_RESP"
    print_result "fail" "Time Limit Exceeded (TLE) enforcement"
fi

# 10. Compile Error test
echo "Testing Compile Error..."
CE_RESP=$(curl -s -f -X POST "$HOST/execute?wait=true" \
  -H "Content-Type: application/json" \
  -d '{
    "language": "cpp",
    "source": "int main() { std::cout << invalid_code; }"
  }')
if echo "$CE_RESP" | grep -q '"status":"Compile Error"' && echo "$CE_RESP" | grep -q "invalid_code"; then
    print_result "success" "Compile Error capturing"
else
    echo "CE response: $CE_RESP"
    print_result "fail" "Compile Error capturing"
fi

# 11. Runtime Error test
echo "Testing Runtime Error..."
RE_RESP=$(curl -s -f -X POST "$HOST/execute?wait=true" \
  -H "Content-Type: application/json" \
  -d '{
    "language": "python",
    "source": "raise Exception(\"Crash!\")"
  }')
if echo "$RE_RESP" | grep -q '"status":"Runtime Error"' && echo "$RE_RESP" | grep -q "Crash!"; then
    print_result "success" "Runtime Error (Exception) capturing"
else
    echo "RE response: $RE_RESP"
    print_result "fail" "Runtime Error (Exception) capturing"
fi

echo "=== All tests completed successfully ==="
