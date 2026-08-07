<#
.SYNOPSIS
    areka-P0-file-slimming: Rust ソースの字句正規化（文字列/コメント潰し）と
    項目単位パースを提供する共有ライブラリ。dot-source して使う。

.DESCRIPTION
    `Measure-TestModules.ps1`（タスク 1.1）の C# トークナイザ `RustTestScan.Tokenize`
    を逐語で持ち上げ、テストモジュールブロックの行範囲探索（`FindTestModules`）と
    モジュール本体の項目単位分解（`ParseItems`）を追加したもの。

    「2 本目の弱いパーサ」にならないよう、持ち上げの等価性は
    `Test-VerificationTools.ps1` の TOKENIZER-EQUIV ケースで機械検証する
    （`scan_raw.csv` の `modules` 列＝タスク 1.1 の実測ブロック範囲を全ファイル再現できること）。

.NOTES
    Requirements: 1.8 / 2.3 / 2.4 / 2.9
    Design:       §検証 / EvidencePipeline（証跡パイプライン）, §実装 / ThemeSplit
#>

if (-not ('RustItemScan' -as [type])) {
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Text;
using System.Text.RegularExpressions;

public class RustItemScan
{
    public class ModBlock
    {
        public string Name;
        public int Start;       // 1-based: #[cfg(test)] 属性行
        public int BraceLine;   // 1-based: `{` のある行
        public int End;         // 1-based: 対応する `}` のある行
        public int Lines { get { return End - Start + 1; } }
    }

    public class Item
    {
        public int Start;       // 1-based inclusive（先行コメント/属性を含む）
        public int End;         // 1-based inclusive
        public string Kind;     // "use" | "mod_decl" | "mod" | "fn" | "comment" | "other"
        public string Name;     // fn / mod の識別子（無ければ ""）
        public bool IsTest;     // #[test] 系属性を伴う fn
    }

    static readonly Regex AttrRe =
        new Regex(@"^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", RegexOptions.Compiled);
    static readonly Regex TestAttrRe =
        new Regex(@"^\s*#\s*\[\s*(?:[A-Za-z_][A-Za-z0-9_]*\s*::\s*)*test\s*[\]\(]", RegexOptions.Compiled);

    string[] code;
    int L;
    int C;

    static bool IsIdent(char c)
    {
        return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_';
    }

    // ---- Measure-TestModules.ps1 の RustTestScan.Tokenize と逐語同一 ----
    // 文字列/raw 文字列/文字リテラル/コメントの中身を空白へ潰した「コードのみ」の行配列を返す。
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

    bool AtEnd { get { return L >= code.Length; } }

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
        while (!AtEnd)
        {
            char c = Peek();
            if (c == ' ' || c == '\t' || c == '\r' || c == '\n') Adv();
            else break;
        }
    }

    void SkipBalanced(char open, char close)
    {
        int depth = 0;
        while (!AtEnd)
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
        while (!AtEnd)
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
        while (!AtEnd)
        {
            char c = Peek();
            if (IsIdent(c)) { sb.Append(c); Adv(); }
            else break;
        }
        return sb.ToString();
    }

    // ======================= テストモジュールブロックの探索 =======================

    public static List<ModBlock> FindTestModules(string[] lines)
    {
        RustItemScan self = new RustItemScan();
        return self.RunMods(lines);
    }

    List<ModBlock> RunMods(string[] lines)
    {
        code = Tokenize(lines);
        List<ModBlock> blocks = new List<ModBlock>();

        int i = 0;
        while (i < code.Length)
        {
            string ln = code[i];
            if (ln.IndexOf("cfg", StringComparison.Ordinal) < 0) { i++; continue; }

            Match m = AttrRe.Match(ln);
            if (!m.Success) { i++; continue; }

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

            if (w != "mod") { i = startLine; continue; }

            SkipWs();
            string name = ReadWord();
            SkipWs();
            char c2 = Peek();

            if (c2 == ';') { i = L + 1; continue; }   // 宣言のみ（既に分離済み）

            if (c2 == '{')
            {
                int braceLine = L + 1;
                int depth = 0;
                while (!AtEnd)
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
                blocks.Add(new ModBlock { Name = name, Start = startLine, BraceLine = braceLine, End = endLine });
                i = endLine;
                continue;
            }

            i = startLine;
        }
        return blocks;
    }

    // ============================ 項目単位パース ============================

    /// lines 全体をトークナイズしたうえで、0-based [from, to) の範囲を項目へ分解する。
    /// flatten=true のとき、入れ子 `mod X { ... }` は展開して中身を親の項目列へ平坦化する
    /// （テーマ分割で入れ子モジュールがファイル root へ持ち上がるケースを同値に扱うため）。
    public static List<Item> ParseItems(string[] lines, int from, int to, bool flatten)
    {
        RustItemScan self = new RustItemScan();
        self.code = Tokenize(lines);
        List<Item> res = new List<Item>();
        if (from < 0) from = 0;
        if (to > lines.Length) to = lines.Length;
        self.ParseRange(lines, from, to, flatten, res);
        return res;
    }

    void ParseRange(string[] lines, int from, int to, bool flatten, List<Item> res)
    {
        int i = from;
        while (i < to)
        {
            while (i < to && lines[i].Trim().Length == 0) i++;
            if (i >= to) break;

            int start = i;

            // コード実体を持つ最初の行（純コメント行はここでスキップされ、先行コメントとして項目に付属する）
            int h = i;
            while (h < to && code[h].Trim().Length == 0) h++;
            if (h >= to)
            {
                res.Add(new Item { Start = start + 1, End = to, Kind = "comment", Name = "", IsTest = false });
                break;
            }

            L = h; C = 0;
            SkipWs();
            SkipAttrs();
            SkipWs();
            int kwLine = L;

            string kw = ReadWord();
            if (kw == "pub")
            {
                SkipWs();
                if (Peek() == '(') SkipBalanced('(', ')');
                SkipWs();
                kw = ReadWord();
            }
            while (kw == "unsafe" || kw == "async" || kw == "default") { SkipWs(); kw = ReadWord(); }
            if (kw == "extern")
            {
                SkipWs();
                if (Peek() == '"') { Adv(); while (!AtEnd && Peek() != '"') Adv(); if (!AtEnd) Adv(); SkipWs(); }
                kw = ReadWord();
            }
            if (kw == "const")
            {
                int sl = L, sc = C;
                SkipWs();
                string nx = ReadWord();
                if (nx == "fn") kw = "fn"; else { L = sl; C = sc; }
            }

            string kind = "other";
            string name = "";
            int end;

            if (kw == "use")
            {
                kind = "use";
                end = ScanToSemicolon(to);
            }
            else if (kw == "mod")
            {
                SkipWs();
                name = ReadWord();
                SkipWs();
                char c2 = Peek();
                if (c2 == ';')
                {
                    kind = "mod_decl";
                    end = L + 1;
                }
                else if (c2 == '{')
                {
                    int braceLine = L;              // 0-based
                    int depth = 0;
                    while (!AtEnd)
                    {
                        char ch = Peek();
                        if (ch == '{') depth++;
                        else if (ch == '}') { depth--; if (depth == 0) break; }
                        Adv();
                    }
                    int closeLine = L;              // 0-based
                    if (flatten)
                    {
                        ParseRange(lines, braceLine + 1, closeLine, flatten, res);
                        i = closeLine + 1;
                        if (i <= start) i = start + 1;
                        continue;
                    }
                    kind = "mod";
                    end = closeLine + 1;
                }
                else
                {
                    end = ScanItemEnd(to);
                }
            }
            else
            {
                if (kw == "fn") { kind = "fn"; SkipWs(); name = ReadWord(); }
                end = ScanItemEnd(to);
            }

            if (end <= start) end = start + 1;
            if (end > to) end = to;

            bool isTest = false;
            if (kind == "fn")
            {
                int last = Math.Min(kwLine, end - 1);
                for (int q = start; q <= last; q++)
                {
                    if (TestAttrRe.IsMatch(lines[q])) { isTest = true; break; }
                }
            }

            res.Add(new Item { Start = start + 1, End = end, Kind = kind, Name = name, IsTest = isTest });
            i = end;
        }
    }

    int ScanItemEnd(int to)
    {
        int pd = 0, bd = 0, cd = 0;
        bool sawBrace = false;
        while (!AtEnd && L < to)
        {
            char c = Peek();
            if (c == '(') pd++;
            else if (c == ')') pd--;
            else if (c == '[') bd++;
            else if (c == ']') bd--;
            else if (c == '{') { cd++; sawBrace = true; }
            else if (c == '}') { cd--; if (cd <= 0 && sawBrace) return L + 1; }
            else if (c == ';' && pd <= 0 && bd <= 0 && cd <= 0) return L + 1;
            Adv();
        }
        return Math.Min(L + 1, to);
    }

    int ScanToSemicolon(int to)
    {
        int pd = 0, bd = 0, cd = 0;
        while (!AtEnd && L < to)
        {
            char c = Peek();
            if (c == '(') pd++;
            else if (c == ')') pd--;
            else if (c == '[') bd++;
            else if (c == ']') bd--;
            else if (c == '{') cd++;
            else if (c == '}') cd--;
            else if (c == ';' && pd <= 0 && bd <= 0 && cd <= 0) return L + 1;
            Adv();
        }
        return Math.Min(L + 1, to);
    }
}
'@ -Language CSharp
}

