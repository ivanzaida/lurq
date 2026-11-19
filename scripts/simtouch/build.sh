#!/bin/sh
# Builds simtouch next to its source, ignored by git: real touches on the iOS simulator shown in Xcode's DeviceHub.
# The process that runs it (the terminal) needs Accessibility in System Settings › Privacy & Security.
set -eu
cd "$(dirname "$0")"
swiftc -O main.swift -o simtouch
