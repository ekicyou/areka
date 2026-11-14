# 検証の記録（audit-placement-follow）

## 5.1 同じ範囲への audit のかけ直し

- 範囲: 受け持ちの 56 ファイル＋この spec で作った 2 本（`crates/areka-placement/src/follow_visibility_balloon_words_tests.rs`・`crates/areka/src/placement_tests/transition_judge_offset_negative_tests.rs`）＝58 ファイル。組は 3 つ（組 A＝本番 12・組 B＝追従のテスト 30・組 C＝判定器 16）。
- やり方: 回ごとに 3 組へ別々の読み係を当て、件数の上限なし・全部の行を読む作法で `/ponytail:ponytail-audit` の見方をかけた。指摘は全部直し、直しはタスク単位のレビュー（kiro-review）を通してからコミットした。
- 数から外したもの: 裁定で残した指摘 21（組の相手のずれ）・`GroupShift` の `#[allow]`（議題 5・要件 2.5）・28 の `MonitorDpiEntry`／`anchor_changed_system`・議題 11（接続宣言の `#[cfg(test)]` 12 個）・`MonitorSnapshot`・設計の議題 6 案 乙で残した「directive で濾した出力を集める部品の写し」（3 回目の組 B が挙げた 1 件）。
- 要件 4 の議題に当たる指摘: 0 件（2 回目の組 A が「種類 3 の疑い」とした「ドラッグ中の窓へ面の `move` のずれを当てる」は、ukadoc の `move` の項 https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#move の原文「ベースレイヤの表示位置を、元の表示位置から指定座標分ずらす。」がドラッグとの関わりを書かないので、要件 4.2 の確かめのうえ指摘 5 と同じ読みで種類 3 に当たらないとし、直した）。

### 回ごとの組ごとの件数

| 回 | 組 A | 組 B | 組 C | 計 | 必ず直す | 本番の振る舞いに効く | 直しのコミット |
|---|---|---|---|---|---|---|---|
| 1 | 5（＋組 B が挙げた組 A の 1） | 13 | 8 | 27 | 0 | 1（バルーンの窓ができる前のずれの記録） | `3216e244b`・`f08f466cc` |
| 2 | 5 | 7 | 8 | 20 | 0 | 2（位置の印・ドラッグ中の窓へのずれ） | `fbfa8a3e4`・`0f51f5dd1` |
| 3 | 2 | 6（議題 6 の 1 件を除く） | 4 | 12 | 0 | 2（ドラッグの控えだけが残る隙間・バルーンのドラッグ中のキャラ窓） | `8b611df78`・`01c567f34` |
| 4 | 3（組 B が挙げた 1 を含む） | 8 | 3 | 14 | 0 | 0 | `229187959`・`f8d57e793` |
| 5 | 4 | 5 | 5 | 14 | 0 | 0（届かない経路の 2 件のみ） | `7d7d5be9b`・`6d0dc0030` |

- 指摘の中身は回ごとに、範囲の中の説明の古さ・テストの弱さ・直しの取り残し（前の回の直しが同じ形の別の所に届いていない）へ移っていった。

### 判定

**5 回目で打ち切った・残りは直し済み**（要件 5.3）。5 回目の指摘 14 件に「必ず直す」と本番の振る舞いに関わるもの（届く経路）が 0 件だったので、5 回目の指摘を全部直したところでかけ直しを打ち切った。打ち切ることは 2026-11-14 に、5 回目の結果が出た時点で開発者へチャットで一言知らせた。

### 実機のログでの判定の確かめ（判定器を厳しくした回）

