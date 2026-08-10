<#
.SYNOPSIS
    areka-P0-file-slimming: リポジトリ全域の Rust ソースを走査し、
    `#[cfg(test)] mod ... { ... }` テストモジュールの行数を全数計測する。

.DESCRIPTION
    research.md §2.1/§2.2/§2.4 と同一条件の再計測を再現可能な形で行うためのスクリプト。

    計測定義（research と同一）:
      - 対象      : リポジトリ配下の全 *.rs（`target/`・`vendors/`・`.git/`・
                    `.claude/worktrees/`・`.kiro/` を除く）
                    ※ `.kiro/` の除外は spec 配下の検証用フィクスチャ
                       （`verification/fixtures/relocate/*.rs`）が実測値へ
                       混入するのを防ぐため（タスク 1.4 のレビューで確定）
      - テストコード行 = 各 `#[cfg(test)]` 属性行から対応する `mod` ブロックの
                    閉じ波括弧行までの行数（属性行・閉じ括弧行を含む）の総和
      - 文字列リテラル・raw 文字列・文字リテラル・行コメント・ブロックコメント
        （入れ子対応）の内部に現れる `#[cfg(test)]` や波括弧は計上しない
      - 宣言のみ（`#[cfg(test)] mod <name>;`）はテストコード行に計上せず、
        「既に本番ファイル外へ分離済み」の証跡として別集計する
      - `#[cfg(test)]` が `mod` 以外の項目に付くものは移設対象外として別集計する

.OUTPUTS
    scan_raw.csv          全 *.rs の計測結果（>500 行フィルタの再現元）
    target_inventory.csv  テストコード > 500 行のファイル（必須対象）
    excluded_inventory.csv 既に本番ファイル外へ分離済みのテストファイル（要件 1.4）
    scan_summary.txt      区分別集計と診断カウンタ

.EXAMPLE
    pwsh -File .kiro/specs/areka-P0-file-slimming/verification/Measure-TestModules.ps1
#>
[CmdletBinding()]
param(
    # 既定はこのスクリプトから 4 階層上（= リポジトリルート）
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path,
    [string]$OutDir   = $PSScriptRoot,
    [int]$Threshold   = 500
)

$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Text;
using System.Text.RegularExpressions;

public class RustTestScan
{
    public class ModBlock
    {
        public string Name;
        public int Start;
        public int End;
        public int Lines { get { return End - Start + 1; } }
    }

    public class FileResult
    {
        public int TotalLines;
        public int TestLines;
        public int BlockCount;
        public int DeclCount;
        public int NonModCount;
        public int InlineAttrCount;   // 行頭以外に現れた #[cfg(test)]（診断）
        public int LooseCfgCount;     // #[cfg(all(test,..))] 等の変種（診断）
        public List<ModBlock> Blocks = new List<ModBlock>();
        public List<ModBlock> Decls  = new List<ModBlock>();
        public List<int> NonModLines = new List<int>();
    }

    static readonly Regex AttrRe =
        new Regex(@"^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", RegexOptions.Compiled);
    static readonly Regex AttrAnywhereRe =
        new Regex(@"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", RegexOptions.Compiled);
    static readonly Regex LooseRe =
        new Regex(@"#\s*\[\s*cfg\s*\([^)]*\btest\b", RegexOptions.Compiled);

    string[] code;
    int L;
    int C;

    static bool IsIdent(char c)
    {
        return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_';
    }

