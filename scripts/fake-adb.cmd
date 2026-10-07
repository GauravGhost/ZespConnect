@echo off
REM Fake adb for development and tests - no phone required.
REM Point Zesp Connect's "adb" tool path at this file (Windows runs .cmd via cmd.exe).
setlocal
set "SUB=%~1"
set "CMD=%~2"

REM Most commands are run scoped as `-s <serial> <subcommand> ...`; skip the scope.
if "%SUB%"=="-s" (
  set "SUB=%~3"
  set "CMD=%~4"
)

if "%SUB%"=="devices" goto devices
if "%SUB%"=="version" goto version
if "%SUB%"=="start-server" exit /b 0
if "%SUB%"=="install" goto ok
if "%SUB%"=="uninstall" goto ok
if "%SUB%"=="shell" goto shell
if "%SUB%"=="track-devices" goto track
exit /b 0

:ok
echo Success
exit /b 0

:devices
echo List of devices attached
echo 0123456789ABCDEF        device product:husky model:Pixel_8 device:husky transport_id:1
exit /b 0

:version
echo Android Debug Bridge version 1.0.41
exit /b 0

:shell
if not "%CMD%"=="" (
  if not "%CMD:pm list packages -f=%"=="%CMD%" (
    echo package:/data/app/~~1a2b==/com.example.notes-xyz==/base.apk=com.example.notes
    echo package:/data/app/~~3c4d==/com.google.android.apps.maps-xyz==/base.apk=com.google.android.apps.maps
    echo package:/system/priv-app/Settings/Settings.apk=com.android.settings
    echo package:/system_ext/priv-app/SystemUI/SystemUI.apk=com.android.systemui
    exit /b 0
  )
  if not "%CMD:pm list packages=%"=="%CMD%" (
    echo package:com.example.notes
    echo package:com.google.android.apps.maps
    echo package:com.android.settings
    exit /b 0
  )
  if not "%CMD:pm path=%"=="%CMD%" (
    echo package:/data/app/~~1a2b==/com.example.notes-xyz==/base.apk
    exit /b 0
  )
  if not "%CMD:pm clear=%"=="%CMD%" (
    echo Success
    exit /b 0
  )
)
echo MODEL:Pixel 8
echo VER:15
echo SDK:35
echo   level: 82
echo   powered: true
echo WIFI:1
echo BT:1
echo DARK:Night mode: yes
echo AIR:0
echo DATA:1
echo LOC:3
echo DND:1
echo ROT:1
echo BRI:1024
echo VOL:volume is 80 in range [0..150]
exit /b 0

:track
REM One empty frame (length 0000), then stay alive so the tracker keeps the stream open.
echo 0000
:loop
timeout /t 3 /nobreak >nul
goto loop
