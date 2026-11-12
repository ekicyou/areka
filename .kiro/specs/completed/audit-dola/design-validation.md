# 設計の見直し: audit-dola

- 日付: 2026-11-06
- 見た物: `spec.json`（language=ja）・`requirements.md`・`design.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C18／C19 の行）・`kiro-validate-design` の決まり（`rules/design-review.md`）
- 進め方: 対話なしで判定まで出した（議題は設計の討議へ渡す）

## まとめ

設計は 88 件の指摘を 1 件ずつ「段・直し方・範囲の外・縛るテスト」の表に落としており、そのまま実装に入れる細かさがある。コードと突き合わせた主な主張（本番のアンカーは 0.0・範囲の外で期待が変わるテストは `compile_sheet_tests.rs` の 2 本だけ・1,000 行の計画・前の段の範囲の外が要件 6.3 の触れない所に入らない）は、どれも事実どおりだった。直すべき点は 3 つで、いちばん重いのは後の段に入る門が C19 の見込みと噛み合っていないこと。どれも設計の討議で決めれば片づく。

## コードとの突き合わせ（抜き取り）

| 設計の主張 | 確かめた所 | 結果 |
|---|---|---|
| 本番の cue のアンカーは必ず 0.0（件 1） | `crates/areka-ghost/src/dispatcher.rs:389-395`（トークの最初の `now` を起点に `(now - base) / 1000.0`）・`crates/areka-sakura/src/drive.rs:318`（最初の Tick をアンカーに刻む）。ほかに `with_absolute_start_time` を呼ぶ本番のコードは無い | 合っている |
| 150 ms＋800 ms の和は 0.9500000000000001・アンカー 1.0 では 1.95 を越える（7.3 の檻が直す前に赤） | python で計算: `0.15+0.8=0.9500000000000001`・`1.0+0.9500000000000001=1.9500000000000002 > 1.95` | 合っている |
| 範囲の外で期待が変わるのは `compile_sheet_tests.rs` の 2 本（333 行目付近・459 行目付近・補助 `relative_horizon` 381-386 行目） | 同じ行にある。ほかの f64 の和の期待（`compile_anchor_tests.rs:76・125`・`compile_font_tests.rs:142`・`drive_delivery_tests.rs:366`・本体の `balloon_visibility_lifecycle_e2e_tests.rs:124`）は、使う台本の値で格子の和と同じ値になる | 合っている |
| `DynamicValue` の `PartialEq` を手書きにしても外は困らない（#28） | 外の 14 ファイルは `Null`・`String`・`Array` を組むだけで、NaN どうしの不等に頼る所は無い。`DolaError`・`RuntimeError` を外で match する所も 0 | 合っている |
| 前の段の範囲の外（`areka-sakura/src/compile_sheet_tests.rs`・wintf の `ecs/dola/mod.rs`・`clippy-199-lints` の brief）は要件 6.3 と C18 のほかの 7 本の触るファイルに入らない | roadmap の C18 の行の「触るファイル」と約束 ⑴〜⑺ | 合っている。wintf は `log-capture-kit` を dev-dependencies に持つ＝依存は増えない |
| 1,000 行の番人 | `schedule_test.rs` 960 行・`sheet_test.rs` 783 行・`command_tests.rs` 830 行・`facade.rs` 522 行・wintf `ecs/dola/mod.rs` 503 行 | 数は設計の表どおり。`schedule_test.rs` を先に減らしてから足す順も筋が通る |
| 消す公開の口の外の呼び手 | `player.stop()` は `window_state_tests.rs:496` だけ。`EvaluatedValue` は wintf のテストが腕で使うだけ（`Display` は使わない）。`update`・`Interpolator`・`PlaybackState` の外の呼び手は 0 | 合っている |

## 直すべき点（3 件）

### 1. 後の段に入る門が「C18 のほかの 7 本の着地」だけで、C19 の見込みと噛み合っていない

- **問題**: 後の段は本体の `crates/areka/src/emo2_boot/frame/window_state_tests.rs`（件 2）と `emo2_boot/talk_lifecycle.rs`（要件 7.9）に触る。ところが roadmap の C19 の見込みには `runtime-module-untangle` があり、これは `emo2_boot/` を `runtime/` へ名前ごと移す spec で、brief の約束は「`crates/areka` を触るほかの spec と同じウェーブに置かない」。さらに C19 の `dola-yaml-backend` は本 spec の着地を待つ。後の段は C18 の着地の後にしか始められないので、そのまま進めると C19 の始まりと重なる。
- **影響**: 本体の同じファイルを 2 本が同時に触るか、`dola-yaml-backend` が C19 に入れずに v0.2.0 の道が 1 ウェーブ延びる。設計の「残る危うさ」は遅れだけを書き、ぶつかりは書いていない。
- **直し方**: 「2 つの段」の門に次のどちらかを決めて書く。⑴ 後の段は C18 の末尾（7 本の着地から C19 の着手までのあいだ）で走り切り、`/kiro-next` に「`audit-dola` の着地までは C19 を組まない、または `runtime-module-untangle` を入れない」と申し送る。⑵ C19 が先に始まったときは、`runtime-module-untangle` のセッションと申し合わせ、着地の後の新しい道（`crates/areka/src/runtime/...`）で直す。あわせて、後の段の頭の grep のし直しが道の改名（`placement-crate-split` による `crate::placement::` の書き換えを含む）も拾うことを書く。
- **出どころ**: 要件 6.1・7.9・8.2・11.2
- **設計の場所**: 「Architecture」の「2 つの段」・「Revalidation Triggers」・「残る危うさ」の「後の段の待ち」（突き合わせ: `roadmap.md` の C19 の行・`.kiro/specs/runtime-module-untangle/brief.md` の 64・117・137 行目）

### 2. 要件 7.4 の字面を、#9 の直しに合わせて読み替えている

- **問題**: 要件 7.4 は「格子に載らない値のときは、今と同じ値を出す」と書く。設計は #9（`absolute_end_time` も clamp を通す）のため、非有限や負の長さの合図では `absolute_end_time` が今と違う値になる、とし、7.4 を「格子へ丸めない」の意味と読む。理屈は通っており、本番の台本はこの形を作らない。ただし、要件の字面と違うことは、討議の決めとしてまだどこにも残っていない。
- **影響**: タスクごとのレビューや `/kiro-validate-impl` は要件の字面で見るので、「7.4 を破っている」と差し戻しが出うる。
- **直し方**: 設計の討議でこの読み方を開発者の決めとして確かめ、記録に残す（requirements.md は確定済みなので、討議の記録か design.md の決めの節に「7.4 は丸めないの意味・非有限と負の長さは #9 の clamp に従う」と書く）。檻は今の 2 本（`off_grid_values_keep_plain_sum` と `end_time_equals_occupancy_horizon_for_non_finite_durations`）のままでよい。
- **出どころ**: 要件 7.4・11.1・2.4
- **設計の場所**: 「Components and Interfaces」の「`CueSheet`・`to_talk_schedule`・`TimedSchedule`（形は変えない）」・`research.md` §11.3 の「要件 7.4 と #9 の読み合わせ」

### 3. `total_cmp` の並びの説明と檻が、計算で生まれる NaN と −0.0 を見ていない（#10）

- **問題**: 設計は「NaN を含む並びは正の NaN が昇順の末尾」と書き、檻は `f64::NAN`（符号なし）だけで組む。ところが x86 で計算から生まれる NaN（`inf - inf`・`0.0 / 0.0`）は符号の桁が立っていて（ビット `fff8000000000000`、python で確かめた）、`total_cmp` では先頭に並ぶ。また `total_cmp` は −0.0 を +0.0 より前に置く。今の `partial_cmp(..).unwrap_or(Equal)` では、この 2 つは「等しい」として台本の順（同じ時刻は先に書いたものが先）を保っていた。
- **影響**: 檻が本当に起こる形の NaN を縛らない。−0.0 の時刻が混ざると、同じ時刻の合図の順が台本の順から外れる（今の本番の台本は −0.0 を作らないが、その前提はどこにも書いていない）。
- **直し方**: 契約の文を「NaN は符号で両端に分かれる（負の NaN が先頭・正の NaN が末尾）」に直し、檻に `-f64::NAN`（または `f64::INFINITY - f64::INFINITY`）を混ぜる。−0.0 は、並べ替えの鍵を `x + 0.0`（−0.0 を +0.0 にそろえる）にして今の順を保つか、「前に並ぶ」を決めとして書くかを選ぶ。どちらも数行で済む。
- **出どころ**: 要件 4.1・4.2・11.1（#10）
- **設計の場所**: 「Components and Interfaces」の「`CueSheet`・`to_talk_schedule`・`TimedSchedule`（形は変えない）」の並べ替えの行・「指摘ごとの直し方」の #10 の行

## よい所

1. **表 1 枚で直し方が引ける**: 88 件それぞれに「段・直し方・範囲の外・縛るテスト」があり、要件 4.1（バグごとに檻 1 本）と 6.4（C18 は足すだけ）の当たりを行ごとに確かめられる。テストの読み替え（`update` → `tick`、`CuePlayer::ready()` → 記録する受け手）を前の段へ出し、後の段の差分を「公開の口を消す所」だけに絞った分け方も、相手の着地を待った後の手間を小さくする。
2. **件 1 の直しが小さく、測りに支えられている**: 和の決まりを `sum_on_ms_grid` の 1 本にまとめ、格子に載らない値は今の和のまま（時刻を丸めない決まりを守る）。本番のアンカーが 0.0 であること、範囲の外で期待が変わるテストが 2 本だけであること、直す前に赤になる檻のアンカーの選び方まで、コードと計算で確かめてある。

## 判定

- **GO**
- **理由**: 層の境目・依存の向き・C18 の約束（足すだけ・依存を足さない・触れない所）と食い違う所は無く、要件はすべて受け先がある。上の 3 件はどれも設計の討議で決めを足せば済み、作りの組み直しは要らない。
- **次の手**:
  1. 設計の討議（`/kiro-design-discussion audit-dola`）で、上の 1〜3 を議題にする（1 は門の決め・2 は読み方の確かめ・3 は契約と檻の直し）。
  2. 決めを design.md に写したら `/kiro-spec-tasks audit-dola` でタスクを作る（見立ては約 22 タスク・要件 11.1 で 1 本のまま）。

## 確かめていないこと

- clippy の 26 か所の数は走らせて数え直していない（`research.md` の測りをそのまま使った）。
- `#2`（周回の待ち）の式で `delay` に `time_scale` を掛けるかどうかは実装の段の細部として見ていない。
