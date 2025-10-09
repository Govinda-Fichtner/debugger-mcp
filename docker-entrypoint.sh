#!/bin/bash
set -e

# If running as root, fix permissions and switch to testuser
if [ "$(id -u)" = "0" ]; then
    # Fix ownership of workspace if mounted (ignore errors if already correct)
    if [ -d /workspace ]; then
        chown -R testuser:testuser /workspace 2>/dev/null || true
    fi

    # Fix ownership of cargo directories
    if [ -d /usr/local/cargo ]; then
        chown -R testuser:testuser /usr/local/cargo 2>/dev/null || true
    fi

    # Switch to testuser and run the command
    # Pass all arguments properly using su with shell wrapper
    exec su testuser -c 'cd /workspace && exec "$@"' sh "$@"
else
    # Already running as testuser, just execute the command
    cd /workspace
    exec "$@"
fi