- 1 回目の組 C の直し（読めない `diff`・`total_us` を `MalformedRecords` に数える）の後、排他テストの机を取って実機の判定器を 1 回ずつ回した。
  - `transition_judge_offset_signoff`（`.kiro/specs/completed/areka-P0-balloon-offset-dpi/signoff-2026-08-28.log`）: 緑。遷移 15 本・offset 行 14 件・違反 0 件。
  - `transition_signoff`（`areka-P0-dpi-transition-atomicity` の最後のサインオフのログ `atom-73-signoff-1\atom-signoff.log` をワークツリーの `target\transition-signoff\` へ写して与えた）: 8 遷移すべて決定論の側は PASS・`malformed=0`。実機専用の側の違反は当時と同じ時間の上限の 40 件だけで、新しい赤は出ていない。
- 2 回目の組 C の直し（番兵 `diff=-` を `Unmeasured(GroundDiff)` にする）の前に、上の実機ログ 2 本の `kind=ground` の行（17 行・15 行）に `diff=-` が 0 行であることを数えて確かめた。
- 3.9 で、判定器の数え方を scope ごとにした後の `transition_judge_offset_signoff` も緑（違反 0 件）。

### 確定した直す対象 28 件との突き合わせ（HEAD `7d7d5be9b`）

直った 27・裁定で残した 1（21）・直っていない 0。8・23・28 は直したうえで、裁定で残した部分（議題 4・8・11）または後へ回した部分（テストだけが使う公開は `runtime-crate-split` へ申し送り）を含む。`MonitorSnapshot` は残したまま（`crates/areka-placement/src/follow/work_area.rs` の `pub struct MonitorSnapshot`）。

### 範囲の外へ伝えたもの

- 核（`audit-placement-core`）: `research.md` の「核へ伝える一覧（実装の段）」を 2026-11-14 に相手のセッションへ送った。
- 本体（`runtime-crate-split`）: 同 spec の `brief.md` の「2026-11-14 `audit-placement-follow` の申し送り」の節。5.1 で、毎フレームの相の注（`crates/areka/src/runtime/frame/move_shift.rs`）が見送り（`debug!`）を書いていない 1 行を足した。

## 5.2 全体テストと約束の静的な確かめ

- 全体テスト: `pwsh -NoProfile -File tools/test-all.ps1`（排他テストの机を取って 1 回）を `fd3cd9d14` で回し、**all steps green**。sample fetch・i686 の組み立て・`fmt --check`・x64 workspace tests（866 s）・dola toml・i686 tests・crates.io の事前確認・文字コード・release notes・roadmap・i686 helper の同期・areka.exe の隣の helper の 14 段がすべて exit 0。
- `git diff 30a343eb -- Cargo.lock` は空。`crates/wintf/`・`vendors/`・`crates/areka-placement/Cargo.toml` の差分も 0。
- 公開の形の変化は、`pub use self::work_area_clamp::{..}` に足した `ChainPlacement`・`ChainClampOutcome` の 2 つ、`pub(crate)` へ狭めた `VisibilityVerdict`（と `position`）・`guard_visibility` の 2 つ、消した `DragPositionPolicy`・`BottomSnapPolicy` の 2 つだけ（`git diff 30a343eb -- crates/areka-placement/src` の `pub` の行で確かめた。ほかは `pub(crate)`／`#[cfg(test)]` の中のテストの部品）。
- 終了の道（`app_exit` ほか）に触る直しは無い（本番で変えたのは `crates/areka-placement/src` の `follow*`・`transition_diag.rs` と、テスト専用の判定器 `crates/areka/src/placement_tests/transition_judge*.rs` だけ）。
- 記録の無い失敗の道は足していない: 直しで足した早期の戻りは、どれも `warn!`（保存しなかった・位置が未確定・窓が無い）か、失敗でない見送りの `debug!`（ドラッグ中・組の相手・札）を出す。タスクごとのレビューと 5 回の audit で確かめた。

## 最終の確かめ（`/kiro-validate-impl`）

- 2026-11-14・排他テストの机を取って実施。DECISION: **GO**（OWNERSHIP: LOCAL・要件 51/51 の受け入れ条件に当たりあり・設計からのずれ無し・依存の新しい矢印は設計が許す `MoveShift → DpiSync` だけ・範囲の外の変更は設計の「Modified Files（範囲の外）」の 3 ファイルだけ・止まったタスク無し）。
- 直さなくてよい小さな点（後で `audit-placement-core` の後にまとめてよい）: `move_shift.rs` の `is_placed` と `drag_follow.rs` の `known_position` が同じ `CW_USEDEFAULT` の判定を別々に持つ。`drag_follow.rs` はちょうど 1,000 行で、この先 1 行足すなら設計の逃げ道どおり守りの関数を `visibility.rs` へ移す。
