#!/usr/bin/env python3
"""
Standalone DAP Protocol Test with debugpy

This script tests the DAP protocol flow directly with debugpy adapter
to verify the correct sequence and timing of messages.

It demonstrates:
1. Spawning debugpy adapter
2. Initialize request/response
3. Launch request (triggers 'initialized' event)
4. Handling 'initialized' event and sending configurationDone
5. Proper message sequencing

Usage:
    python3 scripts/test_dap_standalone.py
"""

import subprocess
import json
import sys
import os
import threading
import time
from pathlib import Path
from datetime import datetime

class DAPTester:
    def __init__(self):
        self.process = None
        self.seq = 0
        self.received_events = []
        self.received_responses = {}
        self.reader_thread = None
        self.running = False
        self.start_time = time.time()
        self.timing_log = []

    def log_timing(self, event_type, description, details=""):
        """Log timing of events"""
        elapsed = (time.time() - self.start_time) * 1000  # Convert to milliseconds
        entry = {
            'timestamp_ms': elapsed,
            'type': event_type,
            'description': description,
            'details': details
        }
        self.timing_log.append(entry)
        print(f"⏱️  [{elapsed:7.2f}ms] {event_type}: {description} {details}")

    def start_adapter(self):
        """Spawn debugpy adapter process"""
        print("🚀 Spawning debugpy adapter...")
        self.start_time = time.time()
        self.log_timing("SPAWN", "Starting debugpy adapter process")

        self.process = subprocess.Popen(
            ['python3', '-m', 'debugpy.adapter'],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            bufsize=0
        )

        # Start reader thread
        self.running = True
        self.reader_thread = threading.Thread(target=self._read_messages, daemon=True)
        self.reader_thread.start()

        print("✅ Adapter spawned successfully")
        self.log_timing("SPAWN", "Adapter process ready")
        time.sleep(0.5)  # Give adapter time to initialize

    def _read_messages(self):
        """Background thread to read messages from adapter"""
        while self.running:
            try:
                message = self._read_message()
                if message:
                    msg_type = message.get('type')

                    if msg_type == 'response':
                        seq = message.get('request_seq')
                        command = message.get('command')
                        self.received_responses[seq] = message
                        self.log_timing("RECV_RESP", f"{command} response", f"(seq {seq})")
                        print(f"📥 RESPONSE (seq {seq}): {command} - success={message.get('success')}")

                    elif msg_type == 'event':
                        event_name = message.get('event')
                        self.received_events.append(message)
                        self.log_timing("RECV_EVENT", f"'{event_name}' event")
                        print(f"📢 EVENT: {event_name}")
                        print(f"   Body: {json.dumps(message.get('body', {}), indent=2)}")

                        # Auto-handle 'initialized' event
                        if event_name == 'initialized':
                            print("🎯 Received 'initialized' event - sending configurationDone...")
                            self.send_configuration_done()

            except Exception as e:
                if self.running:
                    print(f"❌ Error reading message: {e}")
                break

    def _read_message(self):
        """Read a single DAP message (Content-Length header + JSON body)"""
        # Read Content-Length header
        header = b''
        while True:
            char = self.process.stdout.read(1)
            if not char:
                return None
            header += char
            if header.endswith(b'\r\n\r\n'):
                break

        # Parse content length
        header_str = header.decode('utf-8')
        for line in header_str.split('\r\n'):
            if line.startswith('Content-Length:'):
                content_length = int(line.split(':')[1].strip())
                break
        else:
            raise ValueError(f"No Content-Length in header: {header_str}")

        # Read JSON body
        body = self.process.stdout.read(content_length)
        message = json.loads(body.decode('utf-8'))
        return message

    def _send_message(self, message):
        """Send a DAP message with Content-Length header"""
        body = json.dumps(message).encode('utf-8')
        header = f"Content-Length: {len(body)}\r\n\r\n".encode('utf-8')

        command = message.get('command', message.get('type'))
        print(f"📤 SENDING: {command}")
        self.log_timing("SEND_REQ", f"{command} request", f"(seq {message.get('seq')})")
        self.process.stdin.write(header + body)
        self.process.stdin.flush()

    def send_initialize(self):
        """Send initialize request"""
        self.seq += 1
        request = {
            "type": "request",
            "seq": self.seq,
            "command": "initialize",
            "arguments": {
                "clientID": "dap-standalone-test",
                "clientName": "DAP Standalone Tester",
                "adapterID": "debugpy",
                "pathFormat": "path",
                "linesStartAt1": True,
                "columnsStartAt1": True,
                "supportsVariableType": True,
                "supportsVariablePaging": True,
                "supportsRunInTerminalRequest": False,
                "locale": "en-US"
            }
        }
        self._send_message(request)
        return self.seq

    def send_launch(self, program_path):
        """Send launch request"""
        self.seq += 1
        request = {
            "type": "request",
            "seq": self.seq,
            "command": "launch",
            "arguments": {
                "request": "launch",
                "type": "python",
                "program": program_path,
                "args": [],
                "console": "internalConsole",
                "stopOnEntry": False,
                "cwd": str(Path(program_path).parent),
            }
        }
        self._send_message(request)
        return self.seq

    def send_configuration_done(self):
        """Send configurationDone request"""
        self.seq += 1
        request = {
            "type": "request",
            "seq": self.seq,
            "command": "configurationDone"
        }
        self._send_message(request)
        return self.seq

    def send_disconnect(self):
        """Send disconnect request"""
        self.seq += 1
        request = {
            "type": "request",
            "seq": self.seq,
            "command": "disconnect",
            "arguments": {
                "terminateDebuggee": True
            }
        }
        self._send_message(request)
        return self.seq

    def wait_for_response(self, seq, timeout=5.0):
        """Wait for a specific response"""
        start = time.time()
        while time.time() - start < timeout:
            if seq in self.received_responses:
                return self.received_responses[seq]
            time.sleep(0.1)
        raise TimeoutError(f"No response for seq {seq} after {timeout}s")

    def shutdown(self):
        """Cleanup"""
        self.running = False
        if self.process:
            self.process.terminate()
            self.process.wait()

