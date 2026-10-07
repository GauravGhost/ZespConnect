@echo off
REM Fake adb for development and tests - no phone required.
REM Point Zesp Connect's "adb" tool path at this file (Windows runs .cmd via cmd.exe).
setlocal
set "SUB=%~1"

if "%SUB%"=="devices" goto devices
if "%SUB%"=="version" goto version
if "%SUB%"=="start-server" exit /b 0
if "%SUB%"=="shell" goto shell
if "%SUB%"=="track-devices" goto track
exit /b 0

:devices
echo List of devices attached
echo 0123456789ABCDEF        device product:husky model:Pixel_8 device:husky transport_id:1
exit /b 0

:version
echo Android Debug Bridge version 1.0.41
exit /b 0

:shell
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
