<#
.SYNOPSIS
  机の貸し出しのアプリ areka-impl-watch を、このスクリプトの在る作業木のソースから release でビルドし、
  環境変数 AREKA_IMPL_WATCH_HOME が指す置き場所へ置く（走っていれば止めずに入れ替える）。

.DESCRIPTION
  使い方（引数なし・どのフォルダから呼んでも同じに動く・今のフォルダを変えない）:
    pwsh -NoProfile -File <リポジトリの絶対パス>\tools\impl-watch-install.ps1

  手 1〜6 は共通の手順 exe-install.ps1 が行う（このスクリプトは名前・環境変数の名前・確かめを渡すだけ）。
  順:
    1 環境変数を読む。空・空白だけ・絶対パスでない（/c/...・\foo・C:foo を含む）なら何もせず終わる。
      値は画面へ出さず、設定も変更もしない。
    2 cargo build --release -q -p areka-impl-watch でビルドし、実行ファイルの道筋は cargo の JSON の報せ
      （executable）から UTF-8 のバイトとして読む（Invoke-Utf8Child）。失敗なら置き場所に触らない。
    3 置き場所が無ければ作り、新しいものを areka-impl-watch.new.exe として写す。
    4 写した .new.exe の --help を呼んで確かめる。落ちたら .new.exe を消し、今の exe と古い写しには触らない。
    5 今の areka-impl-watch.exe を areka-impl-watch.old-<UTC>.exe（20261016T020500Z の形）へ改名し、
      .new.exe を areka-impl-watch.exe へ改名する。2 つ目の改名に失敗したら古いものを元へ戻し、.new.exe を消す。
    6 areka-impl-watch.old*.exe（前の形の .old.exe も）を消してみる。走っていて消せなければ残す。
    7 置いたものの status を呼ぶ。状態ファイルの版が合わない（標準エラーが state file version mismatch で
      始まる）ときだけ、置いたもので clear を 1 回呼ぶ。版が合っていれば state.json には触らない。
      ほかの失敗なら status の標準エラーの最後の行と FAIL status の行を出して終わる。

  自分で触るのは置き場所の areka-impl-watch.exe・areka-impl-watch.new.exe・areka-impl-watch.old*.exe だけ。
  state.json・居る印・退避のファイルは手 7 の clear（exe が行う）でだけ変わる。手 7 の status（exe が行う）は
  status.md を書き直し、state.lock を一瞬握り、impl-watch.log を開く（無ければ作る）。

  出る行（ASCII だけ・端末の文字コードは読みも書きもしない）:
    FAIL home: AREKA_IMPL_WATCH_HOME is not set
    FAIL home: AREKA_IMPL_WATCH_HOME is not an absolute path
    FAIL build: cargo exit <n>                       （cargo のエラーの行はその前にそのまま出る）
    FAIL build: no executable in cargo output
    FAIL place: copy
    OK check: --help exit 0
    FAIL check: --help exit <n>                      （置いていない・前のものは元のまま）
    FAIL check: cannot start: <理由>                 （同上）
    FAIL cleanup: cannot remove areka-impl-watch.new.exe: <理由>
    FAIL place: rename                               （戻せなければ ; restore failed が付く）
    OK renamed previous to areka-impl-watch.old-<UTC>.exe
    OK placed areka-impl-watch.exe
    OK removed <名前>
    KEEP <名前> (in use; run again later)
    OK clear: state version changed; cleared; backup: <退避したファイル>
    FAIL clear: exit <n>                             （exe のエラーの行はその前にそのまま出る）
    FAIL status: exit <n>                            （status の標準エラーの最後の行がその前に出る）

  終了コード: 0 確かめが通って置け、版の合わない状態ファイルを残さず、status が読めた／1 FAIL の行を出した
#>
#Requires -Version 7

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$root = Split-Path $PSScriptRoot -Parent
$name = 'areka-impl-watch'

# Test-InstallHome / Build-InstallExe / Install-ToolExe / Get-StateStatusAction (steps 1-6 and the step 7 decision)
. (Join-Path $PSScriptRoot 'exe-install.ps1')

# 1 home (never print its value)
$fail = Test-InstallHome 'AREKA_IMPL_WATCH_HOME' $env:AREKA_IMPL_WATCH_HOME
if ($fail) {
    $fail
    exit 1
}
$home_ = $env:AREKA_IMPL_WATCH_HOME

# 2 build
$b = Build-InstallExe -Name $name -Root $root
$b.Lines
if (-not $b.Path) { exit 1 }

# 3-6 copy, check the new exe (--help touches no state), replace, sweep
$i = Install-ToolExe -HomeDir $home_ -Name $name -Built $b.Path -Check {
    param([string]$ExePath)
    $c = Invoke-Utf8Child $ExePath @('--help')
    if ($c.Code -eq 0) { return [pscustomobject]@{ Ok = $true; Line = 'OK check: --help exit 0' } }
    [pscustomobject]@{ Ok = $false; Line = "FAIL check: --help exit $($c.Code)" }
}
$i.Lines
if ($i.Code -ne 0) { exit 1 }

# 7 a state file of another version blocks every command; the new exe clears it once (no migration exists)
$exe = Join-Path $home_ "$name.exe"
$s = Invoke-Utf8Child $exe @('status')
switch (Get-StateStatusAction $s.Code $s.ErrLines) {
    'clear' {
        $k = Invoke-Utf8Child $exe @('clear')
        if ($k.Code -ne 0) {
            foreach ($l in $k.ErrLines) { $l }
            "FAIL clear: exit $($k.Code)"
            exit 1
        }
        "OK clear: state version changed; $($k.Out.Trim())"
    }
    'fail' {
        if ($s.ErrLines.Count) { Show-Ascii $s.ErrLines[-1] 0 }
        "FAIL status: exit $($s.Code)"
        exit 1
    }
}
exit 0
