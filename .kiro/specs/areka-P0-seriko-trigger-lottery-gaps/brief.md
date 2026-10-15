# Brief: areka-P0-seriko-trigger-lottery-gaps

> 2026-10-11 `/kiro-discovery` で起票（開発者の指示「バグの起票は粒度が小さい。修正範囲がかぶらない範囲でいくつかに集約」）。SERIKO のエンジン（`crates/areka-seriko` の `looper.rs`・`parts.rs` ほか）に集まる起票 3 本を 1 本にまとめた。元の brief は下の「件 1〜3」に全文を写してある（見出しを 1 段下げただけ・中身は変えていない）。場所と行数は元の起票（10-07・10-10）のときのもの＝着手時に引き直す。

## Problem

SERIKO の引き金と抽選に、決まりの食い違いが 3 件残っている。3 件とも `looper.rs`・`parts.rs` に触り、うち 2 件は同じ抽選の決定論のテストの期待値を引き直す。別々の spec にすると、同じファイルを 3 回順番待ちで触り、同じテストを 2 回引き直すことになる。

- **件 1（旧 `seriko-trigger-teardown-gaps`・区分 B）**: interval の 3 語（`talk`・`runonce`・`periodic`）の引き金の後始末（構え直し・隠す・捨てる）が、一番上の面と部品で食い違う。
- **件 2（旧 `seriko-lottery-ended-play-skip`・区分 B）**: 抽選の animation が、境界より前に終わった再生の片付けが残っていると、その境界のくじを 1 回飛ばし、乱数も引かない。
- **件 3（旧 `seriko-rebuild-hidden-lottery`・区分 C）**: 部品の組み直し（`rebuild`）の 1 回目が、外側の今のコマでは見えない部品にも回り、抽選のくじを引く。

どれもコードの読みと決定論のテストから出た件で、実機で絵の誤りを見たものは無い。

## Current State

- 前提の `seriko-trigger-intervals` は 2026-10-10 に着地した。3 件ともその後のコードで残っている。細部は各件の節。
- 触るファイル: seriko の `looper.rs`（924 行）・`parts.rs`（893 行）・`table.rs`（900 行）・`actor.rs`・`trigger.rs`・`looper_trigger.rs`・`parts_trigger.rs` と兄弟のテスト。どれも上限 1,000 行の近く＝足すものは兄弟のファイルへ。
- 件 2 と件 3 は、直す向きを選ぶと同じ抽選の決定論のテスト（一番上・部品・着せ替えつき）の期待値を引き直す。

## Desired Outcome

- 件 1: 3 語の引き金の後始末の各項が、「揃える」か「今のままを決まりとして記録する」のどちらかに 1 件ずつ決まっている。面を離れたときの停止の記録が出る。`\b[番号]` の切り替えで構え直ることがテストで固定されている。同じ決まりの写し 3 種が 1 か所にまとまっている。
- 件 2・件 3: 「境界の時刻で測る／見える部品だけを評価する」に直すか、今の決まりとして記録して閉じるかが決まり、選んだ方が決定論のテストで固定されている。
- 決めを変えた・足した所は `doc/COMPAT_ARCHITECTURE.md` §8 と網羅台帳の備考に 1 文ずつ在る。

## Approach

要件の段で、各項を「直す／記録して閉じる」に分ける（候補は各件の節）。**件 2 と件 3 は同じタスクの並びで扱い、抽選のテストの期待値の引き直しを 1 度で済ませる**。順の目安は 件 2・件 3（抽選・乱数の並びが動く側）→ 件 1（3 語の後始末・乱数の並びは動かさない）→ 最後に写しの片付け。件 1 は「乱数の消費の並びを変えない」を約束に持つので、件 2・件 3 で引き直した後の期待値を基準にする。

## Scope

- **In**: 3 語の引き金の状態と時計の構え直し・隠す・捨てるの決まりの見直し（一番上の面と部品）／窓の知らせだけで面の番号が変わる場面の扱い／面を離れたときの停止の記録／`\b[番号]` の切り替えのテスト／同じ決まりの写しの片付け／抽選の「再生中か」の測り方／`rebuild` の評価の順と範囲／抽選の決定論のテストの引き直し／決めの記録（COMPAT §8・網羅台帳の備考）。
- **Out**: 3 語を「いつ鳴らすか」の決まりそのもの（完了 `seriko-trigger-intervals`）・**文字の時刻の写しと起点（`seriko-talk-clock-fidelity`＝本 spec の後に直列）**・`+` の組み合わせ（`seriko-interval-combinations`）・`yen-e`・`never`・`\i[ID]`（`seriko-script-triggers`）・抽選の確率と境界の間隔・乱数の出どころと種・刻みの周期。

