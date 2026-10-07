<#
.SYNOPSIS
  子のプロセスを起こし、標準出力と標準エラーを UTF-8 のバイトとして読んで持ち帰る関数 Invoke-Utf8Child を定義する。

.DESCRIPTION
  読み込み用（. で読み込む）。単独で回しても何もしない。
    . (Join-Path $PSScriptRoot 'utf8-child.ps1')   # tools/ のスクリプトから
    . ./tools/utf8-child.ps1                       # 作業ツリーの根から（release.yml）

  PowerShell の素の外部コマンド呼び出しは子の出力を端末の文字コード（[Console]::OutputEncoding）で解くため、
  端末が 932 のときに UTF-8 の日本語を含む出力（cargo metadata の description など）が壊れる。
  この関数は端末を通さずに子の出力を UTF-8 で解く。

  約束:
    端末の文字コードの設定を読みも書きもしない。子の出力も環境変数も印字しない（印字は呼ぶ側の仕事）。
    子の作業フォルダは今の PowerShell の場所（.NET のプロセスの作業フォルダは Set-Location で動かないので明示する）。
    コマンド名は Get-Command -CommandType Application で解く（素の呼び出しと同じ PATH の探し方）。
    見つからない・起こせないときは throw。子の終了コードが 0 でないことは throw にしない（Code で返す）。

  返り値: [pscustomobject]@{
    Code     = [int]       子の終了コード
    Out      = [string]    標準出力の全部（UTF-8 で解いた字・BOM は外れる）
    ErrLines = [string[]]  標準エラーを行に分け、空の行を除いた並び（無ければ空の並び）
  }
#>

function Invoke-Utf8Child {
    [OutputType([pscustomobject])]
    param(
        [Parameter(Mandatory)][string]$FilePath,   # コマンド名か絶対パス
        [string[]]$ArgumentList = @(),             # 1 要素 1 引数（ProcessStartInfo.ArgumentList に渡す・引用は .NET が付ける）
        [switch]$OwnConsole                        # 窓の無い自分だけの端末で起こす（判定の子だけが使う）
    )

    $cmd = Get-Command -Name $FilePath -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $cmd) {
        throw "Invoke-Utf8Child: command not found: $FilePath"
    }

    # BOM を書かない UTF-8。読む側の StreamReader は BOM を見つければ外す
    $utf8 = [System.Text.UTF8Encoding]::new($false)
    $psi = [System.Diagnostics.ProcessStartInfo]::new($cmd.Source)
    foreach ($a in $ArgumentList) { $psi.ArgumentList.Add($a) }
    $psi.WorkingDirectory = (Get-Location -PSProvider FileSystem).ProviderPath
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = $utf8
    $psi.StandardErrorEncoding = $utf8
    # 既定は端末を分け合う（Ctrl+C で子も道連れに止まる）。-OwnConsole のときだけ窓の無い自分だけの端末
    $psi.CreateNoWindow = [bool]$OwnConsole

    $p = [System.Diagnostics.Process]::Start($psi)
    try {
        # 両方の流れを先に読み始めてから終了を待つ（片方が埋まって子が止まるのを避ける）
        $outTask = $p.StandardOutput.ReadToEndAsync()
        $errTask = $p.StandardError.ReadToEndAsync()
        $p.WaitForExit()
        $out = $outTask.GetAwaiter().GetResult()
        $err = $errTask.GetAwaiter().GetResult()
        return [pscustomobject]@{
            Code     = [int]$p.ExitCode
            Out      = [string]$out
            ErrLines = [string[]]@($err -split '\r?\n' | Where-Object { $_ -ne '' })
        }
    }
    finally {
        $p.Dispose()
    }
}