    // 文字列/raw 文字列/文字リテラル/コメントの中身を空白へ潰した「コードのみ」の行配列を返す。
    // 状態は行をまたいで保持する（Rust の文字列とブロックコメントは複数行にわたる）。
    public static string[] Tokenize(string[] lines)
    {
        string[] outp = new string[lines.Length];
        int state = 0;      // 0 normal / 1 block comment / 2 string / 3 raw string
        int blockDepth = 0;
        int rawHashes = 0;

        for (int li = 0; li < lines.Length; li++)
        {
            string s = lines[li] ?? "";
            int n = s.Length;
            StringBuilder sb = new StringBuilder(n);
            int i = 0;
            while (i < n)
            {
                char c = s[i];

                if (state == 1)
                {
                    if (c == '/' && i + 1 < n && s[i + 1] == '*') { blockDepth++; sb.Append("  "); i += 2; continue; }
                    if (c == '*' && i + 1 < n && s[i + 1] == '/')
                    {
                        blockDepth--; sb.Append("  "); i += 2;
                        if (blockDepth <= 0) { blockDepth = 0; state = 0; }
                        continue;
                    }
                    sb.Append(' '); i++; continue;
                }

                if (state == 2)
                {
                    if (c == '\\') { sb.Append(' '); i++; if (i < n) { sb.Append(' '); i++; } continue; }
                    if (c == '"') { sb.Append('"'); i++; state = 0; continue; }
                    sb.Append(' '); i++; continue;
                }

                if (state == 3)
                {
                    if (c == '"')
                    {
                        int k = i + 1; int h = 0;
                        while (k < n && s[k] == '#' && h < rawHashes) { k++; h++; }
                        if (h == rawHashes)
                        {
                            for (int q = i; q < k; q++) sb.Append(' ');
                            i = k; state = 0; continue;
                        }
                    }
                    sb.Append(' '); i++; continue;
                }

                // ---- normal ----
                if (c == '/' && i + 1 < n && s[i + 1] == '/')
                {
                    for (int q = i; q < n; q++) sb.Append(' ');
                    i = n; continue;
                }
                if (c == '/' && i + 1 < n && s[i + 1] == '*')
                {
                    state = 1; blockDepth = 1; sb.Append("  "); i += 2; continue;
                }
                if ((c == 'r' || c == 'b') && (i == 0 || !IsIdent(s[i - 1])))
                {
                    int k = i;
                    if (c == 'b') { if (k + 1 < n && s[k + 1] == 'r') k++; else k = -1; }
                    if (k >= 0 && s[k] == 'r')
                    {
                        int j = k + 1; int h = 0;
                        while (j < n && s[j] == '#') { j++; h++; }
                        if (j < n && s[j] == '"')
                        {
                            for (int q = i; q <= j; q++) sb.Append(' ');
                            rawHashes = h; state = 3; i = j + 1; continue;
                        }
                    }
                }
                if (c == '"') { sb.Append('"'); state = 2; i++; continue; }
                if (c == '\'')
                {
                    if (i + 1 < n && s[i + 1] == '\\')
                    {
                        int j = i + 2;
                        while (j < n && s[j] != '\'') j++;
                        if (j < n) { for (int q = i; q <= j; q++) sb.Append(' '); i = j + 1; continue; }
                    }
                    else if (i + 2 < n && s[i + 2] == '\'')
                    {
                        sb.Append("   "); i += 3; continue;
                    }
                    sb.Append(c); i++; continue;   // ライフタイム
                }
                sb.Append(c); i++;
            }
            outp[li] = sb.ToString();
        }
        return outp;
    }

    bool End { get { return L >= code.Length; } }

    char Peek()
    {
        if (L >= code.Length) return '\0';
        if (C >= code[L].Length) return '\n';
        return code[L][C];
    }

    void Adv()
    {
        if (L >= code.Length) return;
        if (C >= code[L].Length) { L++; C = 0; }
        else C++;
    }

    void SkipWs()
    {
        while (!End)
        {
            char c = Peek();
            if (c == ' ' || c == '\t' || c == '\r' || c == '\n') Adv();
            else break;
        }
    }

    void SkipBalanced(char open, char close)
    {
        int depth = 0;
        while (!End)
        {
            char c = Peek();
            if (c == open) depth++;
            else if (c == close) depth--;
            Adv();
            if (depth == 0) break;
        }
    }

