@echo off
setlocal enabledelayedexpansion
set APP_NAME=SysPort
set BINARY_NAME=sysport.exe
set DIST_DIR=dist
set PLUGINS_DIR=dist\plugins
set ICON_ICO=..\sysport.ico

if exist %DIST_DIR% rmdir /s /q %DIST_DIR%
mkdir %DIST_DIR%
mkdir %PLUGINS_DIR%

REM Clean previous build artifacts to avoid stale dependencies
cargo clean --manifest-path ..\sysport\Cargo.toml

REM Build main binary with vendored Lua (no system Lua or pkg-config needed)
cargo build --manifest-path ..\sysport\Cargo.toml --release --target x86_64-pc-windows-msvc
if errorlevel 1 exit /b 1
copy ..\target\x86_64-pc-windows-msvc\release\%BINARY_NAME% %DIST_DIR%\
copy %ICON_ICO% %DIST_DIR%\
echo [OK] Built %BINARY_NAME% for Windows.

REM Set icon using rcedit if available
where rcedit >nul 2>nul
if %errorlevel%==0 (
  rcedit %DIST_DIR%\%BINARY_NAME% --set-icon %DIST_DIR%\%ICON_ICO%
  echo [OK] Embedded icon using rcedit.
) else (
  echo [WARN] rcedit not found, skipping icon embedding.
)

REM Build plugins as .dll
for %%d in (..\..\plugins\*) do (
  if exist "%%d\Cargo.toml" (
    pushd %%d
    cargo build --release
    set name=%%~nd
    copy target\release\%name%.dll ..\..\sysport\dist\plugins\
    echo [OK] Built plugin: %name%.dll
    popd
  )
)

REM Create .zip package
powershell Compress-Archive -Path %DIST_DIR%\%BINARY_NAME%,%PLUGINS_DIR%\*,%DIST_DIR%\%ICON_ICO% -DestinationPath %DIST_DIR%\%APP_NAME%.zip
if errorlevel 1 exit /b 1

echo [SUCCESS] Windows build complete. Artifacts in %DIST_DIR%\ 