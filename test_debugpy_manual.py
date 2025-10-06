#!/usr/bin/env python3
"""
Manual test to verify debugpy adapter communication.
This helps diagnose why our Rust integration test times out.
"""
import subprocess
import json
import sys

def send_message(proc, msg):
    """Send a DAP message to the adapter."""
    content = json.dumps(msg)
    header = f"Content-Length: {len(content)}\r\n\r\n"
    message = (header + content).encode('utf-8')
    print(f">>> Sending: {msg}", file=sys.stderr)
    proc.stdin.write(message)
    proc.stdin.flush()

def read_message(proc):
    """Read a DAP message from the adapter."""
    # Read headers
    headers = {}
    while True:
        line = proc.stdout.readline().decode('utf-8')
        if line == '\r\n':
            break
        if ':' in line:
            key, value = line.strip().split(':', 1)
            headers[key.strip()] = value.strip()

    # Read content
    content_length = int(headers.get('Content-Length', 0))
    content = proc.stdout.read(content_length).decode('utf-8')
    msg = json.loads(content)
    print(f"<<< Received: {msg}", file=sys.stderr)
    return msg

def main():
    print("Starting debugpy adapter...", file=sys.stderr)

    # Start the adapter
    proc = subprocess.Popen(
        ['python', '-m', 'debugpy.adapter'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=sys.stderr
    )

    try:
        # Send initialize request
        initialize_req = {
            "seq": 1,
            "type": "request",
            "command": "initialize",
            "arguments": {
                "clientID": "test-client",
                "clientName": "Test Client",
                "adapterID": "debugpy",
                "locale": "en-US",
                "linesStartAt1": True,
                "columnsStartAt1": True,
                "pathFormat": "path"
            }
        }

        send_message(proc, initialize_req)

        # Read messages until we get the response
        response = None
        for _ in range(5):  # Try up to 5 messages
            msg = read_message(proc)
            if msg.get('type') == 'response':
                response = msg
                break
            elif msg.get('type') == 'event':
                print(f"    (Event: {msg.get('event')})", file=sys.stderr)

        if response and response.get('success'):
            print("\n✅ SUCCESS: Adapter responded to initialize request!", file=sys.stderr)
            print(f"Capabilities: {response.get('body', {})}", file=sys.stderr)
        else:
            print(f"\n❌ FAILED: {response}", file=sys.stderr)

    except Exception as e:
        print(f"\n❌ ERROR: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
    finally:
        proc.terminate()
        proc.wait()

if __name__ == '__main__':
    main()
