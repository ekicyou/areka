# Brief: session-module-gather

> 2026-10-28 `/kiro-discovery` で起票。開発者「この際、areka crateのコードベースが大きすぎる問題をサルベージすべきかもしれない」→ 測った後に 6 本で「一括承認」。6 本のうちの 5 本目。分け方と共通の約束は roadmap の「テーマ別の決めごと」の「本体の crate 分け」が正本。数は main `d6054e4d` の実測（非空行）＝着手のときに測り直す。

## Problem

起動・終了・切替の層（ゴーストを降ろして起こし直す単位、起動するゴーストとバルーンの判断、構成の入力、終わり方、切替と読み直し）は、本体で一番大きく残る塊になる。今は `src` の直下にばらばらに置かれ、`main.rs` の中にもその一部がある。crate に出す前に、本体の中で 1 つのモジュールに集める。

参照の向きを逆にする手当て（約 30 件）と crate を作ることは、次の `session-crate-split` で行う。一緒にすると 20 タスクを超える見込みなので、2 本に分けた。

## Current State

- **集める候補**（本番 / テスト）:

  | ファイル | 本番 | テスト |
  |---|---|---|
  | `ghost_session.rs` | 867 | 3,259（ほかに `shell_balloon_switch_session_*` 2,065） |
  | `ghost_session_runtime.rs` | 176 | — |
  | `ghost_session_scope_rect.rs` | 252 | — |
  | `boot_resolve.rs` | 707 | 1,827 |
  | `boot_config.rs` | 623 | 1,281 |
  | `app_exit.rs` | 307 | 546 |
  | `session_end.rs` | 176 | 1,329 |
  | `exit_wait.rs` | 180 | 197 |
  | `write_failure.rs` | 224 | 785 |
  | `readonly.rs` | 48 | 1,041 |
  | `alert.rs` | 365 | 487 |
  | `log_history.rs` | 495 | 1,078 |

  - `runtime-module-untangle` で `src/session/` へ移した切替の 9 ファイル（`ghost_switch`・`reload`・`reload_args`・`shell_balloon_switch`・`shell_balloon_resolve`・`runtime_boot`・`switch_copy`・`same_folder_pin`・`shell_carry`）は本番 約 3,140 行。
  - **main.rs の項目**: 531〜910 行の約 367 行を外へ出せる。
    - `boot_monitor_snapshot`・`restore_merged_placements`・`on_boot_ok`（`insert_persist_wiring`・`record_last_used_at_boot` を含む）・`record_last_used`・`MarkInputs`・`Teardown`・`settle_session_mark`（`session_mark_verdict`・`MarkVerdict` を含む）。
    - どれも main の中ではほかに使われていない。
    - `ConfigInputs`・`ghost_boot_options`・`is_benign_boot_error`・`default_app_profile_dir` は `boot_config` の項目を main が `pub(crate) use` で出し直しているだけ。
    - main に残るのは `fn main`・`finish_after_run`・`install_boot_context`・`boot_first_ghost`（約 470 行）。
  - **roots の芯**（`roots/mod.rs`・`resolve.rs`・`report.rs`・`user_root.rs`・計 660 行）: 依存は `readonly` と `exe_location` だけで、どちらも集める側にある。`edit.rs`・`desk.rs`・`pick.rs` は install に依存するので本体に残る。
  - **readme**（`readme.rs` と `readme/`・約 980 行）: 外への依存は集める側と runtime だけ。
  - **cli の型 2 つ**: `cli::BootRequest`（`PathBuf`・`String`・`bool` だけ）と `cli::ModulePolicy`（列挙型）。`boot_config` と `ghost_session` が使う。
  - **系の登録**: `ghost_session.rs` の `register_systems`（72〜84 行）が、本体だけのモジュールの系を 10 本登録する。
    - 対象は readme・input_events 5・menu・install・update・mcp。
    - 呼び手は `main.rs:253` だけ。
- **集めないもの**:
  - 本体に残す。
    - `png_image`・`thread_roles`・`tick_gate_config`・`perf_thread_report`（計 657 行・集める側が使わない）。
    - `smoke_exit`（本番の経路・呼び手は main だけ）。
  - 診断用のデモ `shiori_host`・`shiori_session`・`shiori_demo`・`reference_brain`（1,082 行）。`AREKA_SHIORI_DEMO` を付けたときだけ動き、集める側は使わない。本体に残し、消せるかどうかは `audit-areka-app` が決める。
