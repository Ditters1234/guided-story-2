@echo off
setlocal
cd /d "%~dp0"
if errorlevel 1 exit /b 1

where npm.cmd >nul 2>&1
if errorlevel 1 (
    echo Node.js 24 or newer is required. Install it, then try again.
    set "rebuildExit=1"
    goto finished
)

echo Rebuilding Guided Story 2...
echo The finished executable will be placed in a new folder under build.
echo.
call npm.cmd run rebuild
set "rebuildExit=%ERRORLEVEL%"
if not "%rebuildExit%"=="0" (
    echo.
    echo Rebuild failed. See the error above.
)

:finished
echo.
if /I not "%~1"=="--no-pause" pause
exit /b %rebuildExit%
