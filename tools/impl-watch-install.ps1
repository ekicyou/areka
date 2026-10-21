<#
.SYNOPSIS
  机の貸し出しのアプリ areka-impl-watch を、このスクリプトの在る作業木のソースから release でビルドし、
  環境変数 AREKA_IMPL_WATCH_HOME が指す置き場所へ置く（走っていれば止めずに入れ替える）。

.DESCRIPTION
  使い方（引数なし・どのフォルダから呼んでも同じに動く・今のフォルダを変えない）:
    pwsh -NoProfile -File <リポジトリの絶対パス>\tools\impl-watch-install.ps1

  順:
    1 環境変数を読む。空・空白だけ・絶対パスでない（/c/...・\foo・C:foo を含む）なら何もせず終わる。
      値は画面へ出さず、設定も変更もしない。
    2 cargo build --release -q -p areka-impl-watch でビルドし、実行ファイルの道筋は cargo の JSON の報せ
      （executable）から UTF-8 のバイトとして読む（Invoke-Utf8Child）。失敗なら置き場所に触らない。
    3 置き場所が無ければ作り、新しいものを areka-impl-watch.new.exe として写す。
    4 今の areka-impl-watch.exe を areka-impl-watch.old-<UTC>.exe（20261016T020500Z の形）へ改名し、
      .new.exe を areka-impl-watch.exe へ改名する。2 つ目の改名に失敗したら古いものを元へ戻し、.new.exe を消す。
    5 areka-impl-watch.old*.exe（前の形の .old.exe も）を消してみる。走っていて消せなければ残す。
    6 置いたものの --help を呼んで確かめる。
    7 置いたものの status を呼ぶ。状態ファイルの版が合わない（標準エラーが state file version mismatch で
      始まる）ときだけ、置いたもので clear を 1 回呼ぶ。版が合っていれば状態には触らない。

  自分で触るのは置き場所の areka-impl-watch.exe・areka-impl-watch.new.exe・areka-impl-watch.old*.exe だけ。
  state.json・ロック・居る印・ログ・status.md・退避のファイルは手 7 の clear（exe が行う）でだけ変わる。

  出る行（ASCII だけ・端末の文字コードは読みも書きもしない）:
    FAIL home: AREKA_IMPL_WATCH_HOME is not set
    FAIL home: AREKA_IMPL_WATCH_HOME is not an absolute path
    FAIL build: cargo exit <n>                       （cargo のエラーの行はその前にそのまま出る）
    FAIL build: no executable in cargo output
    FAIL place: copy
    FAIL place: rename
    OK renamed previous to areka-impl-watch.old-<UTC>.exe
    OK placed areka-impl-watch.exe
    OK removed <名前>
    KEEP <名前> (in use; run again later)
    OK check: --help exit 0
    FAIL check: --help exit <n>
    OK clear: state version changed; cleared; backup: <退避したファイル>
    FAIL clear: exit <n>                             （exe のエラーの行はその前にそのまま出る）

  終了コード: 0 置けて --help が 0 で終わり、版の合わない状態ファイルを残さなかった／1 FAIL の行を出した
#>
#Requires -Version 7

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$root = Split-Path $PSScriptRoot -Parent
$name = 'areka-impl-watch'

# Invoke-Utf8Child: read a child's output as UTF-8 regardless of the console code page
. (Join-Path $PSScriptRoot 'utf8-child.ps1')

# 1 home (never print its value)
$home_ = $env:AREKA_IMPL_WATCH_HOME
if ([string]::IsNullOrWhiteSpace($home_)) {
    'FAIL home: AREKA_IMPL_WATCH_HOME is not set'
    exit 1
}
if (-not [System.IO.Path]::IsPathFullyQualified($home_)) {
    'FAIL home: AREKA_IMPL_WATCH_HOME is not an absolute path'
    exit 1
}

# 2 build; the executable path comes from cargo's JSON messages (independent of CARGO_TARGET_DIR)
$r = Invoke-Utf8Child cargo @('build', '--release', '-q', '-p', $name,
    '--manifest-path', (Join-Path $root 'Cargo.toml'), '--message-format=json-render-diagnostics')
foreach ($l in $r.ErrLines) { $l }
if ($r.Code -ne 0) {
    "FAIL build: cargo exit $($r.Code)"
    exit 1
}
$built = $null
foreach ($line in $r.Out -split '\r?\n') {
    if (-not $line.StartsWith('{')) { continue }
    $m = $line | ConvertFrom-Json
    if ($m.reason -eq 'compiler-artifact' -and $m.target.name -eq $name -and $m.executable) {
        $built = $m.executable
    }
}
if (-not $built) {
    'FAIL build: no executable in cargo output'
    exit 1
}

# 3 copy as .new.exe
$exe = Join-Path $home_ "$name.exe"
$new = Join-Path $home_ "$name.new.exe"
try {
    [void][System.IO.Directory]::CreateDirectory($home_)
    [System.IO.File]::Copy($built, $new, $true)
}
catch {
    'FAIL place: copy'
    exit 1
}

# 4 rename current to .old-<UTC>.exe (works while it runs), then .new.exe to the real name
$oldName = "$name.old-$([DateTime]::UtcNow.ToString('yyyyMMdd\THHmmss\Z')).exe"
$old = Join-Path $home_ $oldName
$moved = $false
try {
    if ([System.IO.File]::Exists($exe)) {
        [System.IO.File]::Move($exe, $old)
        $moved = $true
    }
    [System.IO.File]::Move($new, $exe)
}
catch {
    try { if ($moved) { [System.IO.File]::Move($old, $exe) } } catch { }
    try { [System.IO.File]::Delete($new) } catch { }
    'FAIL place: rename'
    exit 1
}
if ($moved) { "OK renamed previous to $oldName" }
"OK placed $name.exe"

# 5 sweep old copies; a running one cannot be deleted and stays for the next run
foreach ($f in [System.IO.Directory]::GetFiles($home_, "$name.old*.exe")) {
    $leaf = [System.IO.Path]::GetFileName($f)
    try {
        [System.IO.File]::Delete($f)
        "OK removed $leaf"
    }
    catch {
        "KEEP $leaf (in use; run again later)"
    }
}

# 6 check (--help touches no state)
$c = Invoke-Utf8Child $exe @('--help')
if ($c.Code -ne 0) {
    "FAIL check: --help exit $($c.Code)"
    exit 1
}
'OK check: --help exit 0'

# 7 a state file of another version blocks every command; the new exe clears it once (no migration exists)
$s = Invoke-Utf8Child $exe @('status')
if ($s.Code -ne 0 -and ($s.ErrLines -match '^state file version mismatch')) {
    $k = Invoke-Utf8Child $exe @('clear')
    if ($k.Code -ne 0) {
        foreach ($l in $k.ErrLines) { $l }
        "FAIL clear: exit $($k.Code)"
        exit 1
    }
    "OK clear: state version changed; $($k.Out.Trim())"
}
exit 0
