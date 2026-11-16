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