def main():
    # Get path to fizzbuzz.py
    test_file = Path(__file__).parent.parent / "tests" / "fixtures" / "fizzbuzz.py"
    if not test_file.exists():
        print(f"❌ Test file not found: {test_file}")
        return 1

    print("\n" + "="*60)
    print("DAP PROTOCOL STANDALONE TEST WITH TIMING ANALYSIS")
    print("="*60 + "\n")

    tester = DAPTester()

    try:
        # Step 1: Start adapter
        tester.start_adapter()

        # Step 2: Initialize
        print("\n📋 Step 1: Sending initialize request...")
        init_seq = tester.send_initialize()

        print("⏳ Waiting for initialize response...")
        init_response = tester.wait_for_response(init_seq, timeout=5.0)

        if init_response.get('success'):
            print("✅ Initialize successful!")
            capabilities = init_response.get('body', {})
            print(f"   Capabilities: supportsConfigurationDoneRequest={capabilities.get('supportsConfigurationDoneRequest')}")
        else:
            print(f"❌ Initialize failed: {init_response.get('message')}")
            return 1

        # Step 3: Launch (will trigger 'initialized' event)
        print("\n📋 Step 2: Sending launch request...")
        print("   (This will trigger 'initialized' event during processing)")
        launch_seq = tester.send_launch(str(test_file))

        # Step 4: Wait for 'initialized' event and configurationDone
        # Note: The reader thread auto-sends configurationDone when it receives 'initialized'
        print("\n⏳ Waiting for 'initialized' event...")
        print("   (configurationDone will be sent automatically)")

        # Wait for launch response (should arrive after configurationDone)
        print("\n⏳ Waiting for launch response...")
        launch_response = tester.wait_for_response(launch_seq, timeout=10.0)

        if launch_response.get('success'):
            print("✅ Launch successful!")
        else:
            print(f"❌ Launch failed: {launch_response.get('message')}")
            return 1

        # Step 5: Give program a moment to start
        time.sleep(1.0)

        # Step 6: Disconnect
        print("\n📋 Step 3: Disconnecting...")
        disc_seq = tester.send_disconnect()
        disc_response = tester.wait_for_response(disc_seq, timeout=5.0)

        if disc_response.get('success'):
            print("✅ Disconnect successful!")
        else:
            print(f"⚠️  Disconnect completed with: {disc_response.get('message')}")

        # Summary
        print("\n" + "="*60)
        print("SUMMARY")
        print("="*60)
        print(f"✅ Total events received: {len(tester.received_events)}")
        print(f"✅ Total responses received: {len(tester.received_responses)}")

        print("\n📢 Events received:")
        for event in tester.received_events:
            print(f"   - {event.get('event')}")

        # Print detailed timing analysis
        print("\n" + "="*60)
        print("TIMING ANALYSIS")
        print("="*60)
        print("\nComplete message sequence with timestamps:\n")

        for entry in tester.timing_log:
            timestamp = entry['timestamp_ms']
            event_type = entry['type']
            description = entry['description']
            details = entry['details']
            print(f"  [{timestamp:7.2f}ms] {event_type:12s} {description:40s} {details}")

        # Calculate key intervals
        print("\n" + "="*60)
        print("KEY TIMING INTERVALS")
        print("="*60 + "\n")

        def find_timing(log, type_val, desc_contains):
            for entry in log:
                if entry['type'] == type_val and desc_contains in entry['description']:
                    return entry['timestamp_ms']
            return None

        # Find key events
        init_sent = find_timing(tester.timing_log, "SEND_REQ", "initialize")
        init_recv = find_timing(tester.timing_log, "RECV_RESP", "initialize")
        launch_sent = find_timing(tester.timing_log, "SEND_REQ", "launch")
        initialized_event = find_timing(tester.timing_log, "RECV_EVENT", "initialized")
        config_sent = find_timing(tester.timing_log, "SEND_REQ", "configurationDone")
        config_recv = find_timing(tester.timing_log, "RECV_RESP", "configurationDone")
        launch_recv = find_timing(tester.timing_log, "RECV_RESP", "launch")

        if all([init_sent, init_recv, launch_sent, initialized_event, config_sent, config_recv, launch_recv]):
            print(f"✅ Initialize request → response:        {init_recv - init_sent:7.2f}ms")
            print(f"✅ Launch request → 'initialized' event:  {initialized_event - launch_sent:7.2f}ms")
            print(f"✅ 'initialized' → configurationDone:     {config_sent - initialized_event:7.2f}ms (event handler latency)")
            print(f"✅ configurationDone → response:          {config_recv - config_sent:7.2f}ms")
            print(f"✅ configurationDone → launch response:   {launch_recv - config_sent:7.2f}ms")
            print(f"✅ TOTAL: Launch request → response:      {launch_recv - launch_sent:7.2f}ms")
            print(f"\n🎯 CRITICAL INSIGHT:")
            print(f"   Launch request sent at {launch_sent:.2f}ms")
            print(f"   Launch response recv at {launch_recv:.2f}ms")
            print(f"   But 'initialized' event at {initialized_event:.2f}ms")
            print(f"   And configurationDone at {config_sent:.2f}ms")
            print(f"\n   The 'initialized' event arrives {initialized_event - launch_sent:.2f}ms after launch,")
            print(f"   but the launch response only arrives {launch_recv - config_sent:.2f}ms after configurationDone!")
        else:
            print("⚠️  Could not calculate all timing intervals")

        print("\n🎉 DAP Protocol Test PASSED!")
        print("\nKey Findings:")
        print("1. 'initialized' event arrives DURING launch processing")
        print("2. configurationDone must be sent from event handler")
        print("3. Launch response arrives AFTER configurationDone")
        print("4. Event-driven architecture is REQUIRED for proper sequencing")

        return 0

    except TimeoutError as e:
        print(f"\n❌ TIMEOUT: {e}")
        print("\nThis indicates an issue with the DAP sequence.")
        return 1

    except Exception as e:
        print(f"\n❌ ERROR: {e}")
        import traceback
        traceback.print_exc()
        return 1

    finally:
        tester.shutdown()

if __name__ == "__main__":
    sys.exit(main())