## Boundary Candidates

- 引き金の状態の寿命（`trigger.rs` の `Armed`）と 2 つの配線（`looper_trigger.rs`・`parts_trigger.rs`）。
- 部品の時計の捨て方（`parts.rs` の `close`・`drop_finite`・`drop_hidden`・`drop_unseen_transient`）。
- 窓の知らせの受け口（`actor.rs` の `on_stage`）と面の切り替えの受け口（`LoopRuntime::on_surface_changed`）。
- 抽選の「再生中か」（`looper.rs` の `on_tick`・`parts.rs` の `advance`）と `rebuild` の評価の順。

## Out of Boundary

- 回数つきの `always` と動く絵の子の時計の捨て方（完了 `animated-image-playback` で確定。関数を分けるのは可、決まりは変えない）。
- 「見えると決まった部品だけが 3 語の引き金を受ける」の約束（`parts_trigger_tests.rs` が固定＝保つ）。

## Upstream / Downstream

- **Upstream**: `seriko-trigger-intervals`（✅ 10-10）・`animated-image-playback`（✅ 10-07）・`surface-element-nesting`（✅）。どれも着地済み。
- **Downstream**: `seriko-talk-clock-fidelity`（同じ `trigger.rs`・`looper.rs`・`looper_trigger.rs` を触る＝本 spec の後）。`seriko-interval-combinations` が `bind+runonce` などを採るなら、着せ替えが無効になったときの捨て方はここで揃えた決まりの上に乗る。

## Existing Spec Touchpoints

- **Extends**: 完了 `seriko-trigger-intervals` の決めの一部と、完了した SERIKO の抽選の決定論のテストの期待値を改めることがある（改めたら記録する）。
- **Adjacent**（seriko の `looper.rs`・`parts.rs`・`actor.rs`・`table.rs` を触るので直列）: `seriko-talk-clock-fidelity`・`seriko-interval-combinations`・`seriko-script-triggers`・`balloon-canon-residue`（seriko の `actor.rs`）・`animated-image-import`（`table.rs`・`parts.rs`）。

## Constraints

- 段は**バグ**・区分 B（件 3 だけ元は区分 C＝件 2 と同じテストを引き直すので一緒に取る）。
- 規模の見立て: 8〜21 タスク（件 1＝6〜9・件 2＝記録だけ 1〜2／直すなら 4〜6・件 3＝同じ）。件 2・件 3 を両方「直す」と選ぶと上限 20 に近い＝引き直すテストの本数を要件の段で先に数える。
- 既存の決定論のテストの期待値と乱数の回数は、決めを変える項だけ、変える理由を要件に書いてから引き直す。
- 時刻は正確に扱う。1 フレーム遅らせる解は取らない。ログの無い失敗の経路を作らない。1 ファイル 1,000 行以下。
- 要件定義のモデル: Fable（正典が黙る所の決めを 5 つ以上並べて選ぶ）。

---

## 件 1: 旧 `areka-P0-seriko-trigger-teardown-gaps`（元の brief の全文）

