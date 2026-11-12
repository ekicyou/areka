# 設計の検証: audit-kanade

- 日付: 2026-11-06
- 対象: `design.md`（e8ac10bb 時点）・`requirements.md`（要件 1〜9）・`research.md`（§3・§10・§11）
- やり方: `kiro-validate-design`（分析 → 重い問題 → 良い所 → GO／NO-GO）。対話なしで判定した。
- 確かめた事実（コードと突き合わせた）:
  - 指摘 #1〜#96 は「指摘ごとの直し方」の表に全部ある（抜け 0 件。#63 は「直さずに残す 1 件」の表にも出るので 2 回）。
  - 要件 1.1〜9.10 は「要件の対応」の表に全部ある（抜け 0 件）。
  - 1,000 行の番人の表の「今」の行数は、13 ファイルとも実物と一致した（`actor_tests.rs` 983・`steady.rs` 973・`events.rs` 943・`steady_flow_tests.rs` 931・`steady_test.rs` 924・`runtime.rs` 834 ほか）。範囲の外で触るファイル（`ticker.rs` 307・`user_break.rs` 395・`balloon_visibility.rs` 565・`frame/wiring.rs` 419）も上限から遠い。
  - #16: `target\audit-kanade\dead-code-forced.txt` に、許しを外したときの警告が `types.rs:136` の `ClosePending.reason` 1 件だけと残っている（設計の主張どおり）。
  - #6: `BalloonVisibilityState.talk_id` は私的な欄（`balloon_visibility.rs:451`）、`Emo2Wiring.balloon_visibility` は `pub(in crate::emo2_boot)`（`frame/wiring.rs:120`）、`frame/wiring.rs:36` は `crate::placement::zorder_group_ledger` を名指す。読み口を 2 つ足す要がある、という設計の測りは正しい。`crates/areka` で `UserBreak {` を組むのは `user_break.rs` 1・`user_break_tests.rs` 5・`shell_box_handler_tests.rs` 1 で、要件 6.1 の例外の 3 ファイルに収まる。
  - #7: `TickerMsg` を `match` するのは `areka-ghost` の 2 つの輪だけで、`crates/areka` は `Close` を送るだけ（変わらない、は正しい）。`spawn_relay` は `B: From<A>` を求めるので `impl From<MonotonicMs> for TickerMsg` が要る、も正しい。終了の順は「kanade の join → … → 刻み役の Close」なので、中継が刻み役の送り端を持っていても終わりが詰まらない。
  - #33: `run_shiori_loop` は私的、`IDLE_INTERVAL` は公開で結合テストが読む（設計どおり）。

## まとめ

設計は 96 件の全部に直し方・縛るテスト・束を当て、新しい形を引く 4 件（#1・#7・#6・#33）には採らなかった案と理由まで書いている。大原則（時刻は正確に・1 フレーム遅らせない・失敗は記録して返す・依存を足さない）とも合っている。ただし #7 の結線が `areka-ghost` の公開の型と 5 つのテストのファイルに及ぶことが抜けており、#1 の送る点では検査と取り除きの順が書かれていない。どちらも直し方ははっきりしているので、設計の討議で直せば実装へ進めてよい。

## 重い問題（3 件）

### 問題 1: #7 の中継の取っ手の行き先が抜けている（`GhostHandles` と 5 つのテスト）

- **中身**: 設計は「中継の取っ手は `GhostRuntime` に持ち、今の 2 本と同じ順で畳む」とだけ書く。だが `GhostRuntime::into_parts` は全部の取っ手を公開の型 `GhostHandles`（`runtime.rs:178`・欄は全部 `pub`）で渡しており、新しい取っ手もここへ渡さないと、手で畳む道で join されない糸が残る。`GhostHandles` に欄を足すと、欄を全部書いて分解している 5 つのファイル（`src/runtime_tests.rs:144`・`tests/ghost/spine_e2e_test_s2_connect_failure.rs:67`・`spine_e2e_test_s3_helper_liveness_detected.rs:149`・`spine_e2e_test_s6_full_disconnect.rs:89`・`sylphya_integration_test.rs:132`）がコンパイルできなくなる。この 5 つは「範囲の外で触るファイル」の表に無い（`sylphya_integration_test.rs` は #62 の注の行としてだけ載る）。
- **なぜ困るか**: 要件 3.2（壊れる呼び出し元・テストを全部直す）と 6.7（範囲の外は要る変更だけ・表が正本）に対して表が足りない。`shutdown` の段の番号（今は 1〜10）と doc の「全 `ActorHandle`」の説明も変わる。正規の終了の道（要件 5.3）に関わる所が実装係の判断任せになる。
- **直し方**: 「#7」の節と表に次を足す。`GhostHandles` に `choice_wake_relay: Option<ActorHandle>`（`TickerMode::Real` のときだけ `Some`・`ticker` と同じ形）を足し、`shutdown` では down-relay の後に join する。表に `runtime_tests.rs` と spine の 4 つのファイル（分解の行に `choice_wake_relay: _` を足すだけ）を加える。持ち主は `audit-ghost-catalog`（着地済み）なので申し合わせは要らない。
- **要件**: 3.2・5.3・6.7・9.5
- **根拠**: design.md「範囲の外で触るファイル」の表・「#7」の節の「結線（`crates/areka-ghost/src/runtime.rs`）」

### 問題 2: #1 の送る点で、検査と取り除きの順が書かれていない

