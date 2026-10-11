# 設計レビュー: areka-P0-seriko-trigger-intervals

> 2026-10-10・`kiro-validate-design`（非対話・サブエージェント）。入力は `requirements.md`・`design.md`・`research.md`（§1〜7）・`brief.md`（棚卸㉓の分割の約束）・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`）。設計の file:line の主張は、ワークツリー `claude/seriko-trigger-intervals-490874` の実物（`crates/areka-seriko/src/{looper,parts,state,actor,table,timeline}.rs`・`crates/areka-parsers/src/shell/{model,decode}.rs`・`crates/areka-emo-text/src/state.rs`・`crates/areka/src/emo2_boot/talk_clock.rs`・`crates/areka-sakura/src/compile.rs`・`crates/dola/src/cue/sink.rs`・`doc/ukadoc-coverage/{ledger/assets.toml,roadmap-draft.md}`・`crates/ukadoc-survey/tests/consistency/spec_checks.rs`）を Grep／Read で突き合わせてから判定した。

## Review Summary

設計は「いつ始めるか」だけを新しい兄弟モジュール（`trigger.rs`・`talk.rs`）の純関数＋小さな状態に閉じ、再生そのもの（`frame_at`・`-1`・末尾の保持・再生中は始め直さない）と乱数の輪には触れない形で、要件 1〜11 を漏れなく構造に写している。設計が引く実物の根拠（`normalize_interval` が数値を捨てる・`apply` の `Changed`／`Unchanged`・`refresh` の 4 つの呼び手・`rebuild` の 1 回目が見えない部品も評価する・`RevealSchedule::extend_chunk` と `TalkClock::observe_cue` の式・`cue_target_of` の腕・台帳と `roadmap-draft.md` の行数）はすべて実物と一致し、brief の「同じウェーブで触らない約束」も守られている。ただし、文字の写し（`TalkFeed`）が `Clear`／`ClearAll` で未リビールの文字を残す決めは、台本ごとに先頭へ `ClearAll` が前置される実物（`compile.rs`）と組み合わさると `times` の単調性（`partition_point` の前提）を自分で破り、新しいトークの途中で口が勝手に動く見える不具合になる。また、`Armed` を構える条件の門が本文に明記されておらず、字面どおりに実装すると 3 語の無い検体（emo2）の記録が変わる（要件 8.3 違反）。どちらも設計の文言を直せば済む局所の修正であり、構造の組み替えは要らない。

## Critical Issues

### 🔴 Critical Issue 1: `Clear`／`ClearAll` で未リビールの文字を `TalkFeed` に残すと `times` の単調性が破れる（新しいトークの途中で口が勝手に動く）

- **Concern**: 設計は `TalkFeed::restart_chain`（`Clear`／`ClearAll`）を「継ぎ目を初期化するだけ・数えた序数は捨てない」と定め、`talk.rs` の Risks で「`\c` の直後に口が最大 1 区切り分だけ余分に動きうる」と小さく見積もっている。しかし実物の `crates/areka-sakura/src/compile.rs`（「冒頭 ClearAll 前置」の段）は**内容 cue を持つ全部の台本の先頭に `ClearAll`（`at = 0.0`）を前置する**。中断された前のトークの、届いたが未リビールの塊（r が例えば 12.0〜12.5 秒）が `times` に残ったまま、新しいトークの `Text` が `r_0 = at = 0.0` から追記されると、`times` は `[…, 12.5, 0.0, 0.05, …]` と単調でなくなり、`revealed_until` の `partition_point` の前提（設計 Postconditions「`times` は単調非減少」）が崩れる。さらに `TalkEpoch` が新しいトークで前へ飛ぶので、残った旧文字の壁時刻 `epoch_new + 12.5 s` は**新しいトークの 12.5 秒後**に当たり、そこで口が意味なく動く。文字の層（`crates/areka-emo-text/src/state.rs` の `Clear`／`ClearAll` の腕）は `clear_content()` で schedule ごと捨てるので、この現象は seriko の写しにだけ起きる＝「文字の層と同じ式」（要件 6.4・裁定 T2）から外れている。
- **Impact**: 口パクの中核の不変条件が台本 1 本ごとに破れる。檻（`talk_tests.rs`）が同じ入力列で `RevealSchedule` と同じ答えを固定しても、`Clear` を挟む入力で初めて食い違うため、設計どおりに実装すると実機（要件 11）で「新しいトークの途中で口が 1 回余計に動く」として現れ、原因が時計の差（研究項目 11）と見分けにくい。
- **Suggestion**: `restart_chain` を「`Clear`／`ClearAll` の `at` までに現れていない文字（`r_i > at`）を `times` の末尾から捨て、`last` を `None` に戻す」に改める（文字の層の「未リビール分を含む全消去」の写し・`ClearAll` は全スコープ）。これで ⑴ `times` の単調性が保たれ ⑵ 現れなかった文字で口が動かない（要件 4.1「現れる文字」に忠実）⑶ Risks の「`\c` の直後の 1 回」も消えるので台帳の note からも外せる。あわせて `Armed::advance_talk` は `seen = max(seen, now_seen)` とし、切り詰めで `now_seen < prev_seen` になった刻みは鳴らさない（シェルとバルーンの両 slot に `talk` が在る場合の防御）。不変条件「`base + times.len()` は届いた文字の総数」は「届いて現れた（現れる予定の）文字の数」に書き換える。`talk_tests.rs` に「中断→`ClearAll`→新しい塊」と「`\c` の後の塊」の 2 本を足し、`RevealSchedule` と同じ可視数になることを固定する。
- **Traceability**: 要件 4.1・4.3・4.6・6.4・9.4（`Clear` の継ぎ目）・10.1（note）。
- **Evidence**: design.md「文字の写し（`talk.rs`）」の `restart_chain`／Postconditions／Risks、「System Flows › 文字が届いてから」、「台帳の書き方」の `\c` の 1 文、research.md §7.4「`Clear` と序数」。実物: `crates/areka-sakura/src/compile.rs` の `ClearAll` 前置、`crates/areka-emo-text/src/state.rs` の `Clear`／`ClearAll` の腕（`clear_content()`）。

### 🔴 Critical Issue 2: `Armed` を構える門が本文に無く、字面どおりだと 3 語の無い表（emo2）でも `Armed` と `debug!` が生まれる（要件 8.1・8.3 違反）

- **Concern**: 設計は「3 語の無い表は `has_triggers = false`…`armed` は作られない」（looper の「門」）と結論だけ書くが、構える 3 か所の手順にその条件が入っていない。⑴ `refresh`: 「面が在る slot で `armed` に無ければ `Armed::arm` を入れて `debug!`」—`refresh` の早期戻りは `is_continuous()` だけで、emo2 は動く部品を持つ（`has_animated_parts` が真・`looper_parts_emo2_tests.rs` が前提）ので通過する。⑵ `on_tick` (3) ⑴: 「`armed` が無ければ `now_ms` で構える」—(3) の門は `has_playback || has_always || moving_visible` でも通るので、`random` が再生中の emo2 の slot で `Armed` が生まれる。⑶ 部品の 2 段目 `arm(part)`: 「入れ物の `armed[part]` が無ければ `Armed::arm` で構える（`debug!`）」—見える部品全部に対して、その部品が 3 語を 1 本も持たなくても構える。いずれも新しい `debug!`（「trigger 面に入った」）が emo2 の記録に増え、`armed` の表が育つ。
- **Impact**: 要件 8.3「同梱の検体の見た目と記録を変えない」と 8.1「刻みごとの仕事を増やさない」が、実装の読み方しだいで破れる。既存の emo2 の檻が記録の文言まで固定していなければ気付かれず、実機の記録の突き合わせ（手順 5「`emo2` の素の面の記録が前の実行体と同じ」）で初めて赤になる。
- **Suggestion**: 構える条件を 1 文で明記する。「`Armed` を構えるのは、**その slot の表（一番上は `top_has_trigger(table, sid)`・部品は `part_animations(part)` に `Runonce`／`Periodic`／`Talk` が 1 本でも在る）に 3 語が在るときだけ**。部品の 2 段目は `table.has_triggers()` が偽なら呼ばない（`rebuild` の外側の輪は 1 回で終わる＝今の形と同じ）」。あわせて `top_has_always` と `top_has_trigger` は刻みごとに同じ `animations(sid)` を 2 度走査しないよう、1 回の走査で両方の真偽を返す形（または表を組むときに面ごとの印を焼く）にする。檻は `looper_trigger_tests.rs`・`parts_trigger_tests.rs` の「3 語の無い表で `armed` が空のまま」に「`random` が再生中でも・動く部品が見えていても空のまま」を足し、emo2 の既存の檻の記録の件数が変わらないことで固定する。
- **Traceability**: 要件 8.1・8.3・7.3（記録を増やさない）・5.6。
- **Evidence**: design.md「一番上の配線 › `refresh`」「`on_tick` の (3) ⑴」「門（要件 8.1）」、「部品の配線 › 2 段目 `arm(part)`」。実物: `crates/areka-seriko/src/looper.rs` の `refresh`（`is_continuous()` の早期戻り）と `on_tick` (3) の門、`crates/areka-seriko/src/table.rs` の `has_animated_parts` の求め方。

## Design Strengths

- **「いつ始めるか」を 1 つの型に畳み、再生の決まりを一切触らない**: `Armed`／`poll` を一番上（`looper.rs`）と部品（`parts.rs`）が同じ関数で呼ぶので、要件 5.4（一番上でも部品でも同じ決まり）が構造で成り立ち、`-1`・末尾・再生中の扱い（5.1〜5.3）は既存の `frame_at` の経路をそのまま借りる。乱数の輪（`should_fire`）の前で分岐するので 5.7・8.2 も構造で担保される。
- **「面に入った」を `on_surface_changed` が `Armed` を捨てることで表す**: `refresh` の署名も新しい口も足さずに、`apply` の `Changed`／`Unchanged`（実物 `state.rs`）と `refresh` の 4 つの呼び手（実物 `actor.rs`）の既存の順序だけで要件 2.1〜2.6 を満たす。研究の R1／R2 より小さく、`state.rs`・`timeline.rs` に触らないので brief の約束にも余裕がある。2 段の `rebuild` は 1 段目の順と乱数を変えず、2 段目で載ったコマが見せる子を同じ刻みで評価する（1 フレーム遅らせない）点も実物の `rebuild` の輪の形に沿っている。

## Final Assessment

**Decision: GO（条件付き）**

**Rationale**: 構造（引き金の状態と文字の写しを兄弟モジュールへ・再生と乱数は不変・2 段の `rebuild`）に欠陥は無く、要件の全項目に対応と檻の置き場があり、約束のファイルにも触れない。上の 2 件はどちらも設計本文の数行の修正（`restart_chain` の決まりの改め・構える門の明記）で閉じ、型や依存の向きを変えない。ただし 1 件目は口パクの中核の不変条件を破る実害のある決めなので、設計ディスカッションで必ず反映してからタスク生成へ進むこと。

**Next Steps**:
1. 設計ディスカッション（`kiro-design-discussion`）で Critical Issue 1・2 を design.md に反映する（`talk.rs` の `restart_chain`／Postconditions／Risks・台帳の note の `\c` の 1 文・looper／parts の構える門・`top_has_trigger` の走査の 1 本化）。
2. 反映後に `/kiro-spec-tasks areka-P0-seriko-trigger-intervals` でタスクを生成する。

## Minor Observations（議題にしない・実装で拾う）

- `TalkEpoch` の内部が `epoch_s: Option<f64>`（秒）で、本文の式は `epoch_ms` と書かれている。どちらでもよいが、`wall_ms = floor((epoch + r) × 1000)` は浮動小数の誤差で t_k より 1 ms 小さい値を返しうる。1 ms は時計の分解能の下なので要件 6.1 には触れないが、`epoch` を ms（f64）で持ち `round` で写す方が「切り捨て」の説明も要らず単純。
- `arm`／`show` の `talk` の数えは、構えた時点で `TalkEpoch` が未確立（`talk_time` が `None`）だと `None` のまま固定され、その slot では次の構え直しまで `talk` が鳴らない。本番は `observe_cue` が分類の前に呼ばれるので `\s` の cue 自身が起点を作り到達しないが、`TalkWindow` を作る刻みで `talk` が `None` なら `base = now_seen` で遅延生成する 1 行を置くと防御になる。
- `talk` の開始の `debug!` は `every = 3`・50 ms/字で約 150 ms に 1 行。`logging.md` の「高頻度は `trace!`」との境目に近いが、実機の手順 5 がこの行を grep するので `debug!` のままでよい。長い台詞で記録が重いと分かったら `trace!` へ下げ、手順の `RUST_LOG` を `areka_seriko=trace` にする。
- 1,000 行: `table.rs` 779→約 840・`looper.rs` 682→約 800・`parts.rs` 625→約 735 の見積もりは実物の行数と合う。`parts.rs` は 2 段の `rebuild` と `arm` の閉包で見積もりを超えやすいので、900 行を越えそうなら `rebuild` を兄弟モジュールへ出す。
- 検体の面 9103 の `animation2.interval,Talk,3`（大文字混じり）は読み手で `Other("Talk")`、表では先頭の語が `talk` でないので `warn!` でなく既存の `debug!`。手順 5 の「`WARN … 数値が無効` が 9103 で 2 行」はこの読みと整合している（`talk,abc`・`periodic,0` の 2 行）。
- `roadmap-draft.md` の `[briefs].count = 48`・本 spec の `owner_count = 1`・`seriko-interval-combinations` の行が無いことは実物で確認した。`areka-P0-seriko-interval-combinations`・`areka-P0-seriko-script-triggers` の spec ディレクトリは存在する（見張りの腕 b を満たす）。