> 起票: 2026-10-10（`areka-P0-seriko-trigger-intervals` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-seriko-trigger-intervals/tasks.md` の Implementation Notes の 3.1・4.1・5.1・9.2 の ⑧ と、検証（`/kiro-validate-impl`）が拾った軽い点の ⑴〜⑶・⑹。どれもコードの読みと決定論のテストから出た件で、実機で絵の誤りを見たものは無い。区分 B（バグ）。

### Problem

- **ゴーストの作者**: interval の 3 語（`talk,数値`・`runonce`・`periodic,数値`）で始まる animation の「いつ構え直すか・いつ捨てるか」が、一番上の面に書いたときと、element定義で置いた部品の面に書いたときとで食い違う場面がある。同じ pattern定義でも、書いた場所によって、バルーンの窓を閉じて開いた後の絵と、`periodic` の周の起点が変わる。
- **開発者**: 面を離れた瞬間に再生中だった animation の「停止」の記録が出ない。実機の記録から「いつ止まったか」を追えない。
- 正典（ukadoc）は、この後始末の場面について何も書いていない。`seriko-trigger-intervals` は場面ごとに areka の決めを置いたが、決めどうしが揃っていない所と、決めを固定するテストが無い所が残った。

### Current State

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

### Desired Outcome

- 一番上の面と部品の面で、3 語の構え直しと捨て方が同じ決まりになる。揃えない所は、理由つきで「areka の決め」として記録に残る。
- `\b[番号]` を受けていないスコープでバルーンの面の番号が変わったときの扱いが決まり、テストで固定される（一番上の `always` も同じ決まりに従う）。
- 面を離れて止まった再生に、止まったことが分かる記録が 1 行残る。絵に出なかった「鳴らした」の記録が、読む人を誤らせない形になる。
- 上の 7 の一覧の各項が、「網羅台帳の備考に 1 文足す」か「振る舞いを変える」かのどちらかに決まる。変えたものは決定論のテストで固定する。
- 同じ決まりの写しが 1 か所になる（振る舞いは変えない）。

### Approach

要件の段で、上の 1〜4 と 7 の各項を 1 件ずつ「揃える／今のままを決まりとして記録する」に分ける。目安は次のとおり。

- 利用者に見える差（3 の末尾のコマ・2 の周の起点）は揃える向きで検討する。揃える先は一番上の面の扱い（閉じても末尾のコマを保つ・再生中は最後まで進む・窓の新しい出番では起点を動かさない）が第一の候補。部品の時計を閉じたときに捨てているのは、回数つきの `always` と同じ関数で片付けているためなので、3 語だけ別に扱えるかをコードで確かめる。
- 後から見えた部品の起点（7 の 4 つ目）は、「時刻は正確に扱う」の原則（待ち時間を丸めない・更新が遅れたら過ぎた時間の分だけ進める）に照らして、外側のコマの時刻を起点にできるかを設計で見る。できないなら理由を記録する。
- 記録（4）は、落とす所で `info!`（`talk` は `debug!`）を 1 行出す。既存の再生の終わりの記録と同じ文言の型に合わせる。
- 片付け（6）は最後のタスクでまとめて行う。述語は 1 か所（`table.rs` の `LoopTrigger` の隣）に置き、ほかはそれを呼ぶ。

### Scope

- **In**: 3 語の引き金の状態と時計の、構え直し・隠す・捨てるの決まりの見直し（一番上の面と部品）。窓の知らせだけで面の番号が変わる場面の扱い（一番上の `always` を含む）。面を離れたときの停止の記録。`\b[番号]` の切り替えのテスト。同じ決まりの写しの片付け。網羅台帳の 3 行の備考への追記。`doc/COMPAT_ARCHITECTURE.md` §8 への記録（決めを変えた・足したとき）。
- **Out**: 3 語を「いつ鳴らすか」の決まりそのもの（`seriko-trigger-intervals` で確定）。文字の時刻の写しと起点（`seriko-talk-clock-fidelity`）。抽選の境界の判定（`seriko-lottery-ended-play-skip`）。見えていない部品の抽選（`seriko-rebuild-hidden-lottery`）。`+` の組み合わせ（`seriko-interval-combinations`）。`yen-e`・`never`・`\i[ID]`（`seriko-script-triggers`）。

### Boundary Candidates

- 引き金の状態の寿命（`trigger.rs` の `Armed` の `arm`・`hide`・`show`）と、それを呼ぶ 2 つの配線（`looper_trigger.rs` の `arm_slot`・`parts_trigger.rs` の `fire_part_triggers`）。
- 部品の時計の捨て方（`parts.rs` の `close`・`drop_finite`・`drop_hidden`・`drop_unseen_transient`）。
- 窓の知らせの受け口（`actor.rs` の `on_stage`）と面の切り替えの受け口（`LoopRuntime::on_surface_changed`）。

### Out of Boundary

- 回数つきの `always` と動く絵の子の時計の捨て方（完了 `animated-image-playback` で確定。3 語を別に扱うために関数を分けるのは可、決まりは変えない）。
- 抽選の乱数の消費の並び（変えない。既存の決定論のテストの期待値と乱数の回数を動かさない）。

### Upstream / Downstream

- **Upstream**: `areka-P0-seriko-trigger-intervals`（✅ 2026-10-10・3 語の引き金と、ここで見直す決めの出どころ）。完了 `areka-P0-animated-image-playback`（窓の知らせ `SerikoMsg::Stage`・回数つきの時計の捨て方）。
- **Downstream**: なし。`seriko-interval-combinations` が `bind+runonce` などを採るなら、着せ替えが無効になったときの捨て方は、ここで揃えた決まりの上に乗る。

### Existing Spec Touchpoints

- **Extends**: 完了 `seriko-trigger-intervals` の決めの一部を改めることがある（改めたら `doc/COMPAT_ARCHITECTURE.md` §8 と網羅台帳の備考に記録する）。
- **Adjacent**（seriko の `looper.rs`・`parts.rs`・`actor.rs`・`table.rs` を触るので直列）: `seriko-interval-combinations`・`seriko-script-triggers`・`seriko-rebuild-hidden-lottery`・`balloon-canon-residue`（seriko の `actor.rs`）・`seriko-talk-clock-fidelity`・`seriko-lottery-ended-play-skip`・`animated-image-import`（`table.rs`・`parts.rs`）。

### Constraints

- 段は**バグ**・区分 B。先に要るものは `seriko-trigger-intervals` の着地だけ。
- 行数の余裕が少ない: `looper.rs` 924 行・`table.rs` 900 行・`parts.rs` 893 行（1 ファイル 1,000 行以下）。足すものは兄弟の本番ファイル（`looper_trigger.rs` 227 行・`parts_trigger.rs` 127 行・`trigger.rs` 192 行）か新しいファイルへ置く。テストは兄弟のファイルへ。
- 既存の決定論のテストの期待値と乱数の回数は変えない。決めを変える項だけ、変える理由を要件に書いてから期待値を引き直す。
- 時刻は正確に扱う。1 フレーム遅らせる解は取らない。ログの無い失敗の経路を作らない。
- 規模の見込み: S〜M（6〜9 タスク。記録と片付けとテストだけなら 4〜5、3 の末尾のコマと 2 の起点を揃えるなら上の側）。
- 要件定義のモデル: Fable（正典が黙る所の決めを 5 つ以上並べて揃えるかを選ぶ）。

---

## 件 2: 旧 `areka-P0-seriko-lottery-ended-play-skip`（元の brief の全文）

> 起票: 2026-10-10（`areka-P0-seriko-trigger-intervals` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-seriko-trigger-intervals/tasks.md` の Implementation Notes の 4.1（査読の指摘）と 4.1-fix（範囲外の観察）。コードの読みで見つけた件で、実機で絵の誤りを見たものではない。区分 B（バグ）。

