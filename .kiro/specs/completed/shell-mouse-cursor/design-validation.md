# 設計の検証: shell-mouse-cursor

- 日付: 2026-10-29
- 対象: `design.md`（要件 `requirements.md` は確定済み）
- 進め方: `kiro-validate-design`（分析 → 大事な指摘 → よい所 → GO／NO-GO）。対話なしで判定まで出す。
- 照らした実物: `crates/areka/src/input_events/{mod.rs, char_watch.rs, hover_region.rs}`・`crates/wintf/src/ecs/window_proc/{mod.rs, mouse_move.rs, mouse_click.rs, mouse_dblclick_wheel.rs}`・`crates/wintf/src/ecs/clickthrough/controller.rs`・`crates/wintf/src/ecs/layout/hit_test/mod.rs`・`crates/wintf/src/ecs/world/tick_wake.rs`・`crates/areka/src/emo2_boot/{assets.rs, frame/switch.rs, shell_balloon_switch.rs}`・`crates/areka/src/placement/source.rs`・ワークスペースの `Cargo.toml`

## まとめ

設計は要件の 1〜7 をすべて部品と流れに割り当て、「その場で引く（`WM_SETCURSOR`）＋カーソルの来ない場面だけ出し直す」の形で 1 フレームの遅れを持ち込まない。土台にした実物の主張（`resolve_hit_owned` が子孫のモジュールから見えること・`WM_SETCURSOR` の今の道・`SwapFinish::Shell`・`evaluate_targets` が `&World` を持つこと・`read_kv_lenient` の場所・`windows` の機能が足りていること）は、どれもコードで確かめられた。残る指摘は、解放の出し直しの差し込み場所・Win32 の層の判断の分岐のテスト・いくつかの失敗の道の記録で、どれもタスクの段で直せる大きさなので GO とする。

## 確かめた主張（コードと突き合わせた結果）

| 設計の主張 | 結果 | 根拠 |
| --- | --- | --- |
| `resolve_hit_owned`・`char_scope`・`MouseWiring` を、`hover_region.rs`・`char_watch.rs` の `#[path]` の子から `mod.rs` を変えずに使える | 正しい | `resolve_hit_owned` は `input_events/mod.rs:445` の非公開の関数（子孫のモジュールからは見える）。`char_scope` と `MouseWiring` は `pub(crate)`。`char_watch.rs` はすでに `use super::{MouseWiring, char_scope, resolve_hit_owned}` で使っている |
| `#[path = "cursor_pick.rs"]` を `hover_region.rs` に置くと `input_events/cursor_pick.rs` を指す | 正しい | `char_watch.rs` 末尾の `#[path = "char_watch_tests.rs"]` と同じ置き方 |
| `WM_SETCURSOR` は今、十字の診断のときだけ答え、ほかは `None` | 正しい | `mouse_move.rs:30-54`。引数は `_world` ほか `_` 付きで、設計どおり外して使える |
| 絵の透けた所の見分け（持ち主が窓そのもの・無い）が `char_watch` と同じ物差し | 正しい | `char_watch.rs` の `transparent: owner == window`、ボタンの手続きの「フォールバック: ウィンドウエンティティに記録」、`hit_test_in_window` が `None` を返すと透過の制御が `Transparent` にする（`controller.rs:58-80`） |
| `evaluate_targets` は `&World` で呼ばれ、`push_cursor(&World, …)` を足せる | 正しい | `controller.rs:145`（`try_borrow` した世界を渡す） |
| `SwapFinish::Shell`・`SwapBuilt::Shell`・`finish_shell` に欄を足す道 | 正しい | `shell_balloon_switch.rs:144`・`frame/switch.rs:387,435,619,642`。テストの字面（`switch_finish_tests.rs`・`switch_tests.rs`）が多いのも設計の「構造体の字面を書くテスト」の列挙どおり |
| `read_kv_lenient` は `placement/source.rs` の非公開の関数 | 正しい | `source.rs:186`。`pub(crate)` へ広げるだけで足りる |
| `WM_SETCURSOR` は `tick_wake` の `POINTER` の表に在り、`WM_MOUSEHOVER` は無い | 正しい | `tick_wake.rs:305-310`。1 語足す計画と合う |
| 依存の追加なし（`windows` の機能・`thiserror`） | 正しい | ワークスペースの `Cargo.toml` に `Win32_UI_WindowsAndMessaging`・`Win32_UI_Input_KeyboardAndMouse`・`Win32_UI_HiDpi`・`Win32_Graphics_Gdi` が在り、wintf は `thiserror` を持つ |
| `input_events/mod.rs` に 1 行も足さない | 守られている | 新しい口はすべて `hover_region.rs`・`char_watch.rs` の子と `register_char_watch` の 1 行 |
| 1,000 行の目安 | 守られている | 足す先で大きいのは `shell_target.rs` 855・`emo2_boot/mod.rs` 795・`frame/switch.rs` 679・`mouse_move.rs` 632 行。どれも 10 行以内の足し |

