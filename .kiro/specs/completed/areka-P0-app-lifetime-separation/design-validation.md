# 設計レビュー: areka-P0-app-lifetime-separation

> 2026-09-23・本ブランチ（`claude/areka-p0-app-lifetime-c96473`）で `design.md` が引く実ファイルを読み直して検証した。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。非対話で行ったレビューであり、下の指摘は設計ディスカッションの議題として扱う。

## レビュー要約

設計が既存コードについて述べている事実は、引用先を読んで確かめた範囲ですべて成り立っている——`WinApp::new`（`crates/wintf/src/runtime/mod.rs`）が `fn wire_shutdown_hook` で登録表の空遷移フックに `Event::notify` を仕込む唯一の終了起点であること、`ShutdownPolicy::shutdown_future`（`crates/wintf/src/runtime/message_loop.rs`）が通知を記憶しないこと、ウィンドウ手続き（`crates/wintf/src/runtime/wndproc_bridge.rs` の `make_wndproc`）が World の `try_borrow` 失敗を既定手続きへ流すこと、ライブラリの `Window<S>` の `Drop` が `let _ = DestroyWindow(hwnd)` だけであること、`despawn_ghost_windows`／`despawn_smoke_targets` の呼び手が設計の言うとおり 3 か所＋テスト 3 本＋doc 2 行に限られ example からは参照されないこと。wintf の変更は既存ファイル内の 3 定義で収まり、areka の変更は「終了とは何か」を 1 モジュールへ集める形で、既存の層構造（runtime→World の下向き注入・`ClickThroughRegistryHandle` と同型の NonSend）に沿っている。実装へ進んでよい品質である。指摘 3 件はいずれも設計の骨格を変えず、文書の 1 段落か記録の文面の追随で済む。

## 重要な指摘（3 件）

### 指摘 1: OS 起点で最後の窓が閉じられたときプロセスが残る経路を、設計が名指ししていない

- **懸念**: 窓を閉じる経路は areka の 4 か所だけではない。wintf の `WM_CLOSE` ハンドラ（`crates/wintf/src/ecs/window_proc/lifecycle.rs` の `fn WM_CLOSE`）は届いた窓 1 枚の entity を despawn する。ゴースト窓は `WS_POPUP | WS_VISIBLE`／`WS_EX_LAYERED | WS_EX_TOOLWINDOW`（`crates/areka/src/placement/spawn.rs` の `fn window_style`）で `WS_EX_NOACTIVATE` を持たないため、クリックで活性化した窓への Alt＋F4（`WM_SYSCOMMAND`→既定手続き→`WM_CLOSE`）はこの経路に乗る。今日は 4 枚を順に閉じれば登録表が空になって終了するが、`ExitPolicy::Explicit` では「窓 0 でプロセスが残り、終了の入口が無い」状態になる。`quit_app` を通らないので裁定 2 の「片方だけ呼べない形」の外側にある。
- **影響**: 要件 1.1 の意図どおりの振る舞いではあるが、利用者から見える「終わらせられない」状態を作り得る。要件の Out of scope（トレイ等の入口・時間の安全網）と整合させるには、設計がこの経路を**受け入れる**のか**塞ぐ**のかを書いていないと、実装者もレビュアも判断できない。
- **提案**: 設計の Non-Goals または Error Handling に 1 段落——「OS 起点の窓 1 枚の閉鎖（`WM_CLOSE`→despawn）は本仕様で扱わない。`Explicit` では最後の窓が OS 経由で閉じてもプロセスは残る。入口は後続（トレイアイコン等）の仕事」——を足し、並走 spec への申し送りにも 1 行加える。塞ぐ判断をするなら、areka 側でゴースト窓の `WM_CLOSE` を終了指示（`MouseWiring::send_close_request`）へ寄せる案が最小だが、これは要件の範囲を広げるので開発者の裁定が要る。実機確認の走行に「Alt＋F4 で最後の窓を閉じたときの挙動」を 1 行入れておくと、決めた側がどちらでも記録に残る。
- **Traceability**: 要件 1.1・3.6・6.2・Boundary Context「Out of scope」
- **Evidence**: design.md「Non-Goals」「Error Handling」「並走 spec への申し送り」

