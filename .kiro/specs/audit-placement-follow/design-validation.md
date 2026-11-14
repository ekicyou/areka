# 設計の検証: audit-placement-follow

- 検証の日: 2026-11-14
- 読んだもの: `spec.json`（language=ja）・`requirements.md`・`design.md`・`research.md`・steering（`structure.md` のテスト分離の接続規約の節だけ。`product.md`・`tech.md`・`logging.md` は設計と research が引いた決まりを読んだにとどまり、全文は読み直していない）・`kiro-validate-design` の `rules/design-review.md`
- コードで確かめた所: `follow/move_shift.rs`（全体）・`follow/window_move.rs:20-130`・`follow/drag_follow.rs:170-262,325-784`・`follow/visibility.rs:140-365`・`follow.rs:75-175`・`follow/work_area_clamp.rs:110-135`・`diag.rs` の `is_system_reanchor`・本体 `runtime/frame/move_shift.rs`・`runtime/move_cue_place.rs:195-232`・番人 `crates/log-capture-kit/tests/file_length_guard_test.rs:55-200`・`follow_test_support.rs` の語の定数
- 対話は行っていない（非対話の実行・判定を直に出す）

## 設計の検証のまとめ

28 件の指摘それぞれに、直す所・呼び出し元・壊れたら落ちるテストが名指しで書かれ、裁定（21・8・28 の一部・議題 6・11）とも食い違いが無い。必ず直す 1〜4 の直し方はコードの事実（`rebase_shift_axes` が札を見ずに `apply_char_shift` を呼ぶ・`move_window_with_route` が自前の式で随伴を書く・`on_balloon_drag_end` が `BalloonFollow` を出し直さない・`evaluate_visibility_guard` の `OffscreenPull` の条件が `raw == proposed` のバルーンで常に当たる）と合っている。ただし指摘 1 の新しい印がドラッグの終わりの口と噛み合っておらず、そのままでは完了した spec の決めを壊す新しいバグを生む。

## 確かめた主な点（問題なし）

- **指摘 1**: 毎フレームの相（本体 `runtime/frame/move_shift.rs`）は `shift_is_current` が偽でも `dpi_sync::defers_window_write(.., HoldSite::PatternMove)` が真なら書かない。設計の「印が立っている間は相が札の明けた後に当て直す」はこの形で成り立つ。`defers_window_write(world, window, site)` は核の `pub fn` で、署名を変えずに呼べる。
- **指摘 2**: `move_window_with_route` の呼び手は `move_window_to`（`MoveCue`）・核の `chain_realign.rs:351`（`ChainRealign`）・`move_shift.rs:176`（`PatternMove`）と、テストの `dpi_sync_tests.rs`・`frame_work_area_resnap_hold_tests.rs`・`follow_window_move_hold_watch_tests.rs`。`is_system_reanchor` は `MoveCue` で偽・`ChainRealign`／`PatternMove` で真なので、`follow_balloon(.., BalloonFollowTrigger::Placement(route))` へ替えても `\![move]` の書込は今と同じ。
- **指摘 3**: `on_balloon_drag` の出し直しの塊（`:546-598`）は World と バルーンと位置だけで閉じており、私有の関数へ切り出せる。保存（`:770-771`）の直後・退役（`:775`）の前に置けば DD6 の順は崩れない。
- **指摘 4**: `guard_balloon_position` は `raw` と `proposed` に同じ値を渡す（`:500-501`）。条件に `raw != proposed` を足せばバルーンの道では `OffscreenPull` が出なくなり、キャラ窓の `apply_char_shift` の道（`clamped, clamped`）でも今と同じに出ない。
- **1,000 行の計画**: 例外表は 5 件（`OVER_LIMIT_ALLOWED_COUNT = 5`）で、受け持ちの 3 本が載っている。`every_over_limit_exception_is_still_over_the_limit` があるので、`transition_judge_tests.rs`（1,039 行）が上限の下に入れば表の項目と件数の定数を消すのは必須という設計の読みは正しい。指摘 19 の `:628-668`（41 行）だけで 998 行になり、15・22 の減りもあるので見込みは立つ。自己検査の `CALIBRATION_DROPPED_ENTRY` は `frame_transition_branch_tests.rs` で、この spec は触らない。
- **公開を狭める（指摘 28・議題 8）**: ツリー全体（`vendors/` を除く）を `git grep -w` で引き直した。`VisibilityVerdict`・`guard_visibility`・`DragPositionPolicy`・`BottomSnapPolicy`・`resolve_chain_in_work_area`・`WORK_AREA_KEEP_TAG`・`WORK_AREA_UNRESOLVED_TAG` は、範囲の外のコードの使い手が 0 件（範囲の外に出るのは完了した spec の文書と説明の文だけ）。`MonitorDpiEntry` は核の `dpi_sync_tests.rs` が名で呼び、公開の欄 `MonitorDpiTable.entries` の要素の型なので、狭めないという設計の振り分け（`research.md` 13.2）は正しい。`ChainPlacement`・`ChainClampOutcome` は範囲の外のコードの使い手が 0 件で、`pub use` を足すだけなら要件 7.1 に当たらない。
- **新しいテストのファイルの名**: `structure.md` の `<stem>_<モジュール名>.rs` の規約どおり。