### Problem

抽選で始まる animation（interval `random,数値`・`bind+random,数値`・`sometimes`・`rarely`）は、1 秒ごとの境界で「今再生中でなければ」くじを引く。ところが「再生中か」を、境界の時刻に再生が続いているかではなく、「再生の記録がまだ片付けられていないか」で測っている。境界より前に終わった再生でも、片付けがその境界の刻みまで残っていると、その境界のくじを 1 回飛ばし、乱数も引かない。

- **ゴーストの作者**: まばたきなどの抽選の animation が、書いた確率より少し起きにくくなる場面がある（再生の終わりから次の境界までの間に刻みが 1 回も入らなかったとき）。
- **開発者**: 同じ形の件を、`seriko-trigger-intervals` は 3 語（`runonce`・`periodic`・`talk`）の側でだけ直した。抽選の側は、乱数の消費の並びが変わるので触らなかった（同 spec の要件 8.2「`random`・`bind+random`・`sometimes`・`rarely`・`always`・`bind` の決定論のテストの結果と乱数の消費の並びを変えない」）。

### Current State

2026-10-10 時点（`seriko-trigger-intervals` のブランチ・コミット `84ca70e6`）。境界は 1,000 ms の絶対の格子（`crates/areka-seriko/src/timeline.rs` の `LotteryBoundary`）。

