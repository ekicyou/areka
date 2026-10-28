# Brief: input-crate-split

> 2026-10-28 `/kiro-discovery` で起票。開発者「この際、areka crateのコードベースが大きすぎる問題をサルベージすべきかもしれない」→ 測った後に 6 本で「一括承認」。6 本のうちの 4 本目。分け方と共通の約束は roadmap の「テーマ別の決めごと」の「本体の crate 分け」が正本。数は main `d6054e4d` の実測（非空行）＝着手のときに測り直す。

## Problem

入力の出来事（ポインタからマウスのイベント・選択肢・ドラッグ・中断・ホイール）は、本体の中で一番まとまった持ち場の 1 つ。それでも本体と一緒に組み直され、テストも本体の 1 つの実行ファイルに入っている。後の session（起動・終了・切替の層）が input を 15 か所で呼んでいるので、input は session より先に出す。

## Current State

- **大きさ**: `crates/areka/src/input_events/` は本番 3,932 行・テスト 9,319 行（42 ファイル）。
- **本番から外への参照**: 28 行（全件測った）。
  - emo2_boot 11: `frame::Emo2Wiring` の `runtime()`・`presenter()` が 6 ファイル、`hit_region`、`user_break_cue::NoUserBreakSignal`、`talk_lifecycle::TalkLifecycleSignal`、`target_map::balloon_target`。
  - placement 11: 窓の印 3 種・`follow::{begin_char_drag, on_char_drag_end}`・`reset_balloon_pos`。
  - menu 2: `shortcut.rs` だけ。
  - install 2: `file_drop.rs` だけ。
  - src 直下 2: `mod.rs:33` の `app_exit::{ExitOrigin, quit_app}`（Ctrl+左ダブルクリックの退避）、`file_drop.rs:15` の `ghost_session::GhostSlot`。
  - mcp・readme・update・roots は 0。
  - menu・install・ghost_session を参照するのは `shortcut.rs`（本番 83 行）と `file_drop.rs`（332 行）だけで、どちらも input の他のファイルから独立している。
- **外から input への参照**（本番）:
  - `ghost_session.rs` 11。`file_drop::attach_file_drop_receivers`・`shortcut::attach_shortcut_receivers` を含む。
  - `ghost_session_runtime.rs` 3・`app_exit.rs` 1（`MouseWiring`）・menu 3（`MouseWiring`・`PendingDoubleClick`・`char_scope`・`click::on_char_pointer_released`）。
  - emo2_boot 3。`runtime-module-untangle` で切る。
- **menu**: input と一緒には動かせない。menu は install・update・roots・session を引く（本番 約 2,000 行・13 ファイル）。input から menu を引くのは `shortcut.rs` の 3 関数だけ。
- **外の crate**（本番の出現数）: tracing 74・wintf 28・bevy_ecs 25・areka-emo-text 9・areka-kanade 8・areka-sakura 5・areka-nar 4（`file_drop` だけ）・areka-emo-compose 2・areka-ghost 1・areka-emo-present 1。windows は 0。
- **テストが外を使う所**（input に移す分）: 17 ファイル。
  - `#[cfg(test)]` だけの外の項目が 5 ファイル・12 か所: `placement::test_support`・`emily_shaped_scopes`・`spine::GpuPermit`。
  - `balloon_test_support` は、runtime の本番の組み立て（`RuntimeWiring::new`・`BootAssets`・`PositionCue`・`TalkClock`・`ZOrderDirective` など 7 モジュール）を使う。
  - 外のテストで input を参照するのは 19 ファイル。その 1 本が `placement-crate-split` で本体に残す `spawn_assembly_tests.rs`（`attach_char_pointer_handlers`）。
- **ログ**: 出所を名前で読むのは `tests/script_security_real_run.rs:64`（`areka::input_events::user_break=debug`）と `tests/window_link_real_run.rs:49`（`areka::input_events=debug`）の 2 件。`log_history` の決まりに input の行は無い。input のテストはイベントの値と文言で照合しており、出所は見ない。

## Desired Outcome

- `crates/areka-input/`（lib・`publish = false`）が、input（`shortcut.rs`・`file_drop.rs` を除く）の本番とテストを持つ。`areka-runtime`・`areka-placement` に依存し、本体と session からは依存される。
- `shortcut.rs` は本体の menu（例 `menu/shortcut.rs`）へ、`file_drop.rs` は本体に残り、input の crate は menu・install・session を参照しない。
- 振る舞いは何も変わらない（「検証」）。

## Approach

- 新しい crate を作り、`git mv` で履歴を残して移す。input の側で `pub(crate)` から `pub` にするのは約 17 項目と 3 メソッドの見込み。runtime と placement の側で、`RuntimeWiring::runtime`・`presenter`・`NoUserBreakSignal`・`TalkLifecycleSignal`・`balloon_target`・窓の印など、input が使う項目を `pub` にする。
- **`quit_app` の向き**: input から session を引くと循環になる。`runtime-module-untangle` が `frame.rs` に入れた終わりの知らせの仕組みを使い回して、向きを逆にする。新しい仕組みは作らない。
- **境をまたぐテスト**: `spawn_assembly_tests.rs` は placement と input を使うので、`areka-input` のテストへ移す。
- **テストの支え**: 下の crate の `test-support` 機能を dev-dependency で付ける。写しは作らない。
- **ログの出所**: `areka::input_events` を名指しする 2 件を、新しい出所に直す。

## Scope

- **In**: crate の新設と移動・`shortcut.rs` と `file_drop.rs` の置き場・公開面（input・runtime・placement）・`quit_app` の向き・テストと支えの移し替え・ログの出所・文書のパス・`structure.md`・`audit-areka-input-menu` の brief の「受け持つファイル」の書き換え。
- **Out**: menu の crate 化（install・update・roots・session を引くので本体に残す）・入力の振る舞いの直し（`audit-areka-input-menu` へ申し送る）・session（後の 2 本）。

## Boundary Candidates

- `shortcut.rs`・`file_drop.rs` の移し替え（本体の中・先に済ませる）。
- crate の骨組みと移動・公開面。
- `quit_app` の向き。
- テスト・支え・ログ・文書。
- 検証。

## Out of Boundary

- 振る舞いの変更。

## Upstream / Downstream

- **Upstream**: `runtime-crate-split`（input が runtime を crate として使う）。
- **Downstream**: `session-module-gather`。`audit-areka-input-menu` はこの spec の着地を待つ。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - `audit-areka-input-menu`: 受け持つファイル。
  - input_events に触ると書いている約 30 本（`mouse-hover-event`・`sstp-receive`・`shell-tooltip`・`inputbox-user-input`・`balloon-link-hover` ほか）: roadmap の新旧のパスの対応表で読み替える。
  - `multi-ghost-runtime`: `MouseWiring` などの NonSend にゴーストの識別子を持たせる。同じウェーブに置かない。

## Constraints

- **検証（要件に入れる）**: roadmap の「本体の crate 分け」の「共通の約束」の検証の項を、そのまま要件に入れる。この spec で回す実機テストは、出所を名前で読む次の 2 つを 1 回ずつ。
  - `tests/script_security_real_run.rs`
  - `tests/window_link_real_run.rs`

  `quit_app` の向きを変えるので、Ctrl+左ダブルクリックの退避で終わる記録の順が前後で同じことも示す。
- **フレームを遅らせない**: 終わりの知らせで退避を 1 フレームでも遅らせない。
- **共通の約束**: roadmap の同じ節のとおり。
- **Fable**: Opus で足りる。