## 大事な指摘（3 件）

### 指摘 1: ボタンを放したときの出し直しを `handle_button_message` の「終わり」に置くと、世界を借りられずに黙って効かない

- **問題**: `mouse_click.rs` の `handle_button_message` は、当たり判定が当たった枝で `world.try_borrow_mut()` を握ったまま `return Some(LRESULT(0))` する（`mouse_click.rs:72` から 185 行付近）。設計は「解放（`is_down == false`）の終わりで `push_cursor_rc`」としか書かないので、この枝の中に足すと `push_cursor_rc` の借用が必ず失敗し、`trace!` だけ残して形を出し直さない。
- **影響**: ドラッグで捕まえた窓（`CaptureGuard` で `SetCapture` 中）は `WM_SETCURSOR` を受けないので、放した後も握った手の形が次の移動まで残る＝要件 3.4 の「その場で」が外れる。決定論のテストでは見えず、実機の確かめ 2 で初めて見つかる。
- **直し方**: 出し直しは `WM_LBUTTONUP`・`WM_RBUTTONUP`・`WM_MBUTTONUP`・`WM_XBUTTONUP` の各手続きで、`handle_button_message` が返って借用が外れた後に呼ぶ、と設計の Modified Files に書き直す（または `handle_button_message` の返り値を一度受けてから呼ぶ）。`push_cursor_rc` の借用の失敗は `trace!` でなく `debug!` にして、実機の確かめで見えるようにする。
- **当たる要件**: 3.4・3.6
- **設計の場所**: 「Modified Files」の `mouse_click.rs` の行・「出し直す場面」の表の「放した」・CursorWndproc の Implementation Notes

### 指摘 2: wintf の答えの層の判断の分岐が、決定論のテストに入っていない

- **問題**: `on_set_cursor` と `push_cursor` には、形を左右する分岐が 5 つある（当たりの結果が `HTCLIENT` か・`Show`／`Keep`／`None` をどう `SetCursor` と `LRESULT` に写すか・止まった通知を掛け直すのは `Show` かつ `Move` のときだけ・出し直しは `WindowFromPoint` が自分の窓のときだけ・`Keep` と `None` では `SetCursor` しない）。設計のテストは `trigger_of` と「`CursorQuery` が無い世界で何もしない」だけで、残りを「結線は実機で見る」としている。
- **影響**: とくに「`Keep` は `Some(LRESULT(1))` で既定の処理を止める」は要件 2.5 の要で、`None` と取り違えると透けた所で窓のクラスの矢印が出る。これは配線でなく判断なので、プロジェクトの決まり（判断の分岐は決定論のテストで固定する）に当たる。
- **直し方**: `answer.rs` から Win32 を呼ばない純関数を 1 つ切り出す。例: `plan_set_cursor(hit_test: u32, reply: Option<CursorReply>, trigger) -> SetCursorPlan { show: Option<HCURSOR>, result: Option<LRESULT>, rearm_hover: bool }` と `push_allowed(cursor_window: HWND, own: HWND) -> bool`。`answer_tests.rs` で表の全行を固定し、Win32 の呼び出しはその結果を実行するだけの薄い層にする。
- **当たる要件**: 2.5・4.2・7.2
- **設計の場所**: CursorWndproc の Responsibilities・Implementation Notes の Validation・Testing Strategy の Unit Tests 5

