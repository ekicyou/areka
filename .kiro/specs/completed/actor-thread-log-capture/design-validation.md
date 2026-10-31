# 設計の検証: actor-thread-log-capture

> 2026-10-31・`/kiro-validate-design`（対話なしで実行）。読んだもの: `spec.json`・`requirements.md`・`design.md`・`research.md`・steering（`structure.md` の areka-actor 節と log-capture-kit 節）・実コード（`crates/areka-actor/src/spawn.rs`・`spawn_hook_tests.rs`・`crates/log-capture-kit/src/capture.rs`・`probe.rs`・`lib.rs`・`tests/with_default_guard_test.rs`・`tests/workspace_scan/mod.rs`・`crates/areka/src/update/*`・`crates/areka/src/log_history.rs`）・依存のソース（`tracing-core` 0.1.36・`tracing-subscriber` 0.3.23・`tracing` 0.1.44）。

## 設計の検証の要約

設計は要件 1〜6 をすべて部品と試験に割り当てていて、本番の道を変えない形（窓でなければ子に何も据えない）を、`tracing` の公開の API だけで書けている。依存のソースを読み直して、設計の言う仕組みの主なところ（目印の型での問い合わせが健全なこと・本番の受け手が目印に答えないこと・番人への影響・行数・触るファイルの範囲）はどれも成り立つことを確かめた。重大な食い違いは無く、残る 2 点は設計の書き足しで済む。

## 確かめたこと（主張とソースの照合）

| 主張 | 確かめた所 | 結果 |
|---|---|---|
| `Dispatch::is::<T>()` は `downcast_raw(TypeId::of::<T>())` を通る | `tracing-core` `dispatcher.rs` 717 行 → `subscriber.rs` 500〜517 行（`downcast_ref` は null でなければ `&*(raw as *const T)`） | 成り立つ |
| 目印 `[(); 0x6172_656B]` で `self` の番地を返すのは健全 | 大きさ 0・整列 1 の型への参照は、null でない番地ならどこでも正しい。`downcast_ref` は読み出さない。長さは i686 の `usize`（〜0xFFFF_FFFF）にも収まる | 成り立つ |
| 本番の受け手（`Layered`）は目印に答えない | `layered.rs` 214・344 行（自分の型か、層→内側へ順に問う）・`Filtered`（自分・`L`・`F`・PLF の目印）・`fmt::Layer`（自分・`E`・`N`・`W`）・`Option`／`Box`／`Vec`／`reload`（中へ渡す・`reload` は `NoneLayerMarker` だけ）。`log_history.rs` の `HistoryLayer`・`CaptureLayer` は既定（自分の型だけ）。目印と同じ型を部品の型に持つ所は無い | 成り立つ |
| 本番は速い道のまま | `get_default` は `SCOPED_COUNT == 0` なら全体の受け手を直に渡す（379〜398 行）。子に何も据えなければ `SCOPED_COUNT` は増えない。本番のコードに `set_default`／`with_default` の呼出は無い（番人の例外表 4 件はどれもテスト） | 成り立つ |
| `set_default` は写しを持つ・登録し直さない | `set_default(&d)` は `d.clone()` を渡すだけ（276 行）。登録（interest の作り直し）は `Dispatch::new` の時だけ（472 行） | 成り立つ |
| 閉じた後の interest | 窓生まれのアクターが写しを持つ間、窓の受け手は登録表で生き続ける。`CaptureSubscriber` は `register_callsite` を上書きしない＝既定が `enabled`（真）を通って `always`。常駐 probe の `sometimes` と合わさって `sometimes`。閉じた後も `never` は焼かれない | 成り立つ（ただし下の議題 1） |
| 番人 `with_default_guard_test.rs` への影響 | 走査は `crates/log-capture-kit/src/` を除く全ソース・コメントは除く・左の区切りつき。`spawn.rs` の `(` つきの呼出が当たり、例外表 1 件＋件数 4→5 で緑。陳腐化の検査は「当たりがあること」を求めるので、設計の「関数名だけを渡す形にしない」は正しい。`KNOWN_DEV_DEPENDENTS` は陽性側の確かめだけで、`areka-actor` を足さなくても緑。新しい試験のファイルは `get_default` と `Dispatch::new` だけで走査語に当たらない | 成り立つ |
| 1,000 行 | 最大は `with_default_guard_test.rs` 772 行→約 780 行。`spawn.rs` 360→約 400・`capture.rs` 107→約 160・`spawn_hook_tests.rs` 60→約 85。新しい試験の 1 本も 6 本の試験で 1,000 行に届かない見込み | 成り立つ |
| 触るファイルが要件の境界の内 | 設計の File Structure Plan の 8 本は、要件の Boundary（Adjacent expectations の承認済みの表）と一致する。`install/` には触らない | 成り立つ |
| 赤に変わる既存のテスト | `worker_path_tests.rs` 201 行は確実（`update/procedure.rs` 187 行と `update/desk.rs` 206・213 行の両方で同じ名前）。ほかに同じ名前を数える `desk_reload_tests.rs` 224・548 行と `shell_balloon_switch_session_update_tests.rs` 223 行は、窓の中で `ask_reload_for_test`（`desk.rs` 554 行）を直に呼び、`update` のアクターを起こさないので緑のまま | 成り立つ |
| 全スレッドの捕捉を使う 3 本の実行ファイル | その 3 本は `capture` の窓を使っていない。同じ実行ファイルのほかの試験が窓の中でアクターを起こすと、その記録は全体の蓄積でなく窓へ行く（全体の蓄積が静かになる側の変化で、3 本の主張は自分のアクターの記録なので変わらない） | 影響なし |
| プロセスに 1 つだけのアクター | `spawn_actor` の呼び手（本番 11 ファイル）に `OnceLock`／`static` で 1 度だけ起こすアクターは無い＝あるテストの窓生まれのアクターがほかのテストの頼みの記録を拾う形は無い | 影響なし |