### 指摘 2: 移送する 3 本のテストは「本文不変」では通らない——打ち切り行の相名が変わる

- **懸念**: 設計は `main_seam_tests.rs` の `despawn_smoke_targets_*` 3 本を `app_exit_tests.rs` へ「本文は不変・関数名だけ追随」で移すと書く。しかし連鎖破棄の 1 本（`despawn_smoke_targets_skips_cascade_despawned_target_without_warning`）は打ち切り行の文面 `skipped.message().contains("smoke 自動 close")` を主張しており、この文面は今日の `despawn_smoke_targets`（`crates/areka/src/main.rs`）が出す「smoke 自動 close: 標的 entity は既に破棄済み…」の相名である。私有部品 `despawn_app_windows` は 4 出所（終了系列の完了・強制退避・ダミー窓・smoke）共通になるので、文面をそのまま残すと smoke 以外の終了でも記録が「smoke 自動 close」と名乗る。
- **影響**: 文面を変えなければ記録が実態と食い違い、変えればテストの主張も追随が要る。「本文不変」のまま実装すると、どちらかが赤になるか、記録が嘘をつくかのどちらかになる。
- **提案**: 打ち切り行の相名を `[quit_app]`（設計が採った areka 側の記録の接頭辞）へ改め、テストの主張もその語へ追随させる。設計の「既存テストの追随」表の該当行を「本文の判断は不変・相名の文面だけ追随」に直す。監視の検索語（Monitoring）に `[quit_app]` を足す。
- **Traceability**: 要件 3.6・3.7・4.4
- **Evidence**: design.md「Testing Strategy > 既存テストの追随」の `main_seam_tests.rs` 行・「quit_app と ExitOrigin > Responsibilities」

### 指摘 3: `run()` が登録表を取り去って戻る契約と、`request_exit` の前提が公開面に書かれていない

- **懸念**: 要件 1.3 の実現手段——`run()` が `block_on` 復帰直後に `remove_non_send::<ProdWindowRegistry>()` で登録表ごと drop する——は、areka の経路（`quit_app` が先に全窓を despawn している）では entity の消えた後に `DestroyWindow` するだけで、今日の `reconcile_window_registry` と同じ条件である（この点は設計の主張どおり）。ただし `AppExit` は `pub` で、`request_exit` は窓を閉じずに呼べる。wintf 単体の利用者がそう呼ぶと、`run()` は entity が生きたまま（`WindowHandle`・WUC の資源が World に残ったまま）`DestroyWindow` する。これは今日の経路には無い新しい条件で、`WinApp` の drop まで World 側の資源が無効な HWND を指し続ける。また `run()` の後は登録表が World に無いので、`run()` を 2 度呼ぶ運用が「想定外」から「不可」に変わる。
- **影響**: areka では起きない（`quit_app` が構造で防ぐ）。しかし wintf の公開 API の契約として文書化されていないと、後続（ゴースト切替）や example の利用者が `request_exit` を「閉じる前に呼んでよい口」と読む余地がある。
- **提案**: `WinApp::run` の doc に「明示の指示で戻るとき、登録表に残った窓を壊してから戻る。以後 `run()` は再入できない」を、`AppExit::request_exit` の doc に「窓は閉じてから呼ぶこと（残った窓は `run()` が壊すが、その entity の資源は `WinApp` の drop まで残る）」を書く。テストは足さない（判断分岐ではなく契約の明文化。要件 5.3）。
- **Traceability**: 要件 1.3・2.5
- **Evidence**: design.md「WinApp::run の残存窓破棄 > Responsibilities & Constraints」「AppExit > Service Interface」

## 設計の強み

