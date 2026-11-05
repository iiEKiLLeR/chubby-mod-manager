# Chubby Mod Manager release script
# 1. Bump `version` in Cargo.toml and update CURRENT_RELEASE_NOTES in src/app.rs
# 2. Run:  powershell -ExecutionPolicy Bypass -File .\release.ps1
# Builds the app + installer, writes update.json, publishes a GitHub Release, and pushes the source.

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$gh = "C:\Program Files\GitHub CLI\gh.exe"
$releasesRepo = "iiEKiLLeR/chubby-mod-manager-releases"

$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
Write-Host "Releasing Chubby Mod Manager v$version" -ForegroundColor Cyan

cargo build --release --bin chubby_mod_manager
if ($LASTEXITCODE -ne 0) { throw "App build failed" }
Copy-Item target\release\chubby_mod_manager.exe assets\chubby_mod_manager_payload.exe -Force
cargo build --release --bin Chubby_Mod_Manager_Setup
if ($LASTEXITCODE -ne 0) { throw "Installer build failed" }

New-Item -ItemType Directory -Force release | Out-Null
Copy-Item target\release\Chubby_Mod_Manager_Setup.exe release\Chubby_Mod_Manager_Setup.exe -Force
Start-Process -FilePath target\release\chubby_mod_manager.exe -ArgumentList '--write-update-json', "`"$PWD\release\update.json`"" -Wait

$notes = (Get-Content release\update.json -Raw | ConvertFrom-Json).notes | ForEach-Object { "- $_" }
$body = "## What's new`n" + ($notes -join "`n") + "`n`n**Install:** download ``Chubby_Mod_Manager_Setup.exe`` below and run it. Already installed? The app shows an update popup automatically."
& $gh release create "v$version" release\Chubby_Mod_Manager_Setup.exe release\update.json --repo $releasesRepo --title "Chubby Mod Manager v$version" --notes $body --latest
if ($LASTEXITCODE -ne 0) { throw "GitHub release failed" }

git add -A
git commit -m "Release v$version"
git tag "v$version"
git push origin HEAD --tags

Write-Host "Done! Friends will get the v$version update popup next time they open the app." -ForegroundColor Green