## 重大な議題

🔴 **重大な議題 1**: 窓専用の受け手で「何を渡し、何を既定のままにするか」と、閉じた後も `enabled` を真に保つ理由が正確でない
**懸念**: 設計は渡すメソッドを 6 つ（`enabled`・`new_span`・`record`・`record_follows_from`・`enter`・`exit`）と `event`・`downcast_raw` で挙げているが、残り（`register_callsite`・`max_level_hint`・`event_enabled`・`clone_span`・`try_close`・`current_span`）を既定のままにすることを書いていない。また「閉じた後に `enabled` を偽にすると `never` が焼ける」は、常駐 probe が `sometimes` を返す普段の形では起きない（`Interest::and` が `sometimes` に揃える）。本当に焼けうるのは probe を切った実行（`AREKA_LOG_CAPTURE_PROBES=off`）だけである。さらに `enabled` を真に保つと、窓生まれのアクターのスレッドでは閉じた後も `tracing::enabled!` が真のままになる。
**影響**: 実装する人が `register_callsite` や `max_level_hint` を別の形で上書きしたり、「閉じた後の手間を減らす」つもりで `enabled` を偽にしたりすると、要件 5.2（窓の決まりと、窓の中でアクターを起こさないテストの結果を変えない）を崩しうる。今の本体には `enabled!` で動きを変えるアクターは無い（`perf_thread_report`・`transition_diag` の判定は UI のスレッドだけ）が、その前提が設計に書かれていない。
**提案**: Components の WindowSubscriber に「上書きするのは上の 8 つだけで、ほかは `CaptureSubscriber` と同じく既定」と 1 行足す。`enabled` を真に保つ理由を「probe を切った実行で `never` を焼かないため」と正確に書き、「窓生まれのアクターのスレッドでは閉じた後も `enabled!` が真」を Revalidation Triggers（アクターの本体が `enabled!` で動きを変え始めたとき）に足す。
**対応する要件**: 2.5・5.2
**根拠の場所**: design.md「WindowSubscriber」の Responsibilities & Constraints（`enabled` は閉じた後も真のまま）・Revalidation Triggers

