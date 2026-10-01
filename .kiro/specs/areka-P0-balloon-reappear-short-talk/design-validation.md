# 設計レビュー: areka-P0-balloon-reappear-short-talk

- 対象: `design.md`（2026-10-01 生成・HEAD `adbb4cdb`）
- 照合した入力: `spec.json`・`requirements.md`・`research.md`・steering（`structure.md` のテストの置き場・名前・1,000 行の節）
- 照合したコード（HEAD の実物）: `crates/areka/src/emo2_boot/balloon_visibility.rs`・`balloon_visibility_phase.rs`・`balloon_visibility_phase_tests.rs`・`balloon_visibility_test_support.rs`・`balloon_visibility_forget_tests.rs`・`balloon_visibility_timeout_suppression_tests.rs`・`crates/areka-emo-text/src/state.rs`・`actor.rs`・`state_cue_apply_tests.rs`・`crates/areka/src/shell_balloon_switch_session_balloon_tests.rs`・`crates/areka/src/emo2_boot/mod.rs`（受け口の並び）・`talk_lifecycle.rs`・`crates/dola/src/cue/runtime.rs`（配りの輪）・`crates/areka-sakura/src/compile.rs`（先頭の `ClearAll`）・`doc/COMPAT_ARCHITECTURE.md` §8

## 設計レビューの要約

文字の層が scope ごとの「消去の回数」を数え、観測が見える文字の数と同じ借用・同じ時刻でそれを運び、判定は「回数が変わったら比べる相手を 0 にする」という 1 か所の変更で穴を塞ぐ設計である。表示の時期は動かさず（同じフレームで解く）、隠す側の分岐と記憶の扱いは変えない。コードと突き合わせた結果、設計の具体的な主張はすべて実物と一致し、作るものを変えるべき重大な問題は見つからなかった。

## 確かめたこと

### (a) 修正の前に赤になるテストが HEAD でコンパイルでき、実際に失敗するか

- 再表示のテストは配線の段（`balloon_visibility_phase.rs`）の子モジュールに置く。`collect_observations` は配線の段の非公開の関数だが、子モジュールからは呼べる。`decide`・`BalloonVisibilityState::forget_scope` は `pub(crate)`、`TextLayerRuntime::apply_cue`・`TalkLifecycleSignal` は公開。`ScopeObservation::visible` は `pub(crate)` で書き込める。新しい型 `GlyphObservation` と `clear_count` を名指ししないので、HEAD でも修正の後でもコンパイルが通る。
- 赤の 7 本を HEAD の判定（`decide_content` の `glyphs > last_glyphs && !visible && !state.break_latch`）でなぞった結果:
  - 時間切れ・中断・切替の後の 1 文字: 前の数 5（中断ではさらに増えた数）に対し今の数 1 → 増加にならず、以後も 1 > 1 が偽で出ない。赤。
  - 全文が一度に見える短い台詞（2 文字・再生時間 0）: 2 > 5 が偽。赤。
  - 台詞の途中の `\c`（scope 0）: 今の数 1 が前の数以上にならない。赤。
  - 観測できないフレームを挟む: 観測なしのフレームは記憶を据え置き、次のフレームで 1 > 5 が偽。赤。
  - 合図が先のフレーム: 合図のフレームで前の台詞の数が 1 へ減って記憶され、次のフレームで 1 > 1 が偽。赤。
- 守りの 3 本（全消去が先のフレーム・見えているバルーンの全消去と文字・消去の後に文字が無い）は HEAD でも通る（数 0 からの増加・現に可視なので表示しない・ゼロへの下降で隠す）。
- headless の表示層・本物の `TextLayerRuntime` を使う組み立ては既存の `balloon_visibility_phase_tests.rs` が既に使っている形で、`ClearAll` を headless のランタイムへ流しても描画の資源の無い scope は飛ばされるだけ（`actor.rs` の `apply_cue`）。

### (b) 消去の回数の意味が要件 1.x・2.x の全場面で正しい答えを出すか

- `state.rs` で内容を空にするのは `Clear` の腕（名指しの scope の `clear_content`）と `ClearAll` の腕（既にある全 scope の `clear_content`＋`reset_for_new_talk`）だけで、他に本番から内容を消す経路は無い（`crates/areka/src` の本番ファイルに `apply_cue`・`ClearAll` の直接の呼び出しは無い。切替は文字の層に触らない）。したがって「回数が変わった scope に今ある文字は、すべて最後の消去の後に置かれたもの」という不変条件は成り立つ。`ClearAll` が状態の無い scope を進めないことも、その scope の数が 0 のままなので結論に効かない。
- 届く順: `dola` の配りの輪（`runtime.rs` の、準備のできた cue ごとに全部の受け口へ順に配る輪）と受け口の並び（文字の受け口が 2 番目・表示の合図の受け口が 4 番目）により、先頭の `ClearAll` で「文字の受け口への投函 → `TalkStarted` の送出」、その後に最初の `Text` の投函となる。可視性の段は合図を全件取り出してから数えるので、最初の文字が数えられるフレームでは中断の掛け金は必ず解けている。逆の並び（全消去だけが先に入る・合図だけが先に取り出される）は 2 本のテストが押さえている。
- 場面ごとの答え:
  - 1.1〜1.3: 回数が変わったので 0 と比べて表示。隠れた理由は見ない。
  - 1.4: 観測なし・時刻なしのフレームは数も回数も据え置き、次に観測できたフレームで回数の変化に気付いて表示。合図だけが先・全消去だけが先の形でも、最初の文字が見えたフレームで表示。
  - 1.5: `Clear` が名指しの scope の回数だけを進めるので、その scope だけ表示し、他の scope は動かない。
  - 1.6: 表示の分岐が積む記録は今と同じ 1 件。
  - 2.1: 切替は文字の層に触らず回数も記憶も残るので、比べる相手は前の数のまま。
  - 2.2: 掛け金の条件は表示の分岐に残る。止めた台詞の残りが見えても回数は動かない。
  - 2.3: 新しい台詞の消去が来るまで回数は変わらない。
  - 2.4: 表示は現に不可視のときだけ、非表示は数が 0 のときだけなので、見えているバルーンへ同じフレームに全消去と文字が来ても行動は 0 件。
  - 2.5: 非表示の分岐は素の前の数で比べたまま変えない（比べる相手の 0 をここに使わないことを設計が明記している）。
