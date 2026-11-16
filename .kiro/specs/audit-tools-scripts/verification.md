# 検証の記録: audit-tools-scripts

## タスク 8: 絵の道具 2 本の「コミット済みの出力は最新」の判定

### 比べ直しの結果（判定を足す前）

`tools/app-icon.ps1 -SelfTest` と `tools/cursor-art.ps1 -SelfTest` が `target\` の下（`app-icon-selftest\case1`・`cursor-art-selftest\case1`）に組んだ出力を、コミット済みのものとバイトで比べた（python）。5 ファイルとも同じ。

| コミット済みのファイル | バイト数 | 組んだ結果との比較 | SHA-256（先頭 16 桁） |
|---|---|---|---|
| `crates/areka/icon/app.ico` | 23315 | 同じ | `480111faaab7861f` |
| `crates/areka/icon/tray.ico` | 17773 | 同じ | `29e5a9bde8c3c50e` |
| `crates/areka/icon/icons.res` | 41684 | 同じ | `1403d0a304d0ba4a` |
| `crates/areka/cursor/hand.cur` | 236790 | 同じ | `59a0e6fb8e7cbe39` |
| `crates/areka/cursor/grip.cur` | 236790 | 同じ | `989829219576878e` |

### 判定の追加

両方の `-SelfTest` に `committed outputs are current` を足した（`same bytes twice` で組んだ結果を使い、組む回数は増やしていない）。2 本とも緑（`ok selftest committed outputs are current`・`OK selftest (3 checks)`）。

### 落ちることの確かめ

道具の台本（2 本と `tool-common.ps1`・`utf8-child.ps1`）を `target\tmp-t8\tools\` へ、`icon\`・`cursor\` を `target\tmp-t8\crates\areka\` へ写し、写しの `app.ico`・`grip.cur` の末尾 1 バイトを違えて写しの道具で `-SelfTest` を回した。どちらも落ちた（写しは確かめた後に消した）。

- `FAIL selftest committed outputs are current: app.ico (source changed or rendering changed; run tools/app-icon.ps1 and commit)`
- `FAIL selftest committed outputs are current: grip.cur (source changed or rendering changed; run tools/cursor-art.ps1 and commit)`

`crates/areka/icon/` と `crates/areka/cursor/` は書き替えていない（`git status` は `tools/` の 2 本と本記録だけ）。

## タスク 11.1: `runtime-crate-split` の着地の確認と負荷の再現の道具の選び肢

確認日: 2026-11-17

確認のコマンドと結果（読み取りだけ）:

- `git fetch` のあと `git rev-parse origin/main` は `5b3a361c95c5fb494fdf7845576cf20f1aa20c07`
- `git ls-tree -d origin/main .kiro/specs/completed/runtime-crate-split` は何も返さない（終了コード 0・出力なし）

結論: `runtime-crate-split` はまだ `main` に着地していない。

採った道: 「着地していない」の道。`tools/load-flake.ps1` に `areka-runtime-lib` の選び肢は足さない（コードの変更なし）。`runtime-crate-split` の brief には何も書かない。`-SelfTest` は変更なしで緑のまま。

引き継ぎ: `runtime-crate-split` の完了時に、そのセッションが拾う。渡す文は次のとおり。

> `runtime-crate-split` の着地後、`tools/load-flake.ps1` に `-Binary areka-runtime-lib`（`-p areka-runtime --lib`）の選び肢を足してください。あわせて使い方の例の `-Filter runtime::frame` を新しいクレートの形に直し、`-SelfTest` の `cargo args` の期待を 3 対象から 4 対象へ直してください（`audit-tools-scripts` 11.1 の申し送り）。
