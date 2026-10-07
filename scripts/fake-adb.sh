#!/usr/bin/env bash
# Fake adb for development and tests - no phone required.
# Point Zesp Connect's "adb" tool path at this file.
set -eu

sub="${1:-}"

case "$sub" in
  devices)
    printf 'List of devices attached\n'
    printf '0123456789ABCDEF        device product:husky model:Pixel_8 device:husky transport_id:1\n'
    ;;
  version)
    printf 'Android Debug Bridge version 1.0.41\n'
    ;;
  start-server)
    ;;
  shell)
    printf 'MODEL:Pixel 8\n'
    printf 'VER:15\n'
    printf 'SDK:35\n'
    printf '  level: 82\n'
    printf '  powered: true\n'
    printf 'WIFI:1\n'
    printf 'BT:1\n'
    printf 'DARK:Night mode: yes\n'
    printf 'AIR:0\n'
    printf 'DATA:1\n'
    printf 'LOC:3\n'
    printf 'DND:1\n'
    printf 'ROT:1\n'
    printf 'BRI:1024\n'
    printf 'VOL:volume is 80 in range [0..150]\n'
    ;;
  track-devices)
    # One empty frame (length 0000), then stay alive to keep the stream open.
    printf '0000'
    while true; do sleep 3; done
    ;;
esac
