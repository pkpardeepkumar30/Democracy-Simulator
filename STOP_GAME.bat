@echo off
cd /d "%~dp0"
docker compose down
echo The game has stopped. Saved sessions remain in the Docker volume civic_game_data.
pause
