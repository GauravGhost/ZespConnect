#!/usr/bin/env bash
# Fake adb for development and tests - no phone required.
# Point Zesp Connect's "adb" tool path at this file.
set -eu

args=("$@")

# Most commands are run scoped as `-s <serial> <subcommand> ...`; skip the scope.
i=0
if [ "${args[0]:-}" = "-s" ]; then
  i=2
fi
sub="${args[$i]:-}"
cmd="${args[$((i + 1))]:-}"

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
  install | uninstall)
    printf 'Success\n'
    ;;
  shell)
    case "$cmd" in
      *"ip -f inet addr show wlan0"*)
        printf '3: wlan0: mtu 1500 qdisc mq state UP group default\n'
        printf '    inet 192.168.1.50/24 brd 192.168.1.255 scope global wlan0\n'
        ;;
      *"getprop dhcp.wlan0.ipaddress"*)
        printf '192.168.1.50\n'
        ;;
      *"pm list packages -f"*)
        printf 'package:/data/app/~~1a2b==/com.example.notes-xyz==/base.apk=com.example.notes\n'
        printf 'package:/data/app/~~3c4d==/com.google.android.apps.maps-xyz==/base.apk=com.google.android.apps.maps\n'
        printf 'package:/system/priv-app/Settings/Settings.apk=com.android.settings\n'
        printf 'package:/system_ext/priv-app/SystemUI/SystemUI.apk=com.android.systemui\n'
        ;;
      *"pm list packages"*)
        printf 'package:com.example.notes\n'
        printf 'package:com.google.android.apps.maps\n'
        printf 'package:com.android.settings\n'
        ;;
      *"pm path"*)
        printf 'package:/data/app/~~1a2b==/com.example.notes-xyz==/base.apk\n'
        ;;
      *"pm clear"* | *"pm uninstall"*)
        printf 'Success\n'
        ;;
      *"monkey"*)
        printf 'Events injected: 1\n'
        ;;
      *)
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
    esac
    ;;
  track-devices)
    # One empty frame (length 0000), then stay alive to keep the stream open.
    printf '0000'
    while true; do sleep 3; done
    ;;
  mdns)
    printf 'List of discovered mdns services\n'
    printf 'adb-0123456789ABCDEF-xyz123 _adb-tls-connect._tcp 192.168.1.42:5555\n'
    printf 'studio-abcdEFghij _adb-tls-pairing._tcp 192.168.1.43:37401\n'
    printf 'adb-0123456789ABCDEF _adb._tcp 192.168.1.42:5555\n'
    ;;
  pair)
    printf 'Successfully paired to %s [guid=adb-0123456789ABCDEF-xyz123]\n' "$cmd"
    ;;
  connect)
    printf 'connected to %s\n' "$cmd"
    ;;
  disconnect)
    printf 'disconnected %s\n' "$cmd"
    ;;
  tcpip)
    printf 'restarting in TCP mode port: %s\n' "$cmd"
    ;;
esac