🔴 **重大な議題 2**: 要件 3.5／3.6 の「本番の形では子に受け手を据えない」に檻が無いことを、檻の無い所の一覧に書いていない
**懸念**: 試験 ⑸⒜ は判定の関数を、⑹ は「閉じた窓は写さない」を固める。しかし `spawn_actor` の中の配線の「`None` のときは何も据えない」は、⑸⒝（子と起こした側がどちらも「受け手なし」を返す）では区別できない。子へ `Dispatch::none()` を据えた形でも ⑸⒝ は緑になる。これは `research.md` 4.2 の、全体の受け手を後から据えると子の記録が永久に落ちる形そのものである。要件 5.1（全体の受け手を据えない）と 6.3（受け手を直に差さない）のもとでは、試験で区別する手が無い。設計の本文（「全体の受け手へ届くことそのものは…直には見ない」）には書いてあるが、Testing Strategy の「檻の無い所（明示）」には、閉じた後に捨てる 1 行しか載っていない。
**影響**: レビューする人が ⑸ を「3.6 を固めた」と読み違え、配線の書き換え（例: `unwrap_or` で今の受け手を据える形）を見逃す恐れがある。
**提案**: 「檻の無い所」に「`None` のとき子に何も据えないこと（3.6 の配線）。⑸⒜ の判定と ⑹ の閉じた窓の場合までが檻で、受け手なしを据える形は区別できない」と足す。配線は `if let Some(d)`／`Option::map` の 1 行だけの形に縛り、レビューでの見方（据える呼出は 1 か所・`None` の枝には何も無い）を Implementation Notes に書く。
**対応する要件**: 3.5・3.6・6.2 ⑸
**根拠の場所**: design.md「ControlTests」⑸⑹ と最後の段落・「Testing Strategy」の「檻の無い所（明示）」

（3 つ目は挙げない。窓の中でゴーストを起こすテストで、アクターのスレッドの記録がすべて窓の 1 つの `Mutex` へ積まれる手間は量が測られていないが、アクターの本体の `trace!`／`debug!` は受信 1 件ごとの数で、周期の刻みごとに出す所は見つからなかった（ticker は 0 件・seriko は失敗の時だけ）。要件 5.6 の全体テスト 1 回で、時間切れの赤が出ないかを見れば足りる。）

## 設計の良い所

1. **本番の道を 1 ビットも変えない判定を、公開の API だけで作った**。全体の受け手か差し替えかを見分ける公開の手が無い（`research.md` 4.1）という壁を、受け手そのものに目印を尋ねる形に置き換えた。`tracing-subscriber` 自身が `MagicPlfDowncastMarker`・`NoneLayerMarker` で同じ口を使っていることも確かめられた。目印を std の型だけで書いたので、`log-capture-kit` は leaf のまま、`areka-actor` の本番の依存も増えない。
2. **閉じた後の扱いを窓の印 1 つにまとめた**。同じ `AtomicBool` が「目印に答えるか」と「記録を積むか」の両方を切り替えるので、閉じた後に起きた孫は引き継がず（2.1）、閉じた窓の記録はほかの窓へ行かず（2.4）、長生きのアクターの記録も溜まらない（`research.md` 4.4）。閉じる時点を取り出しの後に置き、panic で抜けても閉じる形にしたのも、要件 1.5 の取りこぼしなしと両立している。

## 最終の判断

**判断**: GO

**理由**: 依存のソースで、判定の仕組みの健全さ・本番の受け手が答えないこと・interest が閉じた後も焼けないこと・番人と行数と触るファイルの範囲を確かめ、どれも成り立った。議題 1・2 はどちらも設計の書き足しで、作りは変わらない。

**次の手順**:
- 設計の討議（`/kiro-design-discussion actor-thread-log-capture`）で議題 1・2 の書き足しを決める。
- 決まったら `/kiro-spec-tasks actor-thread-log-capture` でタスクを作る。全体テスト 1 回のタスクでは、`research.md` 6 の「中」「低」の赤と、窓の中でゴーストを起こすテストの時間切れが無いかを見る。