## 重い問題（3 件まで）

### 重い問題 1: 指摘 1 の「当て直し待ち」の印が、ドラッグの終わりの `reset_applied` で消えない

- **何が問題か**: 設計は印 `awaiting_reapply` を「`apply_char_shift` が書けたときだけ `false` の記録へ置き換わる」と決めている（Data Models の不変条件）。ところがドラッグの終わり（`on_char_drag_end` の `drag_follow.rs:194`・`on_balloon_drag_end` の `:684`）が呼ぶ `reset_applied`（`move_shift.rs:107-111`）は `applied` だけを 0 にし、`want`・`ratio` を残して「毎フレームの相は同じずれを当て直さない」形にしている。印が立ったまま（札の間・作業領域が取れずに書けなかった後）に利用者が窓をドラッグして離すと、`shift_is_current` が偽のままなので、次のフレームの相が利用者の置いた位置へずれをもう一度足し、窓が置いた所から跳ぶ。
- **なぜ効くか**: 完了した `seriko-pattern-calls` の決め（「利用者のドラッグの終わりだけは、その窓の当てた分を 0 にする。利用者が置いた所を新しい元の位置にする」・task 6.2）を、必ず直すの直しが新しく壊す。要件 4.1 の 1 に当たる形が議題を経ずに入り、再 audit でも指摘に上がりうる（要件 5.2）。
- **直し方**: `reset_applied` で印も `false` に戻す（1 行）。不変条件の文を「`apply_char_shift` が書けたとき、または利用者のドラッグの終わり（`reset_applied`）で `false` になる」に直す。テストを 1 本足す（`follow_move_shift_tests.rs`・または `follow_drag_end_persist_tests.rs`）: 札のある窓で絶対の軸の当て直し → 印が立つ → `DragEnd` → `shift_is_current` が真・毎フレームの相を回しても書込の列が空。直す前（印を消さない形）では赤になる。
- **要件**: 3.3・4.1・5.6
- **設計の箇所**: 「指摘ごとの直し方」の 1・「Data Models」の不変条件・「Testing Strategy」の 1 の行

### 重い問題 2: 指摘 23・28 で消す・狭める名前の参照の後始末が「直す所」に揃っていない

- **何が問題か**: `BottomSnapPolicy`・`DragPositionPolicy` を消し、`guard_visibility`・`VisibilityVerdict` を `pub(crate)` へ下げると、次の参照が残る。設計の 23・28・19 の「直す所」と申し送りの一覧には、どれも入っていない。
  1. 範囲の中のテスト `follow_anchor_tests.rs:93-112`（「Bottom は BottomSnapPolicy と同値」）が `BottomSnapPolicy.resolve` を呼ぶ。設計 19 が消すと決めたのは `:18-81` だけで、このテストはビルドが通らなくなる。
  2. 範囲の中の説明のリンク `[`BottomSnapPolicy`]`（`drag_follow.rs:48`・`window_move.rs:116`）が行き先を失う。`pub fn follow_balloon` の説明の `[`guard_visibility`]`（`drag_follow.rs:367,425`）は、公開の項目から私有の項目へのリンクになる。
  3. 範囲の外の核の説明（`persist.rs:227,238`・`persist_restore_tests.rs:295`・`resolver.rs:438`）が消えた名前 `BottomSnapPolicy` を名指したまま残る。設計 22 の範囲の外の古い参照の一覧（`runtime/frame_transition_atomicity_tests.rs`・核の `diag_tests.rs`）にも、核へ伝える一覧にも入っていない。
