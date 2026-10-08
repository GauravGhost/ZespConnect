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
  push)
    printf '1 file pushed. 0 files skipped.\n'
    ;;
  pull)
    printf '1 file pulled. 0 files skipped.\n'
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
      *"ls -la"*)
        printf 'total 48\n'
        printf 'drwxrwx--x   4 root   sdcard_rw    4096 2024-05-01 09:12 Android\n'
        printf 'drwxrwx--x   3 root   sdcard_rw    4096 2024-04-20 18:03 DCIM\n'
        printf 'drwxrwx--x   2 root   sdcard_rw    4096 2024-05-03 11:45 Download\n'
        printf -- '-rw-rw----   1 root   sdcard_rw 1048576 2024-05-02 14:22 My Notes.txt\n'
        printf -- '-rw-rw----   1 root   sdcard_rw     512 2024-05-02 14:23 photo.png\n'
        ;;
      *"mkdir -p"* | *"rm -rf"*)
        ;;
      *"cat "*"clipboard.txt"*)
        printf 'Hello from the device clipboard\n'
        ;;
      *"am broadcast"*)
        printf 'Broadcast completed: result=0\n'
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
  logcat)
    printf '05-01 09:12:00.000  1234  1234 I ActivityManager: Start proc com.example.notes\n'
    printf '05-01 09:12:01.100  1234  1250 D WindowManager: relayout Window{abc}\n'
    printf '05-01 09:12:02.200  1234  1234 W System.err: something odd\n'
    while true; do
      sleep 2
      printf '05-01 09:12:03.300  1234  1234 I ActivityManager: still running\n'
    done
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
