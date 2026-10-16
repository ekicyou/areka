<#
.SYNOPSIS
  子の PowerShell に .ps1 を回させる引数の並びを、UTF-8 の書き手の前置き付きで組む関数 Get-PwshChildArguments を定義する。

.DESCRIPTION
  読み込み用（. で読み込む）。単独で回しても何もしない。
    . (Join-Path $PSScriptRoot 'perf-pwsh-child.ps1')

  PowerShell は出力先が渡された handle でも [Console]::OutputEncoding（＝端末の文字コード）で書くため、
  932 の端末から素の -File で回した子の出力は 932 のバイトで来る。前置きは、子のプロセスの中で
  その子自身の標準出力と標準エラーだけを BOM の無い UTF-8 の書き手（AutoFlush を立てる）に替える。
  使うのは [Console]::SetOut／[Console]::SetError だけで、端末の文字コードの設定には触れない
  （共有の端末は子が終わっても元のまま）。前置きの文の所在はこのファイルだけ。

  引数の引用の規則: -SelfTest のような引数名の形（^-[A-Za-z][A-Za-z0-9]*$）は素で渡し、
  それ以外は単引用符で括って ' を '' にする（引用すると値として渡ってしまう形があるため）。

  返り値: @('-NoProfile', '-NonInteractive', '-Command', '<前置き>; & ''<ScriptPath>'' <引数>; exit $LASTEXITCODE')
  これを ProcessStartInfo.ArgumentList（Invoke-Child・Invoke-Utf8Child）に渡して子の pwsh を起こす。
  子の終了コードはスクリプトの exit の値になる。
#>

function Get-PwshChildArguments {
    [OutputType([string[]])]
    param(
        [Parameter(Mandatory)][string]$ScriptPath,   # 回す .ps1 の絶対パス
        [string[]]$Arguments = @()                    # 子のスクリプトへ渡す引数（1 要素 1 引数）
    )
    # 前置きは自分だけの範囲（& { … }）で回し、子のスクリプトから見える変数を残さない。
    $prefix = '& { $utf8 = [System.Text.UTF8Encoding]::new($false); ' +
              '$out = [System.IO.StreamWriter]::new([Console]::OpenStandardOutput(), $utf8); $out.AutoFlush = $true; [Console]::SetOut($out); ' +
              '$err = [System.IO.StreamWriter]::new([Console]::OpenStandardError(), $utf8); $err.AutoFlush = $true; [Console]::SetError($err) }'
    $parts = @("& '" + ($ScriptPath -replace "'", "''") + "'")
    foreach ($argument in $Arguments) {
        $text = [string]$argument
        if ($text -match '^-[A-Za-z][A-Za-z0-9]*$') { $parts += $text }
        else { $parts += ("'" + ($text -replace "'", "''") + "'") }
    }
    $command = $prefix + '; ' + ($parts -join ' ') + '; exit $LASTEXITCODE'
    return @('-NoProfile', '-NonInteractive', '-Command', $command)
}