- 既存の判定の網羅表は `seen()` を通して観測を組んでいる（`timeout_suppression_tests.rs` も `..seen(glyphs, visible)` で組む）ので、回数が一定の観測として今と同じ答えを返す。

### (c) どのファイルも 1,000 行以下に収まるか

- `balloon_visibility.rs` 895 → 約 930、`balloon_visibility_phase.rs` 588 → 約 600、`state.rs` 599 → 約 625、`balloon_visibility_phase_tests.rs` 919 → 装着の写しを道具へ寄せて減る。どれも上限の内側。新しい 3 ファイル（再表示のテスト・配線の段の道具・回数のテスト）は小さい。
- 道具のファイル名 `balloon_visibility_phase_test_support.rs` は steering の「最長の stem を採る」規則で配線の段の `test_support` と読め、親の `test_support` とは別のモジュールの中なので名前の衝突（E0428）は起きない。`balloon_visibility_phase_tests.rs` の `use super::*;` と `use super::super::{…}` の組み合わせも、明示の取り込みに `test_support` を含まないので曖昧にならない。

### (d) 要件の取りこぼし

- 要件 1.1〜1.6・2.1〜2.6・3.1〜3.8 のすべてが Requirements Traceability の表と、テストの一覧の「要件」の列に現れる。2.6 は既存の判定の網羅表が緑のままであることで守る。3.6 は台詞を変えず注記を事実へ改める形で満たす（実 GPU の往復の統合テストに本 spec と関係の無い揺れの余地を足さないという理由も書かれている）。
- 構造体の字面で `ScopeObservation`・`ScopeVisibility` を組んでいる箇所は、設計が挙げた 3 ファイル（`balloon_visibility_test_support.rs`・`balloon_visibility_forget_tests.rs`・`balloon_visibility_phase_tests.rs` の 2 か所と `Some(2)` の比較 1 か所）で全部であることを検索で確かめた。`TextLayerState` の等しさを比べる既存のテスト（同じ cue の列から同じ状態・表示系の cue で状態が変わらない）は、回数の数え方が決定論で表示系の cue で動かないので通る。`ActorTextState` を初期状態と比べるテスト（`clear_resets_actor_state_to_initial` ほか）は、回数を別の表に置くので影響しない。

## 重大な問題

なし。

作るものを変えるべき問題は見つからなかった。以下は判断を変えない覚え書きで、議題には上げない。

- 判定の記憶は、中断の掛け金で表示を見送ったフレームでも回数を更新する。これが取りこぼしにならないのは「最初の文字が数えられるフレームで掛け金は解けている」という届く順に依るためで、設計はそれを Revalidation Triggers（台本の先頭の全消去の置き方・配りの輪の順・受け口の並び）に書いてある。実装の段で受け口の並びを変える他の spec が来たときに見直せばよい。

## 設計の良い点

1. **状態の持ち方を変えて同じフレームで解いている。** 「回数が変わったら比べる相手を 0 にする」という 1 つの規則で、1 文字の台詞・全文が一度に見える台詞・台詞の途中の `\c`・3 つの隠れ方を区別なく塞ぎ、表示の時期を遅らせない。数と回数を 1 つの型（`GlyphObservation`）に束ねて同じ借用・同じ時刻で読ませ、記憶も同じフレームでだけ一緒に更新するので、片方だけが進む組み合わせを作れない。隠す側の分岐・`forget_scope`・`TalkStarted` の畳み込みには触れず、既存の約束（切替の後や中断の後に台詞の外で現れない）をそのまま残している。
2. **赤の段の選び方が正確である。** 判定だけのテストは新しい型を使うので修正の前にはコンパイルが通らない、という点を見抜き、修正の前から在る口だけで本物の文字の層と本物の観測の収集を通すテストを立てている。合図と全消去の届く順の両方の分岐、観測できないフレーム、見えているバルーンの明滅なしまで一覧にし、外せば赤になることの確かめ（回数を見ない・進めない・0 で運ぶ・非表示の分岐に 0 を使う）も具体的に書かれている。

## 最終判定

- **判定: GO**
- **理由**: 設計の主張（届く順・内容を消す経路・字面で組む箇所・テストのコンパイルと赤の成り立ち・行数）はすべて HEAD のコードで裏付けられ、要件 1.x・2.x の全場面で正しい答えを出し、要件 3.x の検証の道筋も具体的である。
- **次の段**: `/kiro-design-discussion areka-P0-balloon-reappear-short-talk` で設計を確定し、`/kiro-spec-tasks areka-P0-balloon-reappear-short-talk` でタスクを作る。実装では設計の「実装の順序」どおり、道具の移し替え → 再表示のテストを HEAD で走らせて赤の 7 本と守りの 3 本を記録 → 修正、の順を守る。
