[CmdletBinding()]
param (
    [string]$OutputDirectory = "dist\linux"
)

$ErrorActionPreference = "Stop"

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$builderDirectory = Join-Path $projectRoot "docker"
$dockerfile = Join-Path $builderDirectory "linux-builder.Dockerfile"

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    throw "Docker CLI was not found. Install Docker Desktop and make sure docker is on PATH."
}

docker info --format "{{.ServerVersion}}"
if ($LASTEXITCODE -ne 0) {
    throw "Docker Engine is unavailable. Start Docker Desktop and try again."
}

if (-not (Test-Path -LiteralPath $dockerfile -PathType Leaf)) {
    throw "Docker builder file not found: $dockerfile"
}

if ([System.IO.Path]::IsPathRooted($OutputDirectory)) {
    $outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)
} else {
    $outputPath = [System.IO.Path]::GetFullPath((Join-Path $projectRoot $OutputDirectory))
}

if ($projectRoot.Contains(",") -or $outputPath.Contains(",")) {
    throw "Docker bind-mount paths cannot contain commas."
}

$null = New-Item -ItemType Directory -Force -Path $outputPath
$image = "rg-studio-linux-builder:1.98.1"

docker build --file $dockerfile --tag $image $builderDirectory
if ($LASTEXITCODE -ne 0) {
    throw "Docker image build failed with exit code $LASTEXITCODE."
}

$dockerArguments = @(
    "run",
    "--rm",
    "--mount",
    "type=bind,source=$projectRoot,target=/workspace",
    "--mount",
    "type=bind,source=$outputPath,target=/output",
    "--mount",
    "type=volume,source=rg-studio-cargo-registry,target=/usr/local/cargo/registry",
    "--mount",
    "type=volume,source=rg-studio-cargo-git,target=/usr/local/cargo/git",
    "--mount",
    "type=volume,source=rg-studio-linux-target,target=/cargo-target",
    "--workdir",
    "/workspace",
    "--env",
    "CARGO_TARGET_DIR=/cargo-target",
    $image,
    "sh",
    "-ec",
    "cargo build --release --locked && install -D -m 0755 /cargo-target/release/rg-studio /output/rg-studio && tar -C /cargo-target/release -czf /output/rg-studio-linux.tar.gz rg-studio"
)

docker @dockerArguments
if ($LASTEXITCODE -ne 0) {
    throw "Linux release build failed with exit code $LASTEXITCODE."
}

$binaryPath = Join-Path $outputPath "rg-studio"
if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
    throw "Linux release binary was not created: $binaryPath"
}

$archivePath = Join-Path $outputPath "rg-studio-linux.tar.gz"
if (-not (Test-Path -LiteralPath $archivePath -PathType Leaf)) {
    throw "Linux release archive was not created: $archivePath"
}

Write-Host "Linux release binary created: $binaryPath"
Write-Host "Linux release archive created: $archivePath"