### 指摘 3: 答えの層の Win32 の失敗のいくつかに記録の行が無い

- **問題**: 失敗の表（Error Categories and Responses）は読み込みと資源の失敗を押さえているが、答えの層の `GetCursorPos`・`ScreenToClient`・`WindowFromPoint` の失敗（安全なデスクトップ・窓が消えた直後など）と、`GetDpiForWindow` が 0 を返したときの扱い（寸法 0 で同梱の絵を作ろうとしない）が無い。どれも「その場は今の振る舞いへ落とす」で足りるが、記録の段と落とし先が決まっていない。
- **影響**: プロジェクトの決まり（記録の無い失敗の道を持たない）に外れる。とくに DPI が 0 のまま `from_cur_bytes` に進むと、寸法ごとに 1 度の `error!` が「寸法 0」で出て原因が読み違えられる。
- **直し方**: 失敗の表に 2 行足す。「位置・窓が取れない（`GetCursorPos`・`ScreenToClient`・`WindowFromPoint`）→ `None`／出し直さない → `trace!`（移動のたびに来るので）」「`GetDpiForWindow` が 0 → 96 として扱う → `debug!`」。
- **当たる要件**: 6.5（落とし先の考え方）・logging.md
- **設計の場所**: Error Handling の Error Categories and Responses・CursorApi の `cursor_size_for_dpi`

## よい所

1. **1 フレームの遅れを持ち込まない形を、実物の道に沿って選んでいる**。当たり判定の境目は `WM_SETCURSOR` の中でその場で引き（`HoverState` の 1 tick 古い値に頼る案 A を明示して退けた）、カーソルの来ない場面（止まった・ホイール・捕まえたままの解放・透過が外れた直後・シェルの入れ替え）だけを表にして出し直す。「変わった時点が分かるならプッシュ」の決まりにそのまま合い、`input_events/mod.rs` に触らない約束も、可視性の規則で実際に守れることを確かめた。
2. **判断を純関数に寄せ、表で固定できる形にしている**。`plan_cursor` の遷移の表（13 行）と `pick_shape` の 5 段の順（cursorブレス ＞ ゴーストの descript ＞ 既定・行の無い止まった／ホイールは乗る形へ落とす＝SSP の測り）が、そのままテストの表になる。読み込みの失敗は「表から外す＝行が無いのと同じ既定」に一本化され、不変条件（`File(n)` は必ず `files[n]` を指す）と入れ替えの順（入れる → 出し直す → 捨てる）も明記されている。

## 気付いた小さな点（GO を変えない）

- 当たり判定から絵の透けた所へ出た瞬間、透過の切り替え（非同期の見張り）が済むまでの間は窓が不透過のまま `WM_SETCURSOR` を受け、`Keep` で前の当たり判定の形（指差しなど）が次の移動まで残る。要件 2.5（決めない）には合っており、矢印を出す代案はかえってちらつくので、今の形でよい。§8 か Open Questions に 1 行あると後の読み手が迷わない。
- ゴーストの descript は `resolve_with_shell` がすでに一度読んでいるが、`MountModel` は KV を残さないので `read_kv_lenient` で読み直す計画は妥当（依存も増えない）。

## 判定

- **判定: GO**
- **理由**: 既存の層と境目に合い、要件の全項目に部品・流れ・テストの割り当てがあり、土台にした実物の主張はコードで確かめられた。指摘 3 件はどれも設計の骨組みを変えず、タスクの段で直せる。
- **次の一歩**:
  1. 指摘 1〜3 を、設計の短い直し（Modified Files の `mouse_click.rs` の行・Testing Strategy の Unit Tests 5・失敗の表の 2 行）として取り込むか、`/kiro-spec-tasks shell-mouse-cursor` のタスクの完了条件に入れる。
  2. そのうえで `/kiro-spec-tasks shell-mouse-cursor` でタスクを作る。
