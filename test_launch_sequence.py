#!/usr/bin/env python3
"""
Test the complete DAP launch sequence to understand what debugpy expects.
"""
import subprocess
import json
import sys
import os

def send_message(proc, msg):
    """Send a DAP message to the adapter."""
    content = json.dumps(msg)
    header = f"Content-Length: {len(content)}\r\n\r\n"
    message = (header + content).encode('utf-8')
    print(f">>> {msg['type'].upper()} {msg.get('command', msg.get('event', ''))}", file=sys.stderr)
    proc.stdin.write(message)
    proc.stdin.flush()

def read_message(proc, timeout_secs=5):
    """Read a DAP message from the adapter."""
    import select

    # Check if data is available
    if sys.platform != 'win32':
        ready, _, _ = select.select([proc.stdout], [], [], timeout_secs)
        if not ready:
            print(f"!!! TIMEOUT after {timeout_secs}s", file=sys.stderr)
            return None

    try:
        # Read headers
        headers = {}
        while True:
            line = proc.stdout.readline().decode('utf-8')
            if not line:
                return None
            if line == '\r\n' or line == '\n':
                break
            if ':' in line:
                key, value = line.strip().split(':', 1)
                headers[key.strip()] = value.strip()

        # Read content
        content_length = int(headers.get('Content-Length', 0))
        content = proc.stdout.read(content_length).decode('utf-8')
        msg = json.loads(content)
        msg_type = msg['type'].upper()
        desc = msg.get('command', msg.get('event', ''))
        print(f"<<< {msg_type} {desc}", file=sys.stderr)
        return msg
    except Exception as e:
        print(f"!!! Error reading: {e}", file=sys.stderr)
        return None

def main():
    fizzbuzz = os.path.join(os.path.dirname(__file__), 'tests', 'fixtures', 'fizzbuzz.py')

    print("=== DAP Launch Sequence Test ===\n", file=sys.stderr)

    proc = subprocess.Popen(
        ['python', '-m', 'debugpy.adapter'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=sys.stderr
    )

    try:
        # Step 1: Initialize
        print("\n[1] Initialize", file=sys.stderr)
        send_message(proc, {
            "seq": 1,
            "type": "request",
            "command": "initialize",
            "arguments": {
                "clientID": "test",
                "clientName": "Test",
                "adapterID": "debugpy",
                "linesStartAt1": True,
                "columnsStartAt1": True,
                "pathFormat": "path"
            }
        })

        # Read messages until we get initialize response
        for i in range(10):
            msg = read_message(proc, timeout_secs=2)
            if not msg:
                break
            if msg.get('type') == 'response' and msg.get('command') == 'initialize':
                print("    ✅ Got initialize response\n", file=sys.stderr)
                break

        # Step 2: Launch
        print("[2] Launch", file=sys.stderr)
        send_message(proc, {
            "seq": 2,
            "type": "request",
            "command": "launch",
            "arguments": {
                "request": "launch",
                "type": "python",
                "program": fizzbuzz,
                "args": [],
                "console": "internalConsole",
                "stopOnEntry": False
            }
        })

        # Read messages - looking for launch response
        print("    Waiting for launch response...", file=sys.stderr)
        for i in range(10):
            msg = read_message(proc, timeout_secs=2)
            if not msg:
                print("    ❌ TIMEOUT waiting for launch response!", file=sys.stderr)
                break
            if msg.get('type') == 'response' and msg.get('command') == 'launch':
                print("    ✅ Got launch response\n", file=sys.stderr)
                break

        # Step 3: ConfigurationDone
        print("[3] ConfigurationDone", file=sys.stderr)
        send_message(proc, {
            "seq": 3,
            "type": "request",
            "command": "configurationDone",
            "arguments": {}
        })

        # Read messages
        for i in range(5):
            msg = read_message(proc, timeout_secs=2)
            if not msg:
                break
            if msg.get('type') == 'response' and msg.get('command') == 'configurationDone':
                print("    ✅ Got configurationDone response\n", file=sys.stderr)
                break

        print("\n=== Test Complete ===", file=sys.stderr)

    finally:
        proc.terminate()
        proc.wait()

if __name__ == '__main__':
    main()
