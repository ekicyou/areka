# workflow の確かめの記録

道具（`tools/release-notes.ps1`）の手元の走り、workflow の静的な確かめ、段の本文の手元の走り、乾いた走りの結果を、1 項目 1 行で残す。後ろのタスクは、自分の節を末尾に足す。

## 詳しいリリースノートの生成の手元の走り（タスク 2.2）

- 回した日: 2026-10-11。枝 `claude/kiro-spec-release-notes-95477d` の先頭 `a8a30331`（作業木にタスク 2.2 の変更を置いた状態）。コマンドは作業木の根で回した。書き出しの先は、どれも作業木の `target\release-notes-task22\` の下。
- 自分の検査: `pwsh -NoProfile -NonInteractive -File tools/release-notes.ps1 -SelfTest` → 終了コード 0（`selftest: all passed`。詳しいリリースノートの判定は 4 つ: 定型文の鍵・見本の題名が 1 つ以上・見本の出力と 1 字違わず同じ・一つ前のタグが空で fix と perf を除いた出力と同じ）。
- 檻が効くこと: 写しの道具で決まりを 1 つずつ崩して自分の検査を回すと、7 つとも赤（終了コード 1）: spec 名の印を外さない／perf を見出しに当てない／PR のリンクを付けない／0 件の見出しを出す／スコープを見ない／最初からの範囲の文を使わない／件数を固定する。崩していない写しは緑。
- 文字コードの検査: `pwsh -NoProfile -NonInteractive -File tools/encoding-check.ps1` → 終了コード 0（`FAIL` の行 0・`PASS tools/release-notes.ps1`）。
- `v0.0.2` から枝の先頭まで: `-Notes -Version 0.0.3 -PrevTag v0.0.2 -To HEAD -Repository ekicyou/areka -Out target\release-notes-task22\notes-0.0.3.md` → 終了コード 0・`- ` で始まる行 41＝`git rev-list --first-parent --count v0.0.2..HEAD` の 41・`areka-P0-` 0 件（PR へのリンクの付いた行 37・付かない行 4）。
- `v0.0.1` から `v0.0.2` まで: `-Notes -Version 0.0.2 -PrevTag v0.0.1 -To v0.0.2 -Repository ekicyou/areka -Out target\release-notes-task22\notes-0.0.2.md` → 終了コード 0・`- ` で始まる行 614＝`git rev-list --first-parent --count v0.0.1..v0.0.2` の 614・`areka-P0-` 0 件（PR へのリンクの付いた行 194・付かない行 420。うち 2 行は題名の末尾が `(PR#番号)` の形で、番号を文のまま載せている）。
- 一つ前のタグが空: `-Notes -Version 0.0.3 -PrevTag '' -Repository ekicyou/areka -Out target\release-notes-task22\notes-from-start.md` → 終了コード 0・`- ` で始まる行 763＝`git rev-list --first-parent --count HEAD` の 763・`areka-P0-` 0 件・範囲の文は「最初から v0.0.3 まで…（763 件）」。
- 端末の文字コード 932 の子: 窓の無い自分だけの端末を持つ子（`Invoke-Utf8Child … -OwnConsole`）が自分の端末を 932 にしてから `v0.0.1`〜`v0.0.2` を出す → 終了コード 0・その端末では素の `git log` の読みが日本語を壊す（較正）・出力は、端末を 65001 にした子の出力と 1 バイトも違わない（どちらも SHA256 `d9f18ec83d172f47c9a7de1e9e2fbe50a0edb420c2256ebf8a2634ec7b601fb5`）・親の端末の文字コードは前後で同じ（932/932）。
- 書き出しの形: `notes-0.0.2.md` は BOM なし・CR 0 個・U+FFFD 0 個・末尾は LF 1 つ。
- 2 回回す: 同じ引数（`v0.0.1`〜`v0.0.2`）でもう 1 回出す → 1 バイトも違わない（SHA256 が上と同じ）。
- 別のフォルダから: `target\release-notes-task22\elsewhere\sub` を今のフォルダにして、道具を絶対パスで呼び、`-Out ..\new-dir\rel.md` → 終了コード 0・無かったフォルダ `elsewhere\new-dir` が作られ、中身は上と 1 バイトも違わない。
- 変更が 0 件の範囲: `-PrevTag v0.0.2 -To v0.0.2` → 終了コード 0・題の行と範囲の文（0 件）だけで、見出しは 0。
- 見えないタグ: `-PrevTag v9.9.9` → `FAIL git log (exit 128): fatal: bad revision 'v9.9.9..HEAD'`・終了コード 1・`-Out` のファイルは作られない。
- `-` で始まる先: `-To '--output=target/x'` → `FAIL git log (exit 128): fatal: bad revision …`・終了コード 1（`git` のオプションとして働かず、ファイルは作られない）。
- 鍵の欠けた定型文（写しの道具で）: `FAIL strings: cannot read tools/release-notes.strings.json, or a key is missing or empty`・終了コード 1・`-Out` のファイルは作られない。
