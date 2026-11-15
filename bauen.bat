@echo off
rem Baut die App und legt sie als PuzzleCellar.exe in diesen Ordner.
setlocal
cd /d "%~dp0"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
cargo build --release || goto :fehler
copy /y "target\release\puzzlecellar.exe" "PuzzleCellar.exe" >nul || goto :fehler
echo.
echo Fertig: PuzzleCellar.exe
exit /b 0
:fehler
echo.
echo Bauen fehlgeschlagen.
pause
exit /b 1
