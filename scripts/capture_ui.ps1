<#
.SYNOPSIS
    Headless screenshot harness for IRON FAUNA.

.DESCRIPTION
    Thin wrapper around the shared macroquad-toolkit capture script. Builds the
    debug exe and drives it through the env-var capture hook
    (IRON_FAUNA_CAPTURE_*) provided by macroquad_toolkit::capture in
    src/main.rs.

.EXAMPLE
    ./scripts/capture_ui.ps1
    ./scripts/capture_ui.ps1 -Frames 60 -SkipBuild
#>
param(
    [string[]]$Scenes = @("gameplay"),
    [int]$Frames = 150,
    [string]$OutputDir = "docs\verification",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$gameDir = Split-Path -Parent $PSScriptRoot
$shared = Join-Path (Split-Path -Parent $gameDir) "macroquad-toolkit\scripts\capture_ui.ps1"

& $shared -GameDir $gameDir -Prefix "IRON_FAUNA" -Scenes $Scenes -Frames $Frames -OutputDir $OutputDir -SkipBuild:$SkipBuild
