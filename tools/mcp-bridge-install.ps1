<#
.SYNOPSIS
  Claude Desktop／Claude Code 向けの中継 areka-mcp-bridge を、このスクリプトの在る作業木のソースから
  release でビルドし、環境変数 AREKA_MCP_HOME が指す置き場所へ置く（走っていれば止めずに入れ替える）。

.DESCRIPTION
  使い方（引数なし・どのフォルダから呼んでも同じに動く・今のフォルダを変えない）:
    pwsh -NoProfile -File <リポジトリの絶対パス>\tools\mcp-bridge-install.ps1

  手 1〜6 は共通の手順 exe-install.ps1 が行う（このスクリプトは名前・環境変数の名前・確かめを渡すだけ）。
  順:
    1 環境変数を読む。空・空白だけ・絶対パスでない（/c/...・\foo・C:foo を含む）なら何もせず終わる。
      値は画面へ出さず、設定も変更もしない。
    2 cargo build --release -q -p areka-mcp-bridge でビルドし、実行ファイルの道筋は cargo の JSON の報せ
      （executable）から UTF-8 のバイトとして読む（Invoke-Utf8Child）。失敗なら置き場所に触らない。
    3 置き場所が無ければ作り、新しいものを areka-mcp-bridge.new.exe として写す。
    4 写した .new.exe を標準入力を閉じて起こし、15 秒のうちに終了コード 0 で終わるのを確かめる
      （中継は引数を読まない・標準入力の終わりで終わる）。15 秒を過ぎたら止め、終わるのを待つ。
      落ちたら .new.exe を消し、今の exe と古い写しには触らない。
    5 今の areka-mcp-bridge.exe を areka-mcp-bridge.old-<UTC>.exe（20261022T020500Z の形）へ改名し、
      .new.exe を areka-mcp-bridge.exe へ改名する。2 つ目の改名に失敗したら古いものを元へ戻し、.new.exe を消す。
    6 areka-mcp-bridge.old*.exe を消してみる。Claude が起こしたものが走っていて消せなければ残す。

  自分で触るのは置き場所の areka-mcp-bridge.exe・areka-mcp-bridge.new.exe・areka-mcp-bridge.old*.exe だけ。

  出る行（ASCII だけ・端末の文字コードは読みも書きもしない）:
    FAIL home: AREKA_MCP_HOME is not set
    FAIL home: AREKA_MCP_HOME is not an absolute path
    FAIL build: cargo exit <n>                       （cargo のエラーの行はその前にそのまま出る）
    FAIL build: no executable in cargo output
    FAIL place: copy
    OK check: exit 0 on closed stdin
    FAIL check: exit <n> on closed stdin             （置いていない・前のものは元のまま）
    FAIL check: still running after 15s              （同上）
    FAIL check: cannot start: <理由>                 （同上）
    FAIL cleanup: cannot remove areka-mcp-bridge.new.exe: <理由>
    FAIL place: rename                               （戻せなければ ; restore failed が付く）
    OK renamed previous to areka-mcp-bridge.old-<UTC>.exe
    OK placed areka-mcp-bridge.exe
    OK removed <名前>
    KEEP <名前> (in use; run again later)

  終了コード: 0 確かめが 0 で終わって置けた／1 FAIL の行を出した
#>
#Requires -Version 7

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$root = Split-Path $PSScriptRoot -Parent
$name = 'areka-mcp-bridge'

# Test-InstallHome / Build-InstallExe / Install-ToolExe (steps 1-6)
. (Join-Path $PSScriptRoot 'exe-install.ps1')

# 1 home (never print its value)
$fail = Test-InstallHome 'AREKA_MCP_HOME' $env:AREKA_MCP_HOME
if ($fail) {
    $fail
    exit 1
}

# 2 build
$b = Build-InstallExe -Name $name -Root $root
$b.Lines
if (-not $b.Path) { exit 1 }

# 3-6 copy, check the new exe, replace, sweep (a running old one Claude started stays for the next run)
$i = Install-ToolExe -HomeDir $env:AREKA_MCP_HOME -Name $name -Built $b.Path -Check {
    # the bridge reads no arguments and quits when stdin ends
    param([string]$ExePath)
    $psi = [System.Diagnostics.ProcessStartInfo]::new($ExePath)
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $p = [System.Diagnostics.Process]::Start($psi)
    try {
        $outTask = $p.StandardOutput.ReadToEndAsync()
        $errTask = $p.StandardError.ReadToEndAsync()
        $p.StandardInput.Close()
        if (-not $p.WaitForExit(15000)) {
            # wait for the killed child to end so the caller can delete the .new.exe it ran from
            try { $p.Kill() } catch { }
            [void]$p.WaitForExit(5000)
            return [pscustomobject]@{ Ok = $false; Line = 'FAIL check: still running after 15s' }
        }
        [void]$outTask.GetAwaiter().GetResult()
        [void]$errTask.GetAwaiter().GetResult()
        if ($p.ExitCode -ne 0) {
            return [pscustomobject]@{ Ok = $false; Line = "FAIL check: exit $($p.ExitCode) on closed stdin" }
        }
        [pscustomobject]@{ Ok = $true; Line = 'OK check: exit 0 on closed stdin' }
    }
    finally {
        $p.Dispose()
    }
}
$i.Lines
exit $i.Code
