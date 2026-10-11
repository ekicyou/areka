# Brief: areka-P0-seriko-trigger-teardown-gaps

> 起票: 2026-10-10（`areka-P0-seriko-trigger-intervals` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-seriko-trigger-intervals/tasks.md` の Implementation Notes の 3.1・4.1・5.1・9.2 の ⑧ と、検証（`/kiro-validate-impl`）が拾った軽い点の ⑴〜⑶・⑹。どれもコードの読みと決定論のテストから出た件で、実機で絵の誤りを見たものは無い。区分 B（バグ）。

## Problem

- **ゴーストの作者**: interval の 3 語（`talk,数値`・`runonce`・`periodic,数値`）で始まる animation の「いつ構え直すか・いつ捨てるか」が、一番上の面に書いたときと、element定義で置いた部品の面に書いたときとで食い違う場面がある。同じ pattern定義でも、書いた場所によって、バルーンの窓を閉じて開いた後の絵と、`periodic` の周の起点が変わる。
- **開発者**: 面を離れた瞬間に再生中だった animation の「停止」の記録が出ない。実機の記録から「いつ止まったか」を追えない。
- 正典（ukadoc）は、この後始末の場面について何も書いていない。`seriko-trigger-intervals` は場面ごとに areka の決めを置いたが、決めどうしが揃っていない所と、決めを固定するテストが無い所が残った。

## Current State

2026-10-10 時点（`seriko-trigger-intervals` のブランチ・コミット `84ca70e6`）。引き金の状態は `crates/areka-seriko/src/trigger.rs` の `Armed`（見え始めの時刻・`runonce` を鳴らした印・`periodic` の周・`talk` の数え）で、一番上の面は `looper.rs`／`looper_trigger.rs`、部品は `parts.rs`／`parts_trigger.rs` が持つ。

1. **窓の知らせだけで変わるバルーンの面の番号では構え直さない**。`actor.rs` の `on_stage` は、表示の層からの窓の知らせ（`StageNote::Balloon`）を `ScopeStates::note_stage` で覚えてから `LoopRuntime::refresh` を呼ぶ。`\b[番号]` を一度も受けていないスコープでは、この知らせの面の番号がそのままバルーンの面の番号になる（`state.rs` の `balloon_face`）。番号が変わっても `LoopRuntime::on_surface_changed` は呼ばれないので、引き金の状態と再生は前の番号のときのまま残る（`runonce` は鳴り直さず、`periodic` の起点も動かない）。一番上の `always` にも同じ死角があり、`seriko-trigger-intervals` より前からの性質。
2. **窓の新しい出番で、一番上と部品の扱いが違う**。新しい出番の知らせで `on_stage` は `LoopRuntime::drop_finite` を呼ぶ。部品の側（`parts.rs` の `PartClocks::drop_finite`）は 3 語の時計を捨てて引き金の状態を隠すので、次に現したときに `periodic` の起点が置き直る。一番上の `Armed` には触らない。閉じた知らせを見ないまま新しい出番になった場合、部品の `periodic` は起点が動き、一番上の `periodic` は元の起点のまま続く。
3. **窓が閉じたとき、部品だけが末尾のコマと再生を失う**。窓が閉じると部品の 3 語の時計を全部捨てる（`parts.rs` の `close`）。捨てる中には、`-1` で終わらない animation が末尾で保っているコマ（`PartAnim::Residual`）と、再生の途中の時計も入る。そのため、`-1` で終わらない部品の `runonce` は、閉じて開いた後に末尾のコマが戻らない（`runonce` の印は残るので鳴り直しもしない）。一番上の面は、閉じても末尾のコマを保ち、再生中の `runonce`・`periodic` は最後まで進む。閉じている間は絵が出ないので、差が見えるのは開き直した後。
4. **停止の記録が出ない経路が 2 つある**。
   - 面を離れた瞬間に再生中だった animation は、`LoopRuntime::on_surface_changed` が再生の表から黙って落とす（「停止」も「末尾残留」も記録しない）。3 語に限らず抽選の animation も同じで、`seriko-trigger-intervals` より前からの振る舞い。
   - 部品の評価の 2 段目で鳴った部品が、同じ評価の後の回で別の 3 語のコマに隠されると、「鳴らした」の `info!` が 1 行出たまま、その刻みの最後に時計と状態を捨てる（絵には出ない・まれ）。捨てた記録は `debug!`（「seriko: part 引き金の時計を捨てた」）。
5. **`\b[番号]` での切り替えを固定するテストが無い**。バルーンの面の番号を `\b[番号]` で替えたときに 3 語が構え直ること（`actor.rs` がバルーンの面にも `on_surface_changed` を呼ぶ経路）は、シェルの面の切り替えと同じ関数を通るが、バルーンの面でそれを踏むテストは無い。窓の開け閉めのテストは在る。
6. **同じ決まりが何か所にも書いてある**（検証が拾った片付け）。
   - 「3 語のどれかか」の述語が 3 か所: `looper.rs` の `is_trigger_word`・`parts.rs` の `is_trigger`・`table.rs` の `has_triggers` を作る所の `matches!`。
   - 窓の開け閉めを引き金の状態へ写す `match (open, state.is_visible())` が、`looper_trigger.rs` の `arm_slot` と `parts_trigger.rs` の `fire_part_triggers` に同じ形で在る。
   - `Armed::arm` の引数 `at_ms` は `Option<u64>` だが、呼ぶ 2 か所はどちらも `Some` を渡す。
7. **areka が決めた所（裁量）の一覧**。どれも正典が黙る所で、今は設計書とコードのコメントにだけ書いてある。網羅台帳の備考（`doc/ukadoc-coverage/ledger/assets.toml` の `runonce:1`・`periodic_2c_6570_5024:1`・`talk_2c_6570_5024:1` の 3 行）には載っていない。
   - 閉じた窓で構えた面の `runonce` は、最初に開いた時刻で 1 回鳴る。開き直しでは鳴らない。
   - 外側の面が替わっても見え続けた部品は構え直さない（`runonce` は鳴り直さず、`periodic` は元の起点で続く。新しい面に居ない部品は捨てる）。
   - 面が隠れた（`\s[-1]`・`\b[-1]`）ときは時計も引き金の状態も捨てる＝戻ると `runonce` がもう 1 回鳴る。窓が閉じただけのときは `runonce` の印を残す。
   - 後から見えた部品の起点は「見えたと分かった刻みの時刻」で、外側のコマが替わった正確な時刻ではない。表を差し替えた後の構え直しも、次の刻みの時刻になる。
   - 上の 2・3 の、一番上と部品の差。

## Desired Outcome

- 一番上の面と部品の面で、3 語の構え直しと捨て方が同じ決まりになる。揃えない所は、理由つきで「areka の決め」として記録に残る。
- `\b[番号]` を受けていないスコープでバルーンの面の番号が変わったときの扱いが決まり、テストで固定される（一番上の `always` も同じ決まりに従う）。
- 面を離れて止まった再生に、止まったことが分かる記録が 1 行残る。絵に出なかった「鳴らした」の記録が、読む人を誤らせない形になる。
- 上の 7 の一覧の各項が、「網羅台帳の備考に 1 文足す」か「振る舞いを変える」かのどちらかに決まる。変えたものは決定論のテストで固定する。
- 同じ決まりの写しが 1 か所になる（振る舞いは変えない）。

## Approach

要件の段で、上の 1〜4 と 7 の各項を 1 件ずつ「揃える／今のままを決まりとして記録する」に分ける。目安は次のとおり。

- 利用者に見える差（3 の末尾のコマ・2 の周の起点）は揃える向きで検討する。揃える先は一番上の面の扱い（閉じても末尾のコマを保つ・再生中は最後まで進む・窓の新しい出番では起点を動かさない）が第一の候補。部品の時計を閉じたときに捨てているのは、回数つきの `always` と同じ関数で片付けているためなので、3 語だけ別に扱えるかをコードで確かめる。
- 後から見えた部品の起点（7 の 4 つ目）は、「時刻は正確に扱う」の原則（待ち時間を丸めない・更新が遅れたら過ぎた時間の分だけ進める）に照らして、外側のコマの時刻を起点にできるかを設計で見る。できないなら理由を記録する。
- 記録（4）は、落とす所で `info!`（`talk` は `debug!`）を 1 行出す。既存の再生の終わりの記録と同じ文言の型に合わせる。
- 片付け（6）は最後のタスクでまとめて行う。述語は 1 か所（`table.rs` の `LoopTrigger` の隣）に置き、ほかはそれを呼ぶ。

## Scope

- **In**: 3 語の引き金の状態と時計の、構え直し・隠す・捨てるの決まりの見直し（一番上の面と部品）。窓の知らせだけで面の番号が変わる場面の扱い（一番上の `always` を含む）。面を離れたときの停止の記録。`\b[番号]` の切り替えのテスト。同じ決まりの写しの片付け。網羅台帳の 3 行の備考への追記。`doc/COMPAT_ARCHITECTURE.md` §8 への記録（決めを変えた・足したとき）。
- **Out**: 3 語を「いつ鳴らすか」の決まりそのもの（`seriko-trigger-intervals` で確定）。文字の時刻の写しと起点（`seriko-talk-clock-fidelity`）。抽選の境界の判定（`seriko-lottery-ended-play-skip`）。見えていない部品の抽選（`seriko-rebuild-hidden-lottery`）。`+` の組み合わせ（`seriko-interval-combinations`）。`yen-e`・`never`・`\i[ID]`（`seriko-script-triggers`）。

## Boundary Candidates

- 引き金の状態の寿命（`trigger.rs` の `Armed` の `arm`・`hide`・`show`）と、それを呼ぶ 2 つの配線（`looper_trigger.rs` の `arm_slot`・`parts_trigger.rs` の `fire_part_triggers`）。
- 部品の時計の捨て方（`parts.rs` の `close`・`drop_finite`・`drop_hidden`・`drop_unseen_transient`）。
- 窓の知らせの受け口（`actor.rs` の `on_stage`）と面の切り替えの受け口（`LoopRuntime::on_surface_changed`）。

## Out of Boundary

- 回数つきの `always` と動く絵の子の時計の捨て方（完了 `animated-image-playback` で確定。3 語を別に扱うために関数を分けるのは可、決まりは変えない）。
- 抽選の乱数の消費の並び（変えない。既存の決定論のテストの期待値と乱数の回数を動かさない）。

## Upstream / Downstream

- **Upstream**: `areka-P0-seriko-trigger-intervals`（✅ 2026-10-10・3 語の引き金と、ここで見直す決めの出どころ）。完了 `areka-P0-animated-image-playback`（窓の知らせ `SerikoMsg::Stage`・回数つきの時計の捨て方）。
- **Downstream**: なし。`seriko-interval-combinations` が `bind+runonce` などを採るなら、着せ替えが無効になったときの捨て方は、ここで揃えた決まりの上に乗る。

## Existing Spec Touchpoints

- **Extends**: 完了 `seriko-trigger-intervals` の決めの一部を改めることがある（改めたら `doc/COMPAT_ARCHITECTURE.md` §8 と網羅台帳の備考に記録する）。
- **Adjacent**（seriko の `looper.rs`・`parts.rs`・`actor.rs`・`table.rs` を触るので直列）: `seriko-interval-combinations`・`seriko-script-triggers`・`seriko-rebuild-hidden-lottery`・`balloon-canon-residue`（seriko の `actor.rs`）・`seriko-talk-clock-fidelity`・`seriko-lottery-ended-play-skip`・`animated-image-import`（`table.rs`・`parts.rs`）。

## Constraints

- 段は**バグ**・区分 B。先に要るものは `seriko-trigger-intervals` の着地だけ。
- 行数の余裕が少ない: `looper.rs` 924 行・`table.rs` 900 行・`parts.rs` 893 行（1 ファイル 1,000 行以下）。足すものは兄弟の本番ファイル（`looper_trigger.rs` 227 行・`parts_trigger.rs` 127 行・`trigger.rs` 192 行）か新しいファイルへ置く。テストは兄弟のファイルへ。
- 既存の決定論のテストの期待値と乱数の回数は変えない。決めを変える項だけ、変える理由を要件に書いてから期待値を引き直す。
- 時刻は正確に扱う。1 フレーム遅らせる解は取らない。ログの無い失敗の経路を作らない。
- 規模の見込み: S〜M（6〜9 タスク。記録と片付けとテストだけなら 4〜5、3 の末尾のコマと 2 の起点を揃えるなら上の側）。
- 要件定義のモデル: Fable（正典が黙る所の決めを 5 つ以上並べて揃えるかを選ぶ）。