# ------------------------------------------------------------------------------
# PowerShell 側ヘルパ
# ------------------------------------------------------------------------------

# ファイルを行配列として読む（BOM 有無・CRLF/LF を問わない）
function Read-RustLines {
    param([Parameter(Mandatory)][string]$Path)
    return [System.IO.File]::ReadAllLines($Path)
}

<#
 行の正規化（Requirement 2.4 が許容する差分だけを吸収する）:
   1. 行頭空白の除去（lstrip）— `mod tests { … }` を module root へ出したときの一律 de-indent
   2. 行末空白の除去（`git diff -w` 相当）
   3. 行頭の可視性修飾（`pub` / `pub(crate)` / `pub(super)` / `pub(in …)`）の除去
      — テーマ分割で共有ヘルパへ付与される可視性は 2.4 の許容範囲
 アサーション・入力値・期待値・属性・コメントには一切触れない。
#>
function ConvertTo-NormalizedLine {
    param([Parameter(Mandatory)][AllowEmptyString()][string]$Line)
    $t = $Line.Trim()
    return [regex]::Replace($t, '^pub(\s*\([^)]*\))?\s+', '')
}

# 1-based inclusive 行範囲を正規化テキストへ（前後の空行は落とす／内部の空行は保つ）
function Get-NormalizedBlockText {
    param(
        [Parameter(Mandatory)][AllowEmptyString()][string[]]$Lines,
        [Parameter(Mandatory)][int]$Start,
        [Parameter(Mandatory)][int]$End
    )
    $buf = New-Object System.Collections.Generic.List[string]
    for ($k = $Start - 1; $k -le $End - 1; $k++) {
        if ($k -lt 0 -or $k -ge $Lines.Length) { continue }
        $buf.Add((ConvertTo-NormalizedLine -Line $Lines[$k]))
    }
    while ($buf.Count -gt 0 -and $buf[$buf.Count - 1] -eq '') { $buf.RemoveAt($buf.Count - 1) }
    while ($buf.Count -gt 0 -and $buf[0] -eq '') { $buf.RemoveAt(0) }
    return ($buf -join "`n")
}

