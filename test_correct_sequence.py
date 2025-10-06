#!/usr/bin/env python3
"""Test the CORRECT DAP sequence per official spec"""
import subprocess, json, sys, os, select

def send_message(proc, msg):
    content = json.dumps(msg)
    header = f"Content-Length: {len(content)}\r\n\r\n"
    proc.stdin.write((header + content).encode('utf-8'))
    proc.stdin.flush()
    print(f">>> {msg.get('command', msg.get('event', ''))}", file=sys.stderr)

def read_message(proc, timeout_secs=2):
    if sys.platform != 'win32':
        ready, _, _ = select.select([proc.stdout], [], [], timeout_secs)
        if not ready:
            print(f"!!! TIMEOUT", file=sys.stderr)
            return None
    try:
        headers = {}
        while True:
            line = proc.stdout.readline().decode('utf-8')
            if not line or line in ['\r\n', '\n']:
                break
            if ':' in line:
                k, v = line.strip().split(':', 1)
                headers[k.strip()] = v.strip()

        length = int(headers.get('Content-Length', 0))
        content = proc.stdout.read(length).decode('utf-8')
        msg = json.loads(content)
        desc = f"{msg['type']} {msg.get('command', msg.get('event', ''))}"
        print(f"<<< {desc}", file=sys.stderr)
        return msg
    except Exception as e:
        print(f"!!! Error: {e}", file=sys.stderr)
        return None

fizzbuzz = os.path.join(os.path.dirname(__file__), 'tests', 'fixtures', 'fizzbuzz.py')
proc = subprocess.Popen(['python', '-m', 'debugpy.adapter'],
                       stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=sys.stderr)

try:
    print("\n=== CORRECT DAP SEQUENCE ===\n", file=sys.stderr)

    # Step 1: initialize
    print("[1] initialize", file=sys.stderr)
    send_message(proc, {"seq": 1, "type": "request", "command": "initialize",
                       "arguments": {"clientID": "test", "adapterID": "debugpy",
                                   "linesStartAt1": True, "columnsStartAt1": True}})

    # Read until we get initialize response
    for _ in range(5):
        msg = read_message(proc)
        if msg and msg.get('type') == 'response' and msg.get('command') == 'initialize':
            print("    ✅ Got initialize response\n", file=sys.stderr)
            break

    # Step 2: Wait for initialized EVENT
    print("[2] Waiting for 'initialized' event", file=sys.stderr)
    for _ in range(5):
        msg = read_message(proc)
        if msg and msg.get('type') == 'event' and msg.get('event') == 'initialized':
            print("    ✅ Got 'initialized' event\n", file=sys.stderr)
            break

    # Step 3: configurationDone (BEFORE launch)
    print("[3] configurationDone", file=sys.stderr)
    send_message(proc, {"seq": 2, "type": "request", "command": "configurationDone", "arguments": {}})

    for _ in range(5):
        msg = read_message(proc)
        if msg and msg.get('type') == 'response' and msg.get('command') == 'configurationDone':
            print("    ✅ Got configurationDone response\n", file=sys.stderr)
            break

    # Step 4: launch
    print("[4] launch", file=sys.stderr)
    send_message(proc, {"seq": 3, "type": "request", "command": "launch",
                       "arguments": {"request": "launch", "type": "python",
                                   "program": fizzbuzz, "console": "internalConsole",
                                   "stopOnEntry": False}})

    for _ in range(10):
        msg = read_message(proc, timeout_secs=3)
        if not msg:
            print("    ❌ TIMEOUT on launch\n", file=sys.stderr)
            break
        if msg.get('type') == 'response' and msg.get('command') == 'launch':
            success = msg.get('success', False)
            if success:
                print("    ✅ Got launch response - SUCCESS!\n", file=sys.stderr)
            else:
                print(f"    ❌ Launch failed: {msg.get('message')}\n", file=sys.stderr)
            break

    print("=== Test Complete ===\n", file=sys.stderr)
finally:
    proc.terminate()
    proc.wait()
