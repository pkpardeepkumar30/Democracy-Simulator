@echo off
cd /d "%~dp0"
echo This deletes every saved game session.
choice /M "Continue"
if errorlevel 2 exit /b 0
docker compose down -v
echo Saved data deleted.
pause