- **中身**: 今の `round_trip_raw` は、先に `replace_absent_markers` で Reference を直し、その後で `allowed` を判定する（`actor.rs:637` と `:664`）。設計はこの輪を `clean_references` に広げて Reference から 3 字を取り除くとするが、順を変えるとは書いていない。今の順のままだと、`Sstp`・`Property` の Reference に 3 字があっても先に取り除かれ、送る点の `validate_notify` は何も見つけずに送ってしまう。設計の「`Sstp`・`Property` の今の検査は 1 つも削らない」と食い違い、送る点の守りは実質なくなる（入口の検査は残るが、二段目の意味が消える）。
- **なぜ困るか**: 要件 9.2 は二段目の守りとその扱いを設計で決めるとしている。外から頼まれた名前の Reference を「断る」のか「取り除いて送る」のかが、コードの順というたまたまの事情で決まってしまう。縛るテスト 3 本のどれもこの場合を見ていないので、どちらに転んでも緑になる。
- **直し方**: 「#1」の節に順を書く。⑴ ID の 3 字と `allowed`（`Sstp`・`Property` は生の Reference で `validate_notify`）を先に判定し、断るものは今の `event_id_not_allowed` で断る。⑵ 通ったものだけに `clean_references`（欠番の印の置き換えと 3 字の取り除き）をかける。`actor_header_guard_tests.rs` に「`EventId::Sstp` の Reference に LF → 偽の shiori は何も受けない・`Failed(Internal)`」を 1 本足す（順を逆にすると赤）。
- **要件**: 9.2・4.1・5.2
- **根拠**: design.md「#1」の節の「二段目」と「縛るテスト」・「安全の考え」

### 問題 3: #6 は要件 6.1 の例外の外の 2 ファイルに及ぶので、開発者の裁定が要る

- **中身**: 設計は、押されたバルーンのトークの番号を運ぶのに `crates/areka/src/emo2_boot/balloon_visibility.rs` と `emo2_boot/frame/wiring.rs` に読み口を 1 つずつ足すとした。測りは正しい（上の「確かめた事実」）。だが要件 6.1 の例外に挙がるのは `input_events/`・`log_history.rs`・`sakurascript_log_tests.rs` だけで、要件 6.5 は「6.1 を破らないと直せないときは直さず議題にする」と決めている。設計はこれを「設計の討議で開発者に確かめる」と正しく書いているが、裁定と `placement-crate-split` の承知がそろうまで T17 は始められない。あわせて、`talk_id` が `None`（表示の側が観測できない）のときは止めない、という決めは、今は止まる押下が止まらなくなる場合を生む。どの構成で `None` になるか（`Emo2Wiring` が無い・まだ最初のトークの合図を畳んでいない）を開発者に見せる要がある。
- **なぜ困るか**: 裁定なしに進めると要件 6.1・6.5 を破る。裁定が「不可」なら #6 の形（運び道）を考え直すことになり、T16 の後ろ、T18 の前の並びにも響く。
- **直し方**: 設計の討議の議題にする。推奨は「例外を 2 ファイル（読み口を足すだけ・`crate::placement::` の行に触らない）へ広げ、T17 の前に `placement-crate-split` のセッションへ知らせる」。裁定を要件 6.1 の例外の列と design.md の申し合わせの節に書き込む。`None` は止めない、という決めも同じ議題で承認を取る。
- **要件**: 6.1・6.5・6.6・9.9
- **根拠**: design.md「`placement-crate-split` との申し合わせ」・「#6」の節・「要件の対応」の 6.1 と 6.5 の行

## 良い所

1. **全件に打ち手と束がある**: 96 件の 1 行ずつに直し方・範囲の外・縛るテスト・束がそろい、1,000 行の番人の表は実物の行数と一致している。減らす束（T1〜T4）を足す束より前に置いた順の決めも、上限の近いファイルを越えさせない筋が通っている。
2. **#7 が大原則どおり**: 期限が決まる時点の分かる kanade がプッシュで頼み、時計を持つ刻み役だけが本物の時刻を読んで知らせる。丸めも 1 フレームの遅れも無く、毎秒の刻みでの判定を拾い手として残すので、起こしの線が無い構成でも今と同じに動く。採らなかった 5 つの案の理由も開発者の決め（プッシュかプルか・時刻は正確に）に結び付いている。

## 判定

**GO（条件つき）**

- **理由**: 層と依存の向きは今のまま、依存も `Cargo.lock` も変わらず、要件と指摘の抜けは 0 件。3 件の問題はどれも局所で、設計書に数行と表の行を足せば直る。設計を作り直す要は無い。
- **次の手**:
  1. 設計の討議（`/kiro-design-discussion audit-kanade`）で問題 1・2 を設計書へ反映し、問題 3 を議題として開発者の裁定を取る。
  2. 裁定を要件 6.1 と design.md に書き込んでから `/kiro-spec-tasks audit-kanade` へ進む。
  3. T17 に入る前に `placement-crate-split` のセッションへ知らせる（要件 6.1・6.6）。

## 確かめなかったこと

- #2〜#5・#8〜#96 の各行の直し方は、表の筋だけを読み、コードの全部とは突き合わせていない（行番号は `research.md` §3 の時点のもの）。
- #6 の「入力の段は表示の更新の段より先に回る」は `research.md` §11 の記述を信じ、ECS の段の並びまでは読んでいない。
- clippy・テストは回していない（読むだけの検証）。
