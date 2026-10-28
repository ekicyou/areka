# Brief: session-crate-split

> 2026-10-28 `/kiro-discovery` で起票。開発者「この際、areka crateのコードベースが大きすぎる問題をサルベージすべきかもしれない」→ 測った後に 6 本で「一括承認」。6 本のうちの 6 本目（最後）。分け方と共通の約束は roadmap の「テーマ別の決めごと」の「本体の crate 分け」が正本。数は main `d6054e4d` の実測（非空行）＝着手のときに測り直す。

## Problem

`session-module-gather` で、起動・終了・切替の層は本体の `src/session/` に集まる。それでも、集めた層は本体だけのモジュール（input・menu・install・update・mcp・cli）を呼んでいる。たとえば窓を開いた後に入力やメニューの受け手を差し込む所や、切替の節目に install と update の机へ知らせる所がそうで、crate に出せない。この spec で呼ぶ向きを逆にし、`areka-session` へ移す。これで本体は「UI スレッドからエンジンへつなぐ」部分だけになる。

## Current State

- **集めた層の大きさ**（見込み）: 本番 約 8,600 行・テスト 約 25,000 行。
- **外への参照を 0 にする手当て**（全件測った・どうしても切れないもの 0 件）。
  - **項目を移すだけで済む約 40 件**と**型を下ろす 2 種**は、`session-module-gather` で済む。
  - **向きを逆にする約 30 件**（仕組み 6 つ）がこの spec の中心になる。

  | 仕組み | 件数 | 中身 |
  |---|---|---|
  | C1 窓を開いた直後の差し込み | 5 | input 4（`attach_char_pointer_handlers`・`attach_balloon_pointer_handlers`・`attach_file_drop_receivers`・`attach_shortcut_receivers`）・menu 1（`attach_release_handlers`） |
  | C2 実行系が立った後の結線 | 13 | menu 9（`wire_menu` と各枠の `register` 8 本）・input 3（`wire_mouse_input`・`wire_choice_drain` ほか）・`install::names::reseed` 1 |
  | C3 切替の節目の知らせ | 5 | `ghost_switch` から install・update の机（`on_steady`・`run_overwrite_between`・`on_switch_end`） |
  | C4 机が忙しいかの問い | 3 | `reload`・`shell_balloon_switch` から `install::desk::is_busy`・`update::desk::is_busy` |
  | C5 受け口 | 3〜5 | `runtime_boot` が作る install・update の `raw_sender` と `InstallCueSink`・`UpdateCueSink` |
  | C6 OS が窓を閉じる求め | 1 | `app_exit.rs` の `MouseWiring` |

  - C1・C2・C5 は、本体が渡す trait 1 つ（メソッド 3 つ）にまとめられる見込み（推測）。
  - C6 は、`GhostSlot` の実行系の `kanade()` から送れば消せる見込み（推測）。
- **テスト**: 集めた層のテストのうち 35 ファイルが、本体だけのモジュールを参照している（roots の残り 47 件・menu 11・cli 10・input 7・install 7・update 7）。
- **外から集めた層への参照**（本体から session への向き・許される）: main.rs 約 68・install 15・update 11・menu 11・mcp 10・roots 7・readme（集めた後は中）・offline_dump 4・input 2・cli 1。
- **`multi-ghost-runtime`**: brief が作り替える中心（`GhostSlot` を識別子から `GhostSession` への集まりにする・止まる流れを「このゴーストが止まった」と「アプリを終える」に分ける・App の範囲の記憶を 1 か所に集める）は、ほぼ全部この層の中にある。

## Desired Outcome

- `crates/areka-session/`（lib・`publish = false`）が集めた層の本番とテストを持つ。`areka-input`・`areka-runtime`・`areka-placement` に依存し、本体からは依存される。
- `areka-session` は本体を参照しない。本体だけのモジュールへの呼び出しは、本体が渡す口（trait や知らせ）を通る。
- 本体（`crates/areka`）の本番は約 15,000 行になる。残るのは main と起動オプションのほかは、MCP・メニュー・インストール・更新・投げ込み・オフラインの面の書き出し・SHIORI のデモだけで、どれも UI スレッドからエンジンへつなぐ部分。
- 振る舞いは何も変わらない（「検証」）。