<#
 与えられた行配列の [From, To)（0-based）を項目単位で正規化し、比較用の 2 つの器へ振り分ける。
   TestBodies : ハッシュテーブル  識別子 -> 正規化本文のリスト（同名複数を許す多重集合）
   Helpers    : 正規化本文のリスト（多重集合として比較する）
 `use` 項目とモジュール接続宣言（`mod X;`）は Requirement 2.4 が許容する機械的調整のため比較対象から外す。
#>
function Get-ItemDigest {
    param(
        [Parameter(Mandatory)][AllowEmptyString()][string[]]$Lines,
        [int]$From = 0,
        [int]$To = -1,
        [string]$Origin = ''
    )
    if ($To -lt 0) { $To = $Lines.Length }

    $items = [RustItemScan]::ParseItems($Lines, $From, $To, $true)

    $testBodies = New-Object 'System.Collections.Generic.Dictionary[string, System.Collections.Generic.List[object]]'
    $helpers = New-Object System.Collections.Generic.List[object]
    $dropped = 0

    foreach ($it in $items) {
        if ($it.Kind -eq 'use' -or $it.Kind -eq 'mod_decl') { $dropped++; continue }
        $text = Get-NormalizedBlockText -Lines $Lines -Start $it.Start -End $it.End
        if ($text -eq '') { continue }
        $rec = [pscustomobject]@{
            Text   = $text
            Origin = $Origin
            Start  = $it.Start
            End    = $it.End
            Name   = $it.Name
        }
        if ($it.Kind -eq 'fn' -and $it.IsTest) {
            if (-not $testBodies.ContainsKey($it.Name)) {
                $testBodies[$it.Name] = New-Object System.Collections.Generic.List[object]
            }
            $testBodies[$it.Name].Add($rec)
        }
        else {
            $helpers.Add($rec)
        }
    }

    return [pscustomobject]@{
        TestBodies = $testBodies
        Helpers    = $helpers
        Dropped    = $dropped
        ItemCount  = $items.Count
    }
}