    void SkipAttrs()
    {
        while (!End)
        {
            SkipWs();
            if (Peek() != '#') break;
            Adv();
            SkipWs();
            if (Peek() == '!') { Adv(); SkipWs(); }
            if (Peek() != '[') break;
            SkipBalanced('[', ']');
        }
    }

    string ReadWord()
    {
        StringBuilder sb = new StringBuilder();
        while (!End)
        {
            char c = Peek();
            if (IsIdent(c)) { sb.Append(c); Adv(); }
            else break;
        }
        return sb.ToString();
    }

    public static FileResult Analyze(string[] lines)
    {
        RustTestScan self = new RustTestScan();
        return self.Run(lines);
    }

    FileResult Run(string[] lines)
    {
        code = Tokenize(lines);
        FileResult r = new FileResult();
        r.TotalLines = lines.Length;

        int i = 0;
        while (i < code.Length)
        {
            string ln = code[i];
            if (ln.IndexOf("cfg", StringComparison.Ordinal) < 0) { i++; continue; }

            Match m = AttrRe.Match(ln);
            if (!m.Success)
            {
                if (AttrAnywhereRe.IsMatch(ln)) r.InlineAttrCount++;
                else if (LooseRe.IsMatch(ln)) r.LooseCfgCount++;
                i++; continue;
            }

            int startLine = i + 1;
            L = i; C = m.Index + m.Length;
            SkipAttrs();
            SkipWs();

            string w = ReadWord();
            if (w == "pub")
            {
                SkipWs();
                if (Peek() == '(') SkipBalanced('(', ')');
                SkipWs();
                w = ReadWord();
            }
            if (w == "unsafe" || w == "async") { SkipWs(); w = ReadWord(); }

            if (w != "mod")
            {
                r.NonModCount++;
                r.NonModLines.Add(startLine);
                i = startLine;   // 次の行から再開
                continue;
            }

            SkipWs();
            string name = ReadWord();
            SkipWs();
            char c2 = Peek();

            if (c2 == ';')
            {
                int endLine = L + 1;
                r.Decls.Add(new ModBlock { Name = name, Start = startLine, End = endLine });
                r.DeclCount++;
                i = endLine;
                continue;
            }

            if (c2 == '{')
            {
                int depth = 0;
                while (!End)
                {
                    char ch = Peek();
                    if (ch == '{') depth++;
                    else if (ch == '}')
                    {
                        depth--;
                        if (depth == 0) break;
                    }
                    Adv();
                }
                int endLine = L + 1;
                r.Blocks.Add(new ModBlock { Name = name, Start = startLine, End = endLine });
                r.BlockCount++;
                r.TestLines += (endLine - startLine + 1);
                i = endLine;
                continue;
            }

            i = startLine;
        }
        return r;
    }
}
'@ -Language CSharp

$RepoRoot = (Resolve-Path $RepoRoot).Path
Write-Host "RepoRoot : $RepoRoot"
Write-Host "OutDir   : $OutDir"

# NOTE: 判定は必ず「リポジトリルートからの相対パス」に対して行う。
# 本ブランチのワークツリー自身が `...\.claude\worktrees\...` 配下にあるため、
# 絶対パスへ適用すると全ファイルが除外されてしまう。
# NOTE: `.kiro/` も除外する。spec 配下には検証スクリプトのテスト用フィクスチャ
# （`verification/fixtures/relocate/*.rs` 等）が置かれ、その一部は本物の
# `#[cfg(test)] mod` ブロックを含む。除外しないと spec 自身のフィクスチャが
# リポジトリ全域の実測値へ混入する（実測: 619 → 627 ファイル）。
$excludeRe = '^(target|vendors|\.git|node_modules)/|^\.claude/worktrees/|^\.kiro/'