## Approach

- 向きの逆転を先に本体の中で済ませ、`session/` の外への参照が 0 になったのを grep で示してから、crate に移す（`runtime-module-untangle` → `runtime-crate-split` と同じ順を、1 本の中で行う）。
- 仕組みは要件の段で決める。推奨は次のとおり。
  - C1・C2・C5 は本体が実装する 1 つの trait（窓を開いた直後・実行系が立った後・受け口を作る）。
  - C3 は切替の節目の知らせ。install と update が受け手になる。
  - C4 は install と update が立てる「忙しい」の資源を session が読む形。
  - C6 は kanade へ直に送る形。
- **呼ぶ順は変えない**。登録・差し込み・結線の順は、今の順と同じであることをテストで示す。
- 本体だけのモジュールを参照するテスト 35 ファイルは、2 つに分ける。逆転した口の偽物で session の中で確かめるテストと、本体に残して本物どうしをつなぐテストにする。
- **crate の名前**: `areka-session`。`session_end.rs` は OS のログオンのセッションの終わりの受け手で、ゴーストのセッションとは別の意味。紛らわしければ要件の段で `os_session_end` などへ改める。

## Scope

- **In**: 向きの逆転 6 つ・crate の新設と移動・公開面・テストの分け方と移し替え・`test-support` 機能・ログの出所（`areka::session::…` を `areka_session::…` へ。real-run-kit の名指しの決まり・`log_history` の決まり・`RUST_LOG` の文字列・手順書）・文書のパス・`structure.md`・`audit-areka-app`・`audit-areka-install-update` の brief の「受け持つファイル」の書き換え。
- **Out**: ゴーストの単位を複数にすること（`multi-ghost-runtime`）・menu・install・update・mcp・roots の残りの crate 化（どれも本体がつなぐ役そのもの＝本体に残す）・中身の直し（audit へ申し送る）。

## Boundary Candidates

- 向きの逆転（仕組みごと）。
- crate の骨組みと移動・公開面。
- テストの分け方。
- ログと文書。
- 検証。

## Out of Boundary

- 振る舞いの変更。起動の段取り・切替の段取り・終わり方の順は変えない。

## Upstream / Downstream

- **Upstream**: `session-module-gather`。
- **Downstream**: `audit-areka-app`・`audit-areka-install-update` はこの spec の着地を待つ。`multi-ghost-runtime` は、v0.1.0 の余力の枠で着地していなければこの spec の後に、新しい crate の形で測り直して取る。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - `multi-ghost-runtime`: ファイルがほぼ全部重なる。同じウェーブに置かない。
  - `shiori4-api`（v0.2.0・`ghost_session` を触る）・`resident-ghost-rotation`（v0.4.0）・`tasktray-icon`（windows-reactor 待ち）: roadmap の新旧のパスの対応表で読み替える。

## Constraints

- **検証（要件に入れる）**: roadmap の「本体の crate 分け」の「共通の約束」の検証の項を、そのまま要件に入れる。この spec は向きを逆にするので、実機で次の道を 1 回ずつ回し、記録の順が前後で同じことを示す。
  - 起動
  - メニューからのゴーストの切替
  - MCP の `reload`
  - メニューの終了
  - 煙の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）
  - 窓への `.nar` の投げ込み

  OS のセッションの終わり（`WM_ENDSESSION`）は、今の決定論テストで足りる（実機では起こせない）。
- **フレームを遅らせない**: 逆転した知らせや口が、切替・終了・差し込みを 1 フレームでも遅らせない。
- **共通の約束**: roadmap の同じ節のとおり。
- **Fable**: Opus で足りる（口は本体と session の間だけの内向きのもの）。