- **なぜ効くか**: 要件 6.3（消す前にツリー全体で文字列での参照まで確かめる）と 6.4（範囲の外は持ち主へ伝える）の確かめが一覧に落ちていない。1 はビルドの赤、2 は再 audit が古い説明として拾い、0 件に収まるまでの回数を増やす（要件 5.2）。3 はこの spec の直しが範囲の外に新しく作る古い名前で、後から着地する `audit-placement-core` が知らないと見落とす。
- **直し方**: 23 の「直す所」に `follow_anchor_tests.rs:93-112` を足す（私有の `bottom_snap` と比べる形にするか、`project_anchor(Bottom, ..)` の期待の値で書き直す）。説明の名指しは、消す名前はふつうの文の語へ、狭める名前はリンクを外すか `pub(crate)` の項目の説明だけに残す。範囲の外の 4 行は直さず、`audit-placement-core` へ伝える一覧（設計「並走の申し合わせ」の 28 の項・16・22 の一覧）に足す。
- **要件**: 3.3・5.2・6.3・6.4・7.8
- **設計の箇所**: 「指摘ごとの直し方」の 19・23・28・「並走の申し合わせと着地の順」

## 重くはないが設計の討議で拾うとよい点

- **1,000 行の例外表の残り 2 本**: 設計は `transition_judge_verdict_tests.rs`（1,037 行）を「表は残す」と書くが、19 と 15 の減りで 1,000 行以下になれば `every_over_limit_exception_is_still_over_the_limit` が赤になる。見込みは約 1,024 行で越えたままだが、「上限の下に入ったら表から消す」（要件 8.3）を 3 本とも同じ決めにしておくと、実装の途中で迷わない。
- **手順書の語の数**: `diagnosis-procedure.md` の「3 語は接頭辞 `[visibility-guard]` を共有する」は O17〜O19 の 3 語を数えていて、`OffscreenPull` は表に載っていない（grep 0 件）。設計の「3 語を今の数に直す」の「今の数」が 4（O17〜O19＋新しい語）か 5（`OffscreenPull` も足す）かを決めておく。
- **新しい接続宣言のモジュール名**: `transition_judge_offset.rs` の今の接続宣言は、モジュール名にファイルの stem をそのまま使っている（`mod transition_judge_offset_tests;`・`mod transition_judge_offset_signoff_tests;`）。設計の `mod negative_tests;` は `structure.md` の規約どおりだが、同じファイルの中で形が 2 つになる。どちらに揃えるかをタスクに書いておく。

## 設計のよい所

- **直し方がコードの名と行で引ける**: 28 件の 1 件ずつに、直す関数・変える条件・足すテストの形（直す前は何が起き、直した後に何が起きるか）が書かれ、呼び手の署名を変えないことまで確かめてある。必ず直すの 1〜4 は、ここで読んだコードの事実とすべて合っていた。
- **並走と番人への目配り**: 1,000 行の計画が例外表の自己検査の振る舞い（上限の下に入れば消すのが必須）まで読み、議題 6 では完了した spec の決め（`env-filter` は `wintf` だけ）と見張りに当たることを設計の段で見つけて案 乙へ決め直している。C20 の段・着地の順・相手の触る行（`fake_handle` の 1 行・`initial_inset`・`is_primary`）の申し合わせも具体的に書かれている。

## 最後の判定

- **判定: GO**
- **理由**: 境目・依存の向き・公開の形・番人の決めとの食い違いは無く、直し方は実装へそのまま落とせる。重い問題 2 件はどちらも直しの手順の抜け（印を消す 1 行と、消す名前の参照の一覧）で、設計の形を変えずに設計の討議で書き足せる。
- **次の手順**:
  1. 設計の討議（`/kiro-design-discussion audit-placement-follow`）で重い問題 1・2 を設計に書き足す（1 は `reset_applied` で印を戻す・テスト 1 本、2 は 19・23・28 の直す所と核へ伝える一覧に足す）。
  2. 重くはない 3 点の扱いを決める。
  3. `/kiro-spec-tasks audit-placement-follow` でタスクを作る。
