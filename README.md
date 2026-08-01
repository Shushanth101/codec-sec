# CodecSec: Lightweight Secure Code Execution Engine

CodecSec is a production-quality, secure code execution engine built in Rust using Axum, Tokio, and the `isolate` sandboxing utility. It is designed as a high-performance, low-RAM alternative to Judge0, executing untrusted code securely inside sandboxed environments with strict resource constraints.

## Architecture Overview

CodecSec is structured using a modular, layered architecture:

```
HTTP API (POST /submissions, GET /submissions/{id}, POST /execute?wait=true)
    │
    ▼
Request Validation (API Layer)
    │
    ▼
Submission Store & Job Queue (Decoupled InMemory initially, ready for Redis/RabbitMQ)
    │
    ▼
Worker Pool (Tokio concurrent worker tasks)
    │
    ▼
Sandbox Manager (Maintains pool of isolate boxes to prevent concurrency conflicts)
    │
    ▼
Compiler & Executor (Runs commands inside isolate sandbox)
    │
    ▼
Result Collector (Parses meta file and captures stdout/stderr)
```

---

## Setup & Running

To run the entire engine inside Docker:

```bash
docker compose up --build
```

The server will start and listen on port `54054`.

---

## API Documentation & Test Request Bodies

Here are the endpoints and request bodies you can use to test with Postman or Curl.

### 1. Get Supported Runtimes
*   **Method**: `GET`
*   **Path**: `/runtimes`
*   **Response**: Lists all registered language compilers/interpreters and whether they require compilation.

---

### 2. Synchronous Execution (Ideal for Online Compilers)
*   **Method**: `POST`
*   **Path**: `/execute?wait=true`
*   **Headers**: `Content-Type: application/json`

#### C++ Example Request Body
```json
{
  "language": "cpp",
  "source": "#include <iostream>\n\nint main() {\n    std::cout << \"Hello from CodecSec C++!\" << std::endl;\n    return 0;\n}",
  "stdin": "",
  "time_limit_ms": 2000,
  "memory_limit_kb": 262144
}
```

#### Python Example Request Body
```json
{
  "language": "python",
  "source": "import sys\nprint('Hello from Python!')\nprint('Stderr output', file=sys.stderr)",
  "stdin": "",
  "time_limit_ms": 2000,
  "memory_limit_kb": 262144
}
```

#### Java Example Request Body
```json
{
  "language": "java",
  "source": "public class Main {\n    public static void main(String[] args) {\n        System.out.println(\"Hello from Java Main!\");\n    }\n}",
  "stdin": "",
  "time_limit_ms": 2000,
  "memory_limit_kb": 262144
}
```

#### Rust Example Request Body
```json
{
  "language": "rust",
  "source": "fn main() {\n    println!(\"Hello from Rust!\");\n}",
  "stdin": "",
  "time_limit_ms": 2000,
  "memory_limit_kb": 262144
}
```

#### Node.js Example Request Body
```json
{
  "language": "node",
  "source": "console.log('Hello from Node.js!');",
  "stdin": "",
  "time_limit_ms": 2000,
  "memory_limit_kb": 262144
}
```

#### Ruby Example Request Body
```json
{
  "language": "ruby",
  "source": "puts 'Hello from Ruby!'",
  "stdin": "",
  "time_limit_ms": 2000,
  "memory_limit_kb": 262144
}
```

#### Response (JSON)
```json
{
  "stdout": "Hello from CodecSec C++!\n",
  "stderr": "",
  "compile_output": "",
  "exit_code": 0,
  "status": "Accepted",
  "time_ms": 4,
  "memory_kb": 3216
}
```

---

### 3. Asynchronous Execution (Enqueuing)
*   **Method**: `POST`
*   **Path**: `/submissions` (or `/execute` with `wait=false` or omitted)
*   **Headers**: `Content-Type: application/json`

#### Request Body
```json
{
  "language": "python",
  "source": "import time\ntime.sleep(1)\nprint('Done sleeping!')"
}
```

#### Response (JSON)
```json
{
  "id": "e4bca232-a50d-4074-b5f3-79d9e63e26bb",
  "status": "Queued"
}
```

---

### 4. Fetch Submission Status & Result
*   **Method**: `GET`
*   **Path**: `/submissions/{id}` (Replace `{id}` with the UUID returned from the enqueue call)

#### Response (JSON)
```json
{
  "id": "e4bca232-a50d-4074-b5f3-79d9e63e26bb",
  "language": "python",
  "status": "Accepted",
  "stdout": "Done sleeping!\n",
  "stderr": "",
  "compile_output": "",
  "exit_code": 0,
  "time_ms": 1005,
  "memory_kb": 4124
}
```

---

## Testing Resource Limits

You can test error handling and sandboxing constraints with these request bodies under `/execute?wait=true`:

### Time Limit Exceeded (TLE)
```json
{
  "language": "python",
  "source": "while True: pass",
  "time_limit_ms": 1000
}
```
*   **Result Status**: `Time Limit Exceeded`

### Memory Limit Exceeded (MLE)
```json
{
  "language": "python",
  "source": "arr = [0] * (100 * 1024 * 1024) # Allocates huge list",
  "memory_limit_kb": 16384
}
```
*   **Result Status**: `Memory Limit Exceeded`

### Runtime Error (RE)
```json
{
  "language": "python",
  "source": "x = 1 / 0"
}
```
*   **Result Status**: `Runtime Error`

### Compile Error (CE)
```json
{
  "language": "cpp",
  "source": "int main() { broken_code; }"
}
```
*   **Result Status**: `Compile Error`

---

## Adding a New Runtime Language

Adding a new language is modular and doesn't require modifying the API or orchestrator layers:

1. Create a new file under `src/runtime/` (e.g. `src/runtime/go.rs`).
2. Implement the `Runtime` trait:
   ```rust
   pub struct GoRuntime;
   
   impl Runtime for GoRuntime {
       fn id(&self) -> &str { "go" }
       fn name(&self) -> &str { "Go" }
       fn extension(&self) -> &str { "go" }
       fn compiled(&self) -> bool { true }
       
       fn compile_command(&self, source_file: &str, output_file: &str) -> Option<Vec<String>> {
           Some(vec![
               "/usr/bin/go".to_string(),
               "build".to_string(),
               "-o".to_string(),
               output_file.to_string(),
               source_file.to_string()
           ])
       }
       
       fn execute_command(&self, source_or_exec_file: &str) -> Vec<String> {
           vec![source_or_exec_file.to_string()]
       }
   }
   ```
3. Expose the submodule in `src/runtime/mod.rs`:
   ```rust
   pub mod go;
   ```
4. Register the new runtime in `src/runtime/registry.rs`:
   ```rust
   runtimes.insert("go".to_string(), Arc::new(GoRuntime::new()));
   ```
5. Ensure the compiler/runtime package (e.g., `golang-go`) is installed in the `Dockerfile`.