1. **終了の完了機構が 1 本になる。** 既定ポリシーの空遷移フックも明示の指示も同じ `AppExit::request_exit` を通り、`ShutdownPolicy::shutdown_future` の「arm → 指示済みの確認 → await」が単一スレッド上で要件 1.4・1.5 を順序だけで閉じる。`event-listener` がリスナ不在の通知を失う既知の性質に対して、記憶 1 ビットを足すだけで足りており、`window_registry.rs` は 0 行、新しい依存は 0 である。
2. **段取りが検証可能になっている。** `main` の `with_exit_policy(Explicit)` を最後に入れることで、それまでの各段は既定ポリシーのまま smoke テストが緑であることを確かめられる（`quit_app` の指示が先に立ち、空遷移フックの 2 回目は `debug!` で流れる）。裁定 2（片方だけ呼べない形）は `despawn_ghost_windows`／`despawn_smoke_targets` の削除と私有化で構造として守られ、呼び手 3 か所と example に参照が無いことは実測で裏が取れた。

## 最終判断

**GO**

- **根拠**: 既存アーキテクチャとの整合（runtime 層の facade 拡張・NonSend の先例踏襲・上向き依存なし）、要件の追跡表の網羅、明確な実装経路（wintf 3 定義＋areka 1 モジュール＋4 か所の 1 行化＋テスト 4 本）が揃っている。指摘 3 件はいずれも設計文書の 1 段落か記録の文面の追随で解消でき、骨格の変更を要しない。
- **次の段取り**: 設計ディスカッションで指摘 1 の裁定（受け入れる／塞ぐ）を取り、指摘 2・3 を設計へ反映してから `/kiro-spec-tasks areka-P0-app-lifetime-separation` へ進む。

## 補足の観察（ディスカッションで軽く扱うもの）

1. **`new_owns_unfired_shutdown_signal` の主張を 1 行強める。** 欄の置き換え（`app.shutdown.listen()` → `app.exit.signal().listen()`）で意味は変わらないが、`!app.exit.is_requested()` を並べて主張すると「構築直後は指示されていない」が signal の待ち時間ではなく状態で固定される。
2. **doc の追随箇所が設計の一覧より多い。** `crates/areka/src/input_events/mod.rs` の `on_char_pointer_pressed` の doc と強制退避の腕のコメント（「window-close funnel（`run()` 復帰→main shutdown→`ForceQuit` 系列）」）、`crates/areka/src/main.rs` の `on_dummy_pressed` の doc と `app.run()` 直前のコメント、`crates/areka/src/placement/spawn.rs` のモジュール doc の「全 `GhostWindowMarker` despawn→window-close funnel→`run()` 正常復帰」の行、`crates/areka/tests/smoke_boot_loop_exit.rs` のモジュール doc「自動 despawn → `WindowRegistry` 空遷移 → `run()` 復帰」。設計の File Structure Plan は `frame.rs`・`spawn.rs`・`main.rs`・smoke テストを挙げているが、`input_events/mod.rs` は「import の追随」としか書いていない。タスク生成時に doc 追随の対象として明記しておくと取りこぼさない。
3. **wintf の新テスト 4 本の置き場。** `runtime/mod.rs`（628 行）と `message_loop.rs` の既存のファイル内 `mod tests` に足す形は歴史的形式の維持として許容範囲だが、steering の「新規のテストモジュールは兄弟ファイルへ」に照らすと、`mod.rs` は 700 行近くまで増える。1,000 行の目安の内側なので今回は問題にしないが、次にこの 2 ファイルへテストを足す spec が兄弟ファイルへ切り出す契機になる。
4. **`quit_app` の記録の語彙。** `ExitOrigin::KanadeStopped(cause)` の `Debug` は `KanadeStopped(Quit)` と出るのに対し、既存の `ghost_quit` の `cause` は `Quit` と出る。同じ語を含むので検索は通るが、`origin=KanadeStopped(Quit)` を等値で引く場合は括弧込みになる。実機確認の期待行はそのとおり書かれているので、実装時に文面を変えないこと。
5. **Flow 2 の順序は 2 通りとも正常。** smoke の `quit_app`（tick の外）のあと、`block_on` が戻る前に vblank の tick が 1 巡すれば同 tick の `FrameFinalize` が窓を壊し、`run()` の残存窓破棄は空振りになる。設計はこれを「どちらも正常」と書いており、実機確認で `windows remained open` の有無を合否にしないことが要点である。
