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
- **ログ**: 出所を名前で読むのは `tests/emo2_mcp_real_run.rs` の `rust_log`（`areka::input_events::user_break=debug`）と `tests/window_link_real_run.rs:49`（`areka::input_events=debug`）の 2 件。`log_history` の決まりに input の行は無い。input のテストはイベントの値と文言で照合しており、出所は見ない。

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
  - `tests/emo2_mcp_real_run.rs`（2026-11-13 に `script_security_real_run.rs` ほか 2 本をまとめた）
  - `tests/window_link_real_run.rs`

  `quit_app` の向きを変えるので、Ctrl+左ダブルクリックの退避で終わる記録の順が前後で同じことも示す。
- **フレームを遅らせない**: 終わりの知らせで退避を 1 フレームでも遅らせない。
- **共通の約束**: roadmap の同じ節のとおり。
- **Fable**: Opus で足りる。

## 2026-11-17 棚卸㊴の再測定（main 7f41b53e）
- **前提の変化**: 待っていた `runtime-crate-split` は 11-17 に着地した。input の本番は今 `areka_runtime::` を 27 か所・`areka_placement::` を 16 か所、crate の公開面で引いている＝Approach の「runtime と placement の側で、input が使う項目を `pub` にする」は済んだ（残るのは input の側の `pub(crate)` → `pub` と、テストの支えの付け方）。
- **大きさの引き直し（非空行）**: `crates/areka/src/input_events/` は 50 ファイル・本番 5,018 行・テスト 9,981 行（起票は 42・3,932・9,319＝`balloon_wheel`・`cursor_answer`・`cursor_pick`・`hover_region` ほかが増えた）。本番で本体を引くのは `mod.rs:34`（`app_exit::{ExitOrigin, quit_app}`）・`file_drop.rs:15〜17`（`ghost_session::GhostSlot`・`install`）・`shortcut.rs:10〜11`（menu）の 3 ファイルだけ＝起票の形のまま。外から input を引く本番は `ghost_session.rs` 11・`ghost_session_runtime.rs` 2・`app_exit.rs` 1・`menu/{mod,trigger}.rs`・`session/runtime_boot.rs` 198 行（`user_break::wire_user_break`）、テストは 20 本。
- **`quit_app` の向き**: Approach の「`runtime-module-untangle` が `frame.rs` に入れた終わりの知らせの仕組み」は古い。終わる相は本体の `session/ghost_quit.rs` へ移った。本体の口を下の crate から呼ぶ前例は `crates/areka-runtime/src/session_hooks.rs` の `SessionHooks`（関数の束を本体の `session/hooks.rs` が据える）。どの形で向きを逆にするかは設計で選ぶ。
- **触るファイル**: 起票のまま（新しい `crates/areka-input/`・`input_events/` の全部・上の外の呼び手・`placement_tests/spawn_assembly_tests.rs`・`tests/emo2_mcp_real_run.rs` 115 行・`tests/window_link_real_run.rs` 49 行・`Cargo.toml`・`Cargo.lock`）に加えて、`session/runtime_boot.rs`・`runtime_boot_tests.rs`（`audit-emo2-boot-spine` の持ち物）と `runtime_tests/` の 8 本（`frame*`・`balloon_visibility*`＝`audit-emo2-boot-frame`・`audit-emo2-boot-switch` の持ち物）。`main.rs` に `use areka_input as input_events;` の出し直しを置けば、外の呼び手のパスは変えずに済む（設計で選ぶ）。
- **規模**: 12〜15 タスクのまま（本番は増えたが、runtime の側の公開は済んだ）。
- **先に要るもの**: `audit-placement-follow-residue`（`input_events/drag_test_support.rs` を先に書き換える＝同じファイルの直列）。
- **席**: crate を足すので `Cargo.lock` の 1 本だけの席を使う（見込みの C23 のとおり）。
- **同じウェーブに置けない相手**: `session-module-gather`（`ghost_session.rs`・`ghost_session_runtime.rs`・`app_exit.rs`・`input_events/mod.rs`・`file_drop.rs`・`file_drop_wiring_tests.rs`・`shortcut_tests.rs`・`tests/emo2_mcp_real_run.rs`）＝見込みの C23 で並べているのは誤り。出し直しを置かないなら `audit-emo2-boot-spine`・`audit-emo2-boot-frame` とも重なる。`collisionex-regions`（`input_events/mod.rs`）・`audit-areka-input-menu` とも重なる。
- `crates/log-capture-kit/src/event.rs` 150 行・`event_tests.rs` 18〜19 行と 121 行・`crates/ukadoc-survey/src/evidence/candidates.rs` 51 行の説明の中の `input_events/` のパスは、見直しの 2 本が先に着地するなら、この spec が直す。
- **優先度の区分**: A（開発者「この際、areka crateのコードベースが大きすぎる問題をサルベージすべき」）。
- **要件定義のモデル**: Opus（crate 分けの前例は `runtime-crate-split`・`placement-crate-split`。向きを逆にする口にも前例 `SessionHooks` がある）。
- **分割の案**: 無し。