$files = Get-ChildItem -Path $RepoRoot -Recurse -Filter '*.rs' -File |
    Where-Object {
        $rel0 = $_.FullName.Substring($RepoRoot.Length).TrimStart('\', '/').Replace('\', '/')
        $rel0 -notmatch $excludeRe
    } |
    Sort-Object FullName

Write-Host ("Rust files: {0}" -f $files.Count)

$rows  = New-Object System.Collections.Generic.List[object]
$decls = New-Object System.Collections.Generic.List[object]

foreach ($f in $files) {
    $lines = [System.IO.File]::ReadAllLines($f.FullName)
    $res   = [RustTestScan]::Analyze($lines)

    $rel = $f.FullName.Substring($RepoRoot.Length).TrimStart('\', '/').Replace('\', '/')

    $crate = ''
    $area  = 'other'
    if ($rel -match '^crates/([^/]+)/([^/]+)/') {
        $crate = $Matches[1]
        $seg   = $Matches[2]
        if ($seg -in @('src', 'tests', 'examples', 'benches')) { $area = $seg }
    } elseif ($rel -match '^crates/([^/]+)/build\.rs$') {
        $crate = $Matches[1]; $area = 'build.rs'
    }

    $blockDesc = ($res.Blocks | ForEach-Object { "$($_.Name):$($_.Start)-$($_.End)($($_.Lines))" }) -join '|'
    $declDesc  = ($res.Decls  | ForEach-Object { "$($_.Name)@$($_.Start)" }) -join '|'

    $rows.Add([pscustomobject]@{
        path          = $rel
        crate         = $crate
        area          = $area
        total_lines   = $res.TotalLines
        test_lines    = $res.TestLines
        body_lines    = $res.TotalLines - $res.TestLines
        module_count  = $res.BlockCount
        decl_count    = $res.DeclCount
        nonmod_count  = $res.NonModCount
        inline_attr   = $res.InlineAttrCount
        loose_cfg     = $res.LooseCfgCount
        modules       = $blockDesc
        declarations  = $declDesc
        nonmod_lines  = ($res.NonModLines -join '|')
    })

    # 宣言のみ（= 既に本番ファイル外へ分離済み）の解決
    foreach ($d in $res.Decls) {
        $dir  = Split-Path -Parent $f.FullName
        $base = $f.BaseName
        if ($base -notin @('mod', 'lib', 'main')) { $dir = Join-Path $dir $base }
        $c1 = Join-Path $dir ($d.Name + '.rs')
        $c2 = Join-Path $dir (Join-Path $d.Name 'mod.rs')
        $resolved = $null
        if (Test-Path -LiteralPath $c1) { $resolved = $c1 }
        elseif (Test-Path -LiteralPath $c2) { $resolved = $c2 }

        $rlines = -1
        $rrel   = '(unresolved)'
        if ($resolved) {
            $rrel   = (Resolve-Path -LiteralPath $resolved).Path.Substring($RepoRoot.Length).TrimStart('\', '/').Replace('\', '/')
            $rlines = ([System.IO.File]::ReadAllLines($resolved)).Length
        }

        $decls.Add([pscustomobject]@{
            declaring_file  = $rel
            declaring_line  = $d.Start
            module_name     = $d.Name
            separated_file  = $rrel
            separated_lines = $rlines
            crate           = $crate
            area            = $area
        })
    }
}

$rawPath      = Join-Path $OutDir 'scan_raw.csv'
$targetPath   = Join-Path $OutDir 'target_inventory.csv'
$excludedPath = Join-Path $OutDir 'excluded_inventory.csv'
$summaryPath  = Join-Path $OutDir 'scan_summary.txt'

$rows | Sort-Object path | Export-Csv -Path $rawPath -NoTypeInformation -Encoding utf8

$targets = $rows | Where-Object { $_.test_lines -gt $Threshold } | Sort-Object { -$_.test_lines }
$targets |
    Select-Object path, total_lines, test_lines, module_count, crate |
    Export-Csv -Path $targetPath -NoTypeInformation -Encoding utf8

$decls | Sort-Object declaring_file, declaring_line |
    Export-Csv -Path $excludedPath -NoTypeInformation -Encoding utf8

$sb = New-Object System.Text.StringBuilder
function Add-Line([string]$t) { $null = $sb.AppendLine($t) }

Add-Line ("scan date        : {0}" -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'))
Add-Line ("repo root        : {0}" -f $RepoRoot)
Add-Line ("git branch       : {0}" -f (git -C $RepoRoot rev-parse --abbrev-ref HEAD))
Add-Line ("git HEAD         : {0}" -f (git -C $RepoRoot rev-parse --short HEAD))
Add-Line ("threshold        : test_lines > {0}" -f $Threshold)
Add-Line ''
Add-Line '=== area breakdown ==='
Add-Line ('{0,-12} {1,7} {2,10} {3,10}' -f 'area', 'files', 'total', 'test_lines')
foreach ($g in ($rows | Group-Object area | Sort-Object Name)) {
    Add-Line ('{0,-12} {1,7} {2,10} {3,10}' -f $g.Name, $g.Count,
        (($g.Group | Measure-Object total_lines -Sum).Sum),
        (($g.Group | Measure-Object test_lines  -Sum).Sum))
}
Add-Line ('{0,-12} {1,7} {2,10} {3,10}' -f 'TOTAL', $rows.Count,
    (($rows | Measure-Object total_lines -Sum).Sum),
    (($rows | Measure-Object test_lines  -Sum).Sum))
Add-Line ''
Add-Line '=== targets (test_lines > threshold) ==='
Add-Line ("count            : {0}" -f $targets.Count)
Add-Line ("test lines total : {0}" -f (($targets | Measure-Object test_lines -Sum).Sum))
Add-Line ("crates           : {0}" -f (($targets | Group-Object crate).Count))
Add-Line ("multi-module     : {0}" -f (@($targets | Where-Object { $_.module_count -gt 1 }).Count))
Add-Line ''
Add-Line '=== target breakdown by crate ==='
foreach ($g in ($targets | Group-Object crate | Sort-Object { -(($_.Group | Measure-Object test_lines -Sum).Sum) })) {
    Add-Line ('{0,-26} {1,3} files {2,7} test lines' -f $g.Name, $g.Count,
        (($g.Group | Measure-Object test_lines -Sum).Sum))
}
Add-Line ''
Add-Line '=== diagnostics ==='
Add-Line ("in-file test module blocks   : {0}" -f (($rows | Measure-Object module_count -Sum).Sum))
Add-Line ("files with in-file blocks    : {0}" -f (@($rows | Where-Object { $_.module_count -gt 0 }).Count))
Add-Line ("declaration-only mod sites   : {0}" -f (($rows | Measure-Object decl_count -Sum).Sum))
Add-Line ("files declaring separated mod: {0}" -f (@($rows | Where-Object { $_.decl_count -gt 0 }).Count))
Add-Line ("non-mod #[cfg(test)] items   : {0}" -f (($rows | Measure-Object nonmod_count -Sum).Sum))
Add-Line ("inline (non-line-start) attrs: {0}" -f (($rows | Measure-Object inline_attr -Sum).Sum))
Add-Line ("loose cfg(...test...) variants: {0}" -f (($rows | Measure-Object loose_cfg -Sum).Sum))
Add-Line ("unresolved separated modules : {0}" -f (@($decls | Where-Object { $_.separated_file -eq '(unresolved)' }).Count))

$sb.ToString() | Set-Content -Path $summaryPath -Encoding utf8
Write-Host $sb.ToString()
Write-Host "wrote: $rawPath"
Write-Host "wrote: $targetPath"
Write-Host "wrote: $excludedPath"
Write-Host "wrote: $summaryPath"