- **一番上の面**（`crates/areka-seriko/src/looper.rs` の `LoopRuntime::on_tick`）: 手順の (2)「抽選」が (3)「進行」より前にある。(2) は、再生の表 `playback` にその animation が載っていれば `continue` する（「(a) 非再生中のみ対象」）。終わった再生を表から外すのは (3) の `put_top_play`（末尾に着いた・負の番号で止まった）なので、境界をまたいだ刻みで初めて終わりに気付く再生は、(2) の時点ではまだ表に載っている。その境界ではくじを引かない。
- **部品**（`crates/areka-seriko/src/parts.rs` の `PartClocks::advance` の中の 1 段目の評価）: 同じ形。`playing = matches!(clocks.get(&key), Some(PartAnim::Playing { .. }))` を見てから `should_fire` を呼び、その後で `progress` が終わった時計を片付ける（末尾のコマを保つ時計 `PartAnim::Residual` へ移すか、消す）。
- **起きる条件**: 再生が終わる時刻と次の境界の間に、刻みが 1 回も入らないこと。刻みは約 16 ms ごとなので、ふだんは境界の直前の 1 刻み分の窓だけ。刻みが遅れたときは、その分だけ窓が広がる。抽選の再生は境界をまたいだ刻みの時刻で始まるので、長さが 1,000 ms の倍数に近い animation は終わりが次の境界の刻みの近くに来て、当たりやすい見込み（どのくらいの頻度で起きるかは測っていない＝要件の段で、偽の時計のテストで数える）。
- **3 語の側の直し**（比べる相手）: `crates/areka-seriko/src/trigger.rs` の `Armed::poll` は、「再生中か」を判定の時刻でなく、返すことになる開始の時刻で呼び手に尋ねる（引数 `playing_at`）。呼び手（`looper_trigger.rs` の `fire_top_triggers`・`parts_trigger.rs` の `fire_part_triggers`）は「その時刻のコマがまだ終わりでないこと」で答え、終えていて片付けていない再生は、その時刻まで進めて片付けてから入れ替える。この直しの前は、長さが周期ちょうどの `periodic` が 2 周に 1 回しか鳴らなかった（同 spec の Implementation Notes の 4.1 の査読の指摘）。
- 見た目の誤りを実機で見たことは無い。同梱の検体の抽選の animation（emo2 の `bind+random` 3 本・`random` 4 本、claudia と konnoyayame の `sometimes` 各 1 本）で頻度を測ってもいない。

### Desired Outcome

- 抽選の「再生中か」が、境界の時刻に再生が続いているかで決まる。境界より前に終わった再生は、その境界のくじに入る。
- または、今の振る舞いを「areka の決め」として記録して閉じる（正典は境界と再生の終わりが重なる場面を書いていない）。
- どちらを選んでも、選んだ方が決定論のテスト（乱数を固定した抽選の並び）で固定されている。

### Approach

要件で選ぶ。

- **⑴ 境界の時刻で測る**: 抽選の前に、境界の時刻までに終わった再生を片付ける（3 語の側と同じ「その時刻のコマがまだ終わりでないこと」の述語を使う）。乱数を引く回数と並びが変わるので、抽選の決定論のテスト（一番上・部品・着せ替えつき）の期待値を引き直す。引き直す本数を要件の段で数える。
- **⑵ 記録して閉じる**: `doc/COMPAT_ARCHITECTURE.md` §8 と網羅台帳の備考に「境界の刻みで終わりに気付いた再生は、その境界のくじを引かない」と書く。コードは変えない。

⑴ を選ぶなら、`seriko-rebuild-hidden-lottery`（見えていない部品のくじ）も同じテストの期待値を引き直すので、2 本を続けて取ると引き直しが 1 度で済む。

### Scope

- **In**: 一番上の面と部品の抽選の「再生中か」の測り方。抽選の決定論のテストの引き直し（⑴ のとき）。決めの記録。
- **Out**: 抽選の確率と境界の間隔。3 語の引き金（`seriko-trigger-intervals` で確定）。見えていない部品のくじ（`seriko-rebuild-hidden-lottery`）。`always`（抽選しない）。

### Boundary Candidates

- `looper.rs` の `on_tick` の (2) と (3) の順、または (2) の前の片付け。
- `parts.rs` の `advance` の 1 段目の、抽選の腕の `playing` の測り方。

### Out of Boundary

- 乱数の出どころ（`LoopRng`）と種。
- 刻みの周期。

### Upstream / Downstream

- **Upstream**: `areka-P0-seriko-trigger-intervals`（✅ 2026-10-10・開始の時刻で「再生中か」を測る形の出どころ。働きの上の依存は無いが、同じ述語と同じファイルを使う）。
- **Downstream**: なし。

### Existing Spec Touchpoints

- **Extends**: なし（⑴ を選ぶと、完了した SERIKO の抽選の決定論のテストの期待値を改める）。
- **Adjacent**（seriko の `looper.rs`・`parts.rs` を触るので直列）: `seriko-rebuild-hidden-lottery`（同じ抽選のテスト）・`seriko-trigger-teardown-gaps`・`seriko-talk-clock-fidelity`・`seriko-interval-combinations`・`seriko-script-triggers`・`animated-image-import`（`parts.rs`）。

