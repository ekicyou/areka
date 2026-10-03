# areka-P0-emo-text-file-split — 検証の記録

## 1. 基準（分割前）

- 基準 SHA: `94e3ad78220141e1ee4ed4d61b6ae94d82dabb3e`（実装の最初のコミットの直前の HEAD・対象 7 本は main `1ce4c74e` と同一）
- 採取日時: 2026-10-03
- `tools/test-all.ps1` の終了コード: **0**（i686 導入・i686 成果物ビルド・fmt --check・x64 ワークスペース全テスト・i686 テストの全段 緑）
- 生ログ: `target\emo-text-file-split\before_*`（コミットしない）

| 証跡 | 行数・件数 |
|---|---:|
| `before_list.txt` | 9,371 行 |
| `before_results.txt` | 9,371 行（FAILED 0・ignored 103） |
| `before_warnings.txt` | `cargo build --workspace` 0 件・全体テストのコンパイル出力 0 件 |

### 採り方（after も同じ手順）

1. `pwsh -NoProfile -File tools/test-all.ps1 *>&1 | Tee-Object target\emo-text-file-split\<phase>_testall.log`
2. 一覧: `cargo test --workspace -- --list` と `cargo test -p shiori-host32-helper -p shiori-host32-ipc --target i686-pc-windows-msvc -- --list` の stdout を結合し、`: test`／`: bench` で終わる行だけを序数（`[StringComparer]::Ordinal`）で整列。重複行は残す。
3. 結果: 1 の生ログから `^test .+ \.\.\. (ok|FAILED|ignored.*)$` の行だけを抜き、序数で整列。
4. 警告: `cargo build --workspace` の出力と 1 の生ログの、`warning` で始まる行の件数。
