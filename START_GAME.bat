@echo off
setlocal
cd /d "%~dp0"
where docker >nul 2>nul
if errorlevel 1 (
  echo Docker was not found. Install Docker Desktop, start it, and run this file again.
  pause
  exit /b 1
)
echo Building and starting The Republic...
docker compose up --build -d
if errorlevel 1 (
  echo The container could not be started. Check that Docker Desktop is running.
  pause
  exit /b 1
)
echo Waiting for the server...
for /L %%i in (1,1,30) do (
  curl -fsS http://localhost:8080/api/v1/health >nul 2>nul && goto ready
  timeout /t 1 /nobreak >nul
)
echo The server did not become ready. Run VIEW_LOGS.bat for details.
pause
exit /b 1
:ready
start "" http://localhost:8080
echo Game opened at http://localhost:8080
exit /b 0