### Constraints

- 段は**バグ**・区分 B。
- `looper.rs` は 924 行・`parts.rs` は 893 行（1 ファイル 1,000 行以下）。足すなら兄弟の本番ファイルへ。
- 時刻は正確に扱う。1 フレーム遅らせる解は取らない。
- 規模の見込み: XS〜S（記録だけ 1〜2・直すなら 4〜6 タスク。引き直すテストの本数で上下する）。
- 要件定義のモデル: Opus。

---

## 件 3: 旧 `areka-P0-seriko-rebuild-hidden-lottery`（元の brief の全文）

> 2026-10-07 `animated-image-playback` の完了時の棚卸で起票（`completed/areka-P0-animated-image-playback/tasks.md` の Implementation Notes の 3.x の範囲外の項）。

### Problem

入れ子の面（element定義の子の面）を持つシェルで、外側の面が別のコマに居る刻みでも、見えていない部品の `random` の抽選が回ることがある。見た目の誤りは確認されていないが、乱数の消費が見えない部品に回るので、見える部品の抽選の並びが「見えている部品だけを引いたとき」と変わる。

### Current State

- `crates/areka-seriko/src/parts.rs` の `rebuild` は、`pattern.clear_parts()` の直後の 1 回目の `nest.visible_parts(…)` で、経過 0 の辺・着せ替えの辺の先の部品も見える部品として評価する（`evaluate(PartKey::Surface(part), pattern)`）。外側が別のコマに居ても、この 1 回目では評価が回る。
- 評価が終わった後、見えない部品のコマは捨てる（`evaluated.len() > visible.len()` の枝）。表示には出ない。
- `surface-element-nesting` からの性質。`animated-image-playback` の前後で引く乱数の数は同じ。

### Desired Outcome

- 見えていない部品の抽選を回さない（見える部品だけが乱数を引く）か、今の振る舞いが正典上も問題ないと確かめて記録して閉じる。

### Approach

要件で決める。⑴ 1 回目から外側の今のコマで見える部品だけを評価する。⑵ 今のままにし、乱数の消費の並びを決まりとして記録する。決定論のテスト（乱数を固定した抽選の並び）で選んだ方を固定する。

### Scope
- **In**: `rebuild` の評価の順と範囲・決定論のテスト。
- **Out**: 抽選の確率・間隔の決まり（`seriko-trigger-intervals`）。

### Boundary Candidates
- `crates/areka-seriko/src/parts.rs`（`rebuild`）・`nest_table` の `visible_parts`

### Out of Boundary
- 動く絵の子の再生（`animated-image-playback` で確定）

### Upstream / Downstream
- **Upstream**: `surface-element-nesting`（✅）・`animated-image-playback`（✅ 10-07）
- **Downstream**: なし

### Existing Spec Touchpoints
- **Extends**: なし
- **Adjacent**: `seriko-trigger-intervals`（同じ `parts.rs`・`looper.rs` を触る＝同時に走らせない）

### Constraints
- 規模の見立て: XS〜S（記録だけ〜4〜6）。


### 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化: 起票（10-07）から `crates/areka-seriko/src/parts.rs` は変更 0。Current State は今の main と一致（`rebuild` の 1 回目が見える部品を求めては評価し、終わってから見えない部品のコマを外す）。
- 触るファイル: `crates/areka-seriko/src/parts.rs`（`rebuild`）と兄弟のテスト。案 ⑴ でも、見える部品を求める `visible_parts`（`crates/areka-emo-compose/src/nesting.rs`）は呼ぶだけで変えない見込み。
- 規模: 記録だけで閉じるなら 1〜2、直すなら 4〜6 タスク。切らない。
- 先に要るもの: 働きの上では無し。`seriko-trigger-intervals` と `parts.rs`・`looper.rs` を分け合うので、その後。
- 優先度の区分: C（見た目の誤りが確認されていない持ち越し）。
- 要件定義のモデル: Opus。
- 分割の案: 無し。
- 見つけた穴・古くなった記述: `seriko-trigger-intervals` が `runonce`・`periodic`・`talk` を部品にも足すと、この性質は「乱数の並びが変わるだけ」でなく「見えていない部品で再生が始まる」に変わりうる。あちらの設計で部品の門を見るときに一緒に決めてしまえば、本 spec は記録だけで閉じられる。

