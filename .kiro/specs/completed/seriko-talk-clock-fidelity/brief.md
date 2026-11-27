# Brief: seriko-talk-clock-fidelity

> 起票: 2026-10-10（`areka-P0-seriko-trigger-intervals` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-seriko-trigger-intervals/tasks.md` の Implementation Notes の 3.3・4.2・5.2・9.2 の ⑤⑨ と、検証が拾った軽い点の ⑷⑸、同 spec の設計書の「時計（要件 6）」の節。数値は同 spec の実機の確かめ（2026-10-10）の記録から写した（記録の置き場 `target\` は git に残らないので、要る数はこの brief に書いてある）。区分 B（バグ）。

## Problem

- **利用者**: 口パク（interval `talk,数値`）は「数値分の文字がバルーンに現れた時刻」に動くはずだが、seriko が見積もる「文字が現れた時刻」と、文字の層が実際に文字を出す時刻が、最大で 1 刻み（16 ms）ほどずれうる。今は目に見える差ではないが、設計が引いた線（1 刻み）に触れている。
- **開発者**: seriko は、文字が現れる時刻の式を文字の層から**写して**持っている（共有していない）。文字の層の式を変える spec（早送り・`\_q`・文字の現れ方の演出）が着地しても、赤になるテストが無い。口だけが古い式で動き続けても、誰も気付けない。

## Current State

2026-10-10 時点（`seriko-trigger-intervals` のブランチ・コミット `84ca70e6`）。

- **仕組み**: seriko は届いた cue から 2 つを自分で持つ（`crates/areka-seriko/src/talk.rs`）。
  - `TalkEpoch`＝台本の 0 秒が seriko の時計の何 ms に当たるか（起点）。届いた cue ごとに「今 − cue の時刻」の最大を取る。文字の層の時刻源（`crates/areka/src/emo2_boot/talk_clock.rs` の `TalkClock::observe_cue`）と同じ式。
  - `TalkFeed`＝スコープごとの「i 文字目が現れる台本の秒」の列。文字の層の `RevealSchedule::extend_chunk`（`crates/areka-emo-text/src/state.rs`）と同じ式で積む。
- **時計が 2 つある**: 文字の層の起点は QPC の秒（`TalkClock` の既定の時計）、seriko の起点は `GetTickCount64` のミリ秒（`emo2_boot/mod.rs` の `seriko_clock`）。2 つの層は、同じ cue をそれぞれ自分のスレッドで受け取った時刻から、別々に起点を見積もる。

残っている件:

1. **絶対の時刻の差が 1 刻みの境に触れる**（実機・2026-10-10）。`talk` の開始の時刻と、文字の層の記録から求めた「区切りの文字が現れた時刻」の差の見積もりは −16.9〜+1.2 ms（見積もりの誤差は数 ms）。起点の差を含まない比べ方（隣り合う開始の時刻の差と、文字の層の式の差の食い違い）では最大 9.0 ms（一番上の `talk,3` の 10 件。起点の見積もりが 1 回動いた回）、部品の `talk,2` の 8 件は 0.0 ms。設計書は「ずれが 1 刻みを超えて見えたら、`emo2_boot/mod.rs` の `seriko_clock` の結線の議題として起票する」と書いており、−16.9 ms はその境目に当たる。
2. **cue の届く時刻が揺れる**（実機）。最初の cue を基準にすると、後の cue は台本の時刻より −9〜+50 ms ずれて届いた。両方の層が「今 − cue の時刻」の最大を起点にするので、遅れて届いた cue があると、起点が台詞の途中で 1 回前へ動く（上の 9.0 ms の回）。文字の層でも同じことが起きる。
3. **中断の直後に 1 文字を数え落とす**。台詞を中断して新しい台詞が始まると、頭の全消去が台本の 0 秒で届く。seriko は `TalkFeed::restart_chain(cue.at)` で「その時刻までに現れていない文字」を捨てるが、比べる相手が前の台詞の秒（大きい値）と新しい台詞の 0 秒なので、前の台詞の文字のうち「最後の刻みより後に現れて、まだ刈り込まれていない文字」も捨てる。その文字は数えに入らない。区切りの文字だった場合はその 1 回が鳴らず、面が替わるまで区切りの位相が 1 文字ずれる（1 刻み未満の窓・ふつう 0〜1 文字）。
4. **写しを見張るテストが無い**。`talk.rs` は冒頭に「文字の層の式が変わったら、ここも同じに変える（写しであって、共有ではない）」と書くだけで、`RevealSchedule::extend_chunk` や `TalkClock` を変えても赤になるテストは無い。見張りは実機の差の計測だけ。`seriko-trigger-intervals` の設計書の Revalidation Triggers は、式を変える見込みの spec として `sakura-time-directives`・`talk-fast-forward` を挙げている。
5. **列の単位が違う**。文字の層の列は「スコープ × 文字の場所（普通のバルーン／名前の箱）」ごと、`TalkFeed` はスコープごと。今の台本の組み立て（1 字 50 ms・次の cue は前の cue の終わり以後）では、差は ms の丸めで消える。
6. **後から見えた部品が、自分が見えるより前の区切りで鳴りうる**。部品は文字の数えを持たず、面の文字の窓を借りる。部品が見えた刻みの窓に区切りが入っていると、その刻みで鳴り、開始の時刻（区切りの文字が現れた時刻）は部品が見えた時刻より前になりうる（1 刻み未満）。`runonce`・`periodic` は起点より前に始まらないので、`talk` だけが違う。この場面を固定するテストは無い。
7. **「1 回の刻みで区切りを 2 つ以上越える」のテストが純粋な関数の水準だけ**。判定（`trigger.rs` の `Armed::poll`）のテストは在るが、cue から刻みまでの配線を通したテストは無い。実機でも踏めていない（まとめて文字を出す `\_q` が未対応のため）。
8. **数え始めが 0 で止まる天井**。`trigger.rs` の `Armed::realign_talk` は、消去の後に数えを列の総数へ揃えるとき、数え始めの序数を引き算で動かす。数え始めが捨てられた数より小さいと 0 で止まり、数えた文字の数が減る（次の区切りが 1 回遅れる）。コードに `ponytail:` の印つきで「届くようなら数え始めを符号つきにする」と書いてある。届く条件は「起点の見積もりが前へ動いて、現れた数が減って見える間に消去が届いたとき」。

## Desired Outcome

- 口が動く時刻と、区切りの文字が画面に現れる時刻の差が、いつも 1 刻みより小さいと言える根拠がある（測り方と上限が記録に残る）。
- 文字の層の「文字が現れる時刻」の式か起点の式を変えると、seriko の側が必ず赤になる（または、同じ 1 つの式を両方が使う）。
- 中断の直後の新しい台詞で、文字を数え落とさない。
- 上の 6・7 の場面が決定論のテストで固定される。8 の天井は、届かないことを示すか、符号つきにして外す。

## Approach

直し方は決めていない。要件・設計で選ぶ。

- **写しの見張り**（4）は 2 つの向きがある。⑴ 式を 1 つの純粋な関数にして、文字の層と seriko の両方が呼ぶ（置き場は両方が依存できる下のクレート。`areka-seriko` は今 `areka-sakura` の `cluster_count` を使っている）。⑵ 写しのまま、同じ入力の列を両方の式に通して結果を比べるテストを 1 本置く（両方に依存しているのは `crates/areka`。`TalkFeed` と `RevealSchedule::extend_chunk` は今どちらもクレートの外から呼べないので、比べるための口が要る）。⑴ は文字の層のファイルに触る＝文字とバルーンの列と重なる。
- **起点**（1・2）は、まず実機で差の出どころを分ける: 時計の種類（QPC と `GetTickCount64` の分解能 10〜16 ms）／層ごとの cue の受け取りの遅れ／起点の見積もりが動く回。そのうえで、起点を 1 か所で見積もって両方へ配るか、seriko の時計を文字の層と同じ時計に揃えるか、今のまま上限を記録するかを選ぶ。`seriko-trigger-intervals` の設計は「seriko の時計を QPC に揃える案は採らない」と決めており、その理由（刻み自体が 16 ms）が今も成り立つかを数値で確かめる。
- **中断の直後**（3）は、前の台詞の文字のうち「消去が届くまでに現れていたもの」を数えに残す形にする（消去で比べる時刻を、新しい台詞の 0 秒でなく、前の台詞の起点で測った今の秒で取る、など。起点を新しい台詞へ動かす前に測る順が要る）。文字の層が中断のとき列をどう扱うか（未表示の文字ごと空にする）と同じ結果になることを、テストで並べて確かめる。
- 6 は「部品が見えた時刻より前の区切りでは鳴らさない」に揃えるか、今のままを決まりとして記録するかを要件で決める。

## Scope

- **In**: 文字の時刻の写し（`talk.rs`）と文字の層の式の一致を見張る仕組み。起点の見積もりの差の計測と、差を 1 刻み未満に保つ直し。中断の直後の数え落とし。部品の `talk` と区切りの 2 つ越えの配線の水準のテスト。`realign_talk` の天井。決めを変えたときの `doc/COMPAT_ARCHITECTURE.md` §8 と網羅台帳の備考（`talk_2c_6570_5024:1`）。
- **Out**: 文字が現れる時刻そのものを変えること（早送りは `talk-fast-forward`・`\_q` と `\![quicksection]` と `\![set,balloonwait]` は `sakura-time-directives`・現れ方の演出は `text-reveal-fade`）。`talk` をいつ鳴らすかの決まり（何文字ごと・再生中は見送る＝`seriko-trigger-intervals` で確定）。引き金の後始末（`seriko-trigger-lottery-gaps` の件 1）。cue の届く時刻の揺れそのものを減らすこと（dola と台本の再生機の配送の話＝ここでは揺れても差が出ない形にするだけ）。

## Boundary Candidates

- 文字が現れる時刻の式の持ち主（文字の層か、共有の関数か）。
- 起点の見積もりの持ち主（層ごとか、1 か所か）と時計の種類（`emo2_boot/mod.rs` の `seriko_clock`・`talk_clock.rs` の `TalkClock`）。
- 消去の知らせの写し方（`looper_trigger.rs` の `restart_talk`・`talk.rs` の `restart_chain`）。

## Out of Boundary

- 刻みの周期（16 ms）と刻みの出どころ。
- 文字の層の描画と配置（列の式を共有の関数へ出す場合も、呼び方を替えるだけで結果は変えない）。

## Upstream / Downstream

- **Upstream**: `areka-P0-seriko-trigger-intervals`（✅ 2026-10-10・`talk` の引き金と文字の時刻の写し）。
- **Downstream**（文字が現れる時刻を変えるので、着地のときに `talk.rs` の写しを見直す。本 spec が先に着地していれば、見張りが赤で知らせる）: `talk-fast-forward`・`sakura-time-directives`・`text-reveal-fade`・`text-reveal-dance`（据え置き）。

## Existing Spec Touchpoints

- **Extends**: 完了 `seriko-trigger-intervals` の設計の「時計（要件 6）」の決め（seriko の時計は `GetTickCount64` のまま）を、実機の数値で見直すことがある。
- **Adjacent**: seriko の `looper.rs`・`looper_trigger.rs`・`trigger.rs` を触る spec と直列（`seriko-trigger-lottery-gaps` の件 1・2・`seriko-interval-combinations`・`seriko-script-triggers`・`seriko-trigger-lottery-gaps` の件 3）。式を共有の関数へ出す形を選ぶなら、文字とバルーンの列（`crates/areka-emo-text/src/state.rs`）と `emo2_boot` の結線の列（`emo2_boot/mod.rs`・`talk_clock.rs`）にも触る＝その列の spec と同じ時に走らせない。

## Constraints

- 段は**バグ**・区分 B。先に要るものは `seriko-trigger-intervals` の着地だけ。
- `seriko-trigger-intervals` が同じウェーブで守った約束（`crates/areka-emo-text`・`emo2_boot/mod.rs`・`crates/dola` に触らない）は、その spec の着地で解けている。ここで触るなら、要件の段で触るファイルを挙げて、同じ時に走る spec と照らす。
- 時刻は正確に扱う（待ち時間を丸めない・刻みの境目へ寄せない）。1 フレーム遅らせる解は取らない。3 つ目の時計を作らない（アニメのエンジンは sakura と seriko の 2 つのまま）。
- `talk` の無いシェルでは、cue を写す仕事を増やさない（今の門＝どの表にも `talk` が無ければ何もしない、を保つ）。
- 実機の確かめは、`seriko-trigger-intervals` の検体（`crates/areka-seriko/tests/fixtures/trigger-intervals/`・面 9100〜9104）と README の手順を使い回せる。
- 規模の見込み: M（8〜12 タスク。見張りのテストと数え落としとテストの追加だけなら 5〜7、起点を 1 か所にするなら上の側＝要件で測る）。
- 要件定義のモデル: Fable（時計と起点の持ち主の判断・スレッドをまたぐ時刻の扱い）。

## ほかの spec からの申し送り

- **2026-10-15 `/kiro-discovery`（時刻の土台）**: 開発者が「関連する時刻情報は、すべて `QueryUnbiasedInterruptTimePrecise`（100ns の `u64`）から取る」行き先を決め、`dola-interrupt-ticks`・`talk-anchor-interrupt-ticks`・`instant-to-interrupt-ticks` の 3 本（どれも据え置き・区分 D）を起票した。`talk-anchor-interrupt-ticks` は、台本の起点を配る時に本物の絶対時刻で押し、文字の層と seriko が起点を見積もらずに読む形へ替える＝この spec の「起点」（1・2）の根を後から取り除く。この spec は区分 B で先に着地する見込みなので、次のとおりにする。
  - 起点の直し方で「seriko の時計を文字の層と同じ時計に揃える」を選ぶなら、揃える先は QPC でなく割り込み時刻（`QueryUnbiasedInterruptTimePrecise`）にする（後で 2 度替えないため）。ただし `dola-interrupt-ticks` の型はまだ無いので、型の新設はこの spec でしない。
  - 「起点を 1 か所で見積もって両方へ配る」を選ぶなら、その 1 か所は `talk-anchor-interrupt-ticks` で「押された起点を読む」に置き換わる前提で、置き場を選ぶ。

## 2026-11-06 棚卸㉟で拾った申し送り（audit-seriko から）
- 完了した `audit-seriko` で、トリガーの記録の出し先が `areka_seriko::looper::top_trigger`・`parts::part_trigger` から `areka_seriko::trigger` に変わった。また、同じ刻で面が切り替わるときの `pick_rng` の引く順が変わった。どちらも「v0.2.0 の seriko の spec の注に残す」と audit-seriko の tasks.md（完了の棚卸へ渡すもの）が決めていたが、どの brief にも書かれていなかったので、時刻の土台を直すこの spec の要件の段で前提として読む（文書だけの申し送り・区分 C）。

## 2026-11-27 棚卸㊹の再測定（main ccd790fc）
- **前提の変化**: 着地した `dola-interrupt-ticks`（`talk-anchor-interrupt-ticks` の範囲を含む）が、時計を 1 本にし（`GetTickCount64` は本番から消えた）、文字の層と seriko が起点を見積もらず、合図が運ぶ絶対の発火時刻から字の時刻を求める形にした。上の「残っている件」の 1〜4・6・7 はこれで済んだ: 1・2＝seriko の字の列（`crates/areka-seriko/src/talk.rs` の `TalkFeed`）は時計も起点も持たない。3＝消去は絶対の時刻で比べるので前の台詞の現れた字を捨てない（`talk_tests.rs` の中断の後の全消去・`looper_talk_tests.rs` の中断の後の新しい台詞のテスト）。4＝字の時刻の式は 1 本の関数（`crates/areka-sakura` の `reveal::chunk_reveal_offsets`）を文字の層（`crates/areka-emo-text/src/state.rs`）と seriko の両方が呼ぶ＝写しではない。6＝部品が見えた刻みの中の区切りは見え始めの時刻から始める（`parts_trigger_talk_tests.rs`）。7＝1 回の刻みで越えた区切りは合図からの配線を通して古い順に判定する（`looper_mouth_tests.rs`）。本文の `emo2_boot/` のパスも古い（時刻源は `crates/areka-runtime/src/talk_clock.rs` で、起点を持たない時計だけになった）。
- **残る件**: ⑴ 件 8＝数えを揃える関数（`crates/areka-seriko/src/trigger.rs` の `realign_talk`・`ponytail:` の印）の 0 で止まる天井。届く道は今もある（起点の置き直しの知らせが効く時刻より遅れて届くと、もう現れた字が後ろへ動いて現れた数が減って見える。その間に消去が届く）。⑵ 件 5＝字の列の単位の違い（seriko はスコープごと・`crates/areka-seriko/src/looper.rs` の `feeds`、文字の層はスコープ × 文字の場所ごと）。直すか、差が出ない理由を記録して閉じるかを要件で決める。⑶ 上の audit-seriko の申し送り（記録の出し先の名前と乱数の引く順）を要件の前提として読む。
- **触るファイル**: `trigger.rs`（275 行）と兄弟の `trigger_tests.rs`。件 5 を直すなら `looper.rs`（855・上限まで余裕あり）と `looper_talk_tests.rs`。`talk.rs`（101）・`looper_trigger.rs`（218）・emo-text の `state.rs`・`talk_clock.rs` には触らない見込み。新しい依存も emo-text の新しいファイルも無い。
- **規模**: S（2〜4 タスク）。
- **先に要るもの**: 機能の上で先に要る spec は無い（頭）。roadmap の「`seriko-trigger-lottery-gaps` の後」は、同じ `trigger.rs`・`trigger_tests.rs`（件 5 を直すなら `looper.rs`）を触るためのファイルの順だけで、あちらの直しを使うわけではない＝あちらと同じウェーブに置かなければ先に走れる。今のウェーブの 4 本（`draw-methods-canon` は seriko の `table.rs` の期待だけ・ほかの 3 本は seriko に触らない）とは重ならない。
- **優先度の区分**: B（数えた字が減って次の口パクが 1 回遅れうる）。
- **要件定義のモデル**: Opus（今の刻みの上の数えの直しとテストだけ。Constraints の「Fable」は古い）。
- **分割の案**: 無し。小さすぎるなら、件 8・件 5 を `seriko-trigger-lottery-gaps` へ件として移してこの spec を閉じる手もある（ただしあちらは今の版の外）。
- **見つけた穴・古くなった記述**: Constraints の「先に要るものは `seriko-trigger-intervals` の着地だけ」「要件定義のモデル: Fable」「規模 M」、申し送りの「区分 B で先に着地する見込み」、Scope・Boundary の起点と時計の項（`emo2_boot/mod.rs` の `seriko_clock`）は古い。roadmap の台帳の行（378 行目）の Problem の列も、済んだ 1〜4・6・7 を並べたまま。