- **名前で照合している所**: ログの出所の直書き（`"areka::boot_config"` 7・`"areka::ghost_session"` 4・`"areka::alert"` 4・`"areka::readme"` 3・`"areka::boot_resolve"` 3・`"areka::app_exit"` 2・`"areka::readme::opener"` 1 ほか）。
  - `crates/real-run-kit` の `launch.rs` が `,areka::app_exit=info` を足す決まりを持つ（`doc/testing.md:48`）。
  - モジュールを `session/` の下へ移すと、出所は `areka::session::…` に変わる。

## Desired Outcome

- 上の候補が本体の `crates/areka/src/session/` に集まり、テストも一緒に移っている。
- `main.rs` は `fn main` と最上位の段取りだけ（約 470 行）。
- 本体だけのモジュールの系の登録は、`main.rs` から呼ばれている。
- `session/` の外への参照の一覧（次の spec が向きを逆にする約 30 件）が、設計か研究の記録に残っている。
- 振る舞いは何も変わらない（「検証」）。

## Approach

- `git mv` で履歴を残して移す。中身は書き換えず、パスだけを直す。
- `register_systems` の本体向けの 10 本の呼び出しを、`main.rs` の呼び手へ移す。登録の順は変えない。
- cli の型 2 つは `session/` の中へ下ろし、cli がそれを引く向きにする。
- ログの出所の直書き・`RUST_LOG` の文字列・real-run-kit の名指しの決まり・`log_history` の決まりを、新しい出所に合わせる。直書きの文字列で、モジュールのパスを写したもの（`"areka::boot_config"` など）は、新しい道に合わせる。語彙として付けたもの（`"areka::persist::*"`・`"areka::log::*"`・`"areka::perf"` など）は変えない。どちらに当たるかは、要件の段で 1 件ずつ決める。

## Scope

- **In**: 候補のファイルとテストの移動・main.rs の項目の移動・roots の芯と readme の移動・cli の型 2 つ・系の登録の移し替え・ログの出所の名前で読む所・文書のパス・`structure.md`・`audit-areka-app`・`audit-areka-install-update` の brief の「受け持つファイル」の書き換え。
- **Out**: 参照の向きを逆にすること・crate の新設（`session-crate-split`）・ゴーストの単位を複数にすること（`multi-ghost-runtime`）・デモの 4 本を消すこと（`audit-areka-app`）。

## Boundary Candidates

- src 直下のファイルの移動。
- main.rs の項目の移動。
- roots の芯・readme・cli の型。
- 系の登録。
- ログと文書。
- 検証。

## Out of Boundary

- 振る舞いの変更。登録の順と起動の段取りの順は変えない。

## Upstream / Downstream

- **Upstream**: `input-crate-split`（`ghost_session.rs` の input の参照が `areka_input::` になってから集める＝同じファイルに触る直列）。
- **Downstream**: `session-crate-split`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - `audit-areka-app`・`audit-areka-install-update`: 受け持つファイル。
  - `ghost_session`・`app_exit`・`boot_*`・`roots/` に触ると書いている spec（`multi-ghost-runtime`・`shiori4-api`・`resident-ghost-rotation`・`tasktray-icon`・`startup-install-option`・`install-live-target-hazards`・`placement-measure-bake-once`・`dev-folder-alias`・`test-roots-under-target`・`sylphya-sqlite-kvs` ほか）: roadmap の新旧のパスの対応表で読み替える。
  - `multi-ghost-runtime`: 集める側とファイルがほぼ全部重なる（`GhostSlot`・止まる流れ・印）。同じウェーブに置かない。

## Constraints

- **検証（要件に入れる）**: roadmap の「本体の crate 分け」の「共通の約束」の検証の項を、そのまま要件に入れる。この spec で回す実機テストは、出所を名前で読む 2 つを 1 回ずつ。
  - real-run-kit を使う実機テストのうち `areka::app_exit` を名指しするもの。
  - `tests/smoke_boot_loop_exit.rs`。

  MCP の `get_log` の 5 種別の分け方が前後で同じことも示す。
- **共通の約束**: roadmap の同じ節のとおり。
- **Fable**: Opus で足りる。