### 2026-10-10 `areka-P0-seriko-trigger-intervals` の完了時の申し送り

出どころは `completed/areka-P0-seriko-trigger-intervals/tasks.md` の Implementation Notes（5.1・5.2）と同 spec の設計書（Revalidation Triggers・部品の節）。前提の `seriko-trigger-intervals` は 2026-10-10 に着地した。

- **上の節の心配（「見えていない部品で再生が始まる」）は起きなかった**。`parts.rs` の `rebuild` は 2 段になった。
  - 1 段目は今までの評価のまま＝外側の今のコマでは見えない部品にも回り、抽選のくじを引く（**乱数の消費の並びは変えていない**。本 spec の Current State は今も当たっている）。3 語（`runonce`・`periodic`・`talk`）は、1 段目では在る時計を進めるだけで、始めない。
  - 2 段目は、見えると決まった部品にだけ回り、3 語を始めるかを判定する。1 段目の評価を受けただけで、見えると決まらなかった部品は 2 段目に来ない。
- **本 spec が `rebuild` の評価の順を変えるときの約束**: 「見えると決まった部品だけが 3 語の引き金を受ける」を保つ（`seriko-trigger-intervals` の設計書の Revalidation Triggers が名指ししている）。固定しているテストは `crates/areka-seriko/src/parts_trigger_tests.rs`。
- **触るファイルの今**: `parts.rs` は 893 行（1,000 行の上限まで約 100 行）。部品の 3 語の配線は兄弟の `parts_trigger.rs` に出してある。
- **近い件が 1 本起票された**: `seriko-lottery-ended-play-skip`（終わった再生が片付く前の境界で、抽選のくじを 1 回飛ばす件）。直す向きを選ぶと、本 spec と同じ抽選の決定論のテストの期待値を引き直す＝2 本を続けて取ると引き直しが 1 度で済む。

## 2026-10-16 棚卸㉕の再測定（main `12801f6d`）
- 前提の変化: 版の段で v0.1.0（バグ）。棚卸㉔ではこの spec の節を書いていない（上限で外れた）。seriko は 10-10 から変更 0（`looper.rs` 924・`parts.rs` 893・`table.rs` 900・`actor.rs` 776・`trigger.rs` 192・`looper_trigger.rs` 227・`parts_trigger.rs` 127 行）。3 件の指し先（3 語の述語の写し＝`looper.rs` の `is_trigger_word`・`parts.rs` の `is_trigger`・`table.rs` の `has_triggers`、`parts.rs` の `rebuild`・`drop_hidden`、`actor.rs` の `on_stage`、`looper.rs` の `on_surface_changed`）は今も在る。
- 触るファイル: 上の 7 本と兄弟のテスト・`doc/COMPAT_ARCHITECTURE.md` §8・台帳の備考。
- 規模: 8〜21 タスク（件 2・件 3 を両方「直す」にすると 20 を少し越えうる。両方「記録して閉じる」なら 8〜12）。
- 先に要るもの: 無し。後ろに `seriko-talk-clock-fidelity`（v0.2.0）。
- 同じウェーブに置けない相手: `animated-image-import`（`table.rs`・`parts.rs`）。`mcp-reload`（ついでの片付けで seriko の `actor.rs` の `"bind"` に触る場合）・`mcp-strict-errors`（案 ⑵ で `actor.rs`・`resolve.rs` に触る場合）。版に縛らない列の `seriko-interval-combinations`・`seriko-script-triggers` とも直列（変わらず）。
- 優先度の区分: B（件 3 だけ元は C）。
- 要件定義のモデル: **Opus に改める**（本文は Fable）。正典の黙る所を 5 つ以上選ぶのは「議題が多い」で、10-15 の基準では Fable にならない。件 2 の「境界の時刻で測る」は、着地済み `seriko-trigger-intervals` の時計の決まりの上での直しで、時刻の順の新しい設計ではない。
- 分割の案: 無し（10-11 にわざと 1 本にまとめた＝20 を少し越えるだけなら割らない。引き直すテストの本数を要件の段で先に数える）。
- 見つけた穴・古くなった記述: 件 1 の節の Adjacent に、今はこの spec に入った旧名 `seriko-rebuild-hidden-lottery`・`seriko-lottery-ended-play-skip` が並んでいる（同じ spec の中の話と読む）。
