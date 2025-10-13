#!/usr/bin/env python3
"""
Test MCP server STDIO communication directly.
Sends JSON-RPC messages and displays responses.
"""

import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path

def send_and_receive(process, request, description):
    """Send a JSON-RPC request and read the response."""
    print("\n" + "="*60)
    print(f"📤 SENDING: {description}")
    print("="*60)
    print(json.dumps(request, indent=2))

    # Send the request (newline-delimited JSON)
    request_str = json.dumps(request) + "\n"
    process.stdin.write(request_str)
    process.stdin.flush()

    print("\n" + "="*60)
    print("📥 RESPONSE:")
    print("="*60)

    # Read the response (newline-delimited JSON)
    try:
        response_line = process.stdout.readline()
        if response_line:
            response = json.loads(response_line)
            print(json.dumps(response, indent=2))
            return response
        else:
            print("⚠️  No response received")
            return None
    except json.JSONDecodeError as e:
        print(f"❌ JSON decode error: {e}")
        print(f"Raw line: {response_line}")
        return None
    except Exception as e:
        print(f"❌ Error: {e}")
        return None

def main():
    print("="*60)
    print("MCP Server STDIO Communication Test")
    print("="*60)

    # Build the server
    print("\n📦 Building MCP server...")
    result = subprocess.run(
        ["cargo", "build", "--release"],
        capture_output=True,
        text=True
    )
    if result.returncode != 0:
        print(f"❌ Build failed:\n{result.stderr}")
        return 1
    print("✅ Build complete")

    # Path to binary
    binary = Path("./target/release/debugger_mcp")
    if not binary.exists():
        print(f"❌ Binary not found at {binary}")
        return 1

    # Create test file
    test_dir = Path(tempfile.mkdtemp())
    test_file = test_dir / "test.py"
    test_file.write_text("""def hello(name):
    print(f"Hello, {name}!")

if __name__ == "__main__":
    hello("World")
""")
    print(f"\n📁 Test directory: {test_dir}")
    print(f"✅ Created {test_file}")

    # Start MCP server
    print(f"\n🚀 Starting MCP server: {binary} serve")
    process = subprocess.Popen(
        [str(binary), "serve"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1  # Line buffered
    )

    try:
        # Give it a moment to start
        time.sleep(0.5)

        # 1. Initialize
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0"
                }
            }
        }
        init_response = send_and_receive(process, init_request, "Initialize")

        if not init_response:
            print("\n❌ Failed to get initialize response")
            return 1

        # 2. List tools
        tools_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }
        tools_response = send_and_receive(process, tools_request, "List Tools")

        if tools_response and "result" in tools_response:
            tools = tools_response["result"].get("tools", [])
            print(f"\n✅ Found {len(tools)} tools:")
            for tool in tools:
                print(f"  - {tool.get('name')}: {tool.get('description', 'No description')}")

        # 3. List resources
        resources_request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "resources/list",
            "params": {}
        }
        resources_response = send_and_receive(process, resources_request, "List Resources")

        if resources_response and "result" in resources_response:
            resources = resources_response["result"].get("resources", [])
            print(f"\n✅ Found {len(resources)} resources:")
            for resource in resources:
                print(f"  - {resource.get('uri')}: {resource.get('name', 'No name')}")

        # 4. Try to start debugging session
        debug_request = {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "debugger_start",
                "arguments": {
                    "language": "python",
                    "program": str(test_file),
                    "args": [],
                    "stopOnEntry": True
                }
            }
        }
        debug_response = send_and_receive(process, debug_request, "Start Debugging")

        if debug_response:
            if "result" in debug_response:
                print("\n✅ Debugging session started successfully!")
                # Parse session ID from response
                result_content = debug_response["result"].get("content", [])
                if result_content and len(result_content) > 0:
                    text = result_content[0].get("text", "")
                    print(f"Response: {text}")
            elif "error" in debug_response:
                error = debug_response["error"]
                print(f"\n⚠️  Error starting debugging: {error.get('message')}")
                print(f"Error code: {error.get('code')}")

        print("\n" + "="*60)
        print("✅ Test complete!")
        print("="*60)

    finally:
        # Cleanup
        process.terminate()
        process.wait(timeout=2)

        # Read any stderr output
        stderr = process.stderr.read()
        if stderr:
            print("\n📋 Server stderr output:")
            print(stderr)

    return 0

if __name__ == "__main__":
    sys.exit(main())
