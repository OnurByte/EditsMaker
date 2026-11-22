# EditsMaker installer for Windows. In PowerShell:
#
#   irm https://raw.githubusercontent.com/OnurByte/EditsMaker/main/install.ps1 | iex
#
# Installs edits.exe and the desktop app (edits-app.exe, in the Start menu) for the current user into %LOCALAPPDATA%\Programs\EditsMaker after checking the release
# archive's SHA-256, and adds that folder to the user PATH. EDITS_VERSION=v0.2.0 picks a release.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$Repo = 'OnurByte/EditsMaker'
$Version = if ($env:EDITS_VERSION) { $env:EDITS_VERSION } else { 'latest' }
$InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\EditsMaker'
$Asset = 'edits-x86_64-pc-windows-msvc.zip'
$Base = if ($Version -eq 'latest') { "https://github.com/$Repo/releases/latest/download" } else { "https://github.com/$Repo/releases/download/$Version" }

if (-not [Environment]::Is64BitOperatingSystem) { throw 'EditsMaker needs 64-bit Windows.' }

$Temp = Join-Path ([IO.Path]::GetTempPath()) ("edits-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $Temp | Out-Null
try {
    Write-Host 'Downloading EditsMaker...'
    $Zip = Join-Path $Temp $Asset
    Invoke-WebRequest "$Base/$Asset" -OutFile $Zip -UseBasicParsing
    Invoke-WebRequest "$Base/SHA256SUMS" -OutFile (Join-Path $Temp 'SHA256SUMS') -UseBasicParsing
    $Line = Get-Content (Join-Path $Temp 'SHA256SUMS') | Where-Object { $_ -match [regex]::Escape($Asset) } | Select-Object -First 1
    if (-not $Line) { throw "No checksum for $Asset in this release." }
    $Expected = ($Line -split '\s+')[0].ToLower()
    $Actual = (Get-FileHash $Zip -Algorithm SHA256).Hash.ToLower()
    if ($Expected -ne $Actual) { throw 'Checksum mismatch, refusing to install.' }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Expand-Archive $Zip -DestinationPath $InstallDir -Force

    $Path = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not ($Path -split ';' | Where-Object { $_ -eq $InstallDir })) {
        [Environment]::SetEnvironmentVariable('Path', ($Path.TrimEnd(';') + ';' + $InstallDir).TrimStart(';'), 'User')
        $env:Path += ";$InstallDir"
    }
    Write-Host "Installed: $(& (Join-Path $InstallDir 'edits.exe') --version)"
    # the desktop app in the Start menu
    $App = Join-Path $InstallDir 'edits-app.exe'
    if (Test-Path $App) {
        $Link = (New-Object -ComObject WScript.Shell).CreateShortcut((Join-Path ([Environment]::GetFolderPath('Programs')) 'EditsMaker.lnk'))
        $Link.TargetPath = $App
        $Link.WorkingDirectory = $InstallDir
        $Link.Save()
        Write-Host 'The app: EditsMaker in the Start menu, or edits app'
    }
    # ffmpeg and yt-dlp, if this machine lacks them: static builds into ~\.config\edits\bin, checksum checked
    & (Join-Path $InstallDir 'edits.exe') setup
    if ($LASTEXITCODE -ne 0) { Write-Host 'ffmpeg/yt-dlp download failed; edits tries again on first use.' }
    # the MCP server and skills in every agent on this machine (Claude Code, ChatGPT, OpenCode, Cursor, ...)
    & (Join-Path $InstallDir 'edits.exe') mcp install
    & (Join-Path $InstallDir 'edits.exe') skills install
} finally {
    Remove-Item -Recurse -Force $Temp -ErrorAction SilentlyContinue
}
