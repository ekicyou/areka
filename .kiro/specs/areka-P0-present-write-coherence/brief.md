# Brief: areka-P0-present-write-coherence

> **📌 2026-08-22 起票（`areka-P0-dpi-transition-atomicity` の task 7.3 実機サインオフからの分離）**
> atom は要件 4.2（窓矩形と描画内容が食い違う中間フレームを可視化しない）を**決定論では満たしたが実機では満たしていない**。
> 本 spec はその残りを引き受ける。**atom の候補表 C8 の B-3／B-4 が本 spec の出発点**であり、
> 候補の中身・接触集合・対テストの形は既に atom の設計に書かれている（後述「Upstream」参照）。

## Problem

拡大率を切り替えたとき、**1 フレームだけ「絵は旧寸のまま、窓の位置だけ新しい」状態が画面に見える**。
開発者の実機目視（2026-08-22・task 7.3 の採取中）で確認された。

これは要件 4.2 が名指しで禁じている形——「窓矩形と描画内容（サーフェス寸）が食い違う状態を可視フレームとして提示しない」——そのものである。

> **⚠ 「どちらが先に見えるか」は未特定である。** 上の記述は**開発者の目視申告のまま**であり、機械で裏づいてはいない。アプリ側のログの時刻順は**寸が先・位置が後**（可視化 22,297〜73,104 に対し窓書込 253,890〜339,998）で、申告とは向きが逆である。この不一致は atom 確定台帳 §3.9 が是正前の採取で登記し、§11.3 が是正後の採取でも同じ向きで再現したことを記録している。**可視化の記録時刻と実際に画面へ提示された時刻は一致しない**（DWM の合成と提示が挟まる）ため、ログだけでは名指しできない。**本 spec でも順序を断定せず、必要なら提示側の観測点を足すところから始めること。** 確定しているのは「食い違いが 210,241〜271,320µs 続く＝提示フレーム 12 枚以上をまたぐ」ことだけである。

痛みの度合いは小さい。開発者の所見は「まあこれはよい」「バルーン位置は人が再調整すればよい」「拡大率の切り替えは滅多に起こらない事象」であり、
**M1 の完成を妨げない**と裁定された（2026-08-22）。本 spec は「直せるなら直す」ための追跡先である。

## Current State

> **数量の正本は atom の確定台帳 `.kiro/specs/completed/areka-P0-dpi-transition-atomicity/mechanism-ledger.md` §11 である。**
> 本節の値は 2026-08-22 に atom の task 7.3 が両採取の生ログを再走査して確定させた実測へ差し替えてある（初稿の 5 箇所は遷移 #1 だけの値を 8 遷移の域として書く等の誤りだった）。以後この表を更新するときは台帳 §11.3・§11.6 と突き合わせること。

atom（`areka-P0-dpi-transition-atomicity`）が着地させたもの:

- **逐次適用は消えた**。1 回の遷移で 4 窓の窓書込が散らばる幅（`last_write_t_us − first_write_t_us`）は **93,152〜157,684µs → 40〜101µs**（約 1,500〜2,000 分の 1）。出所は `atom-71-recapture-1`（B-2b **前**・7 遷移）と `atom-73-signoff-1`（B-2b **後**・8 遷移）。
- 決定論テストは 8 遷移すべて PASS（同一フレーム・窓ごと 1 回・経路 A 0・接地点差 0・連鎖 1 回・随伴の同一フレーム性）。

残っているもの——**可視化と窓書込のあいだの隙間**:

| 実測（2026-08-22・release・遷移 8 回・`atom-73-signoff-1`・全 8 遷移の域） | 値 |
|---|---|
| 絵が新寸になる（`kind=surface stage=visualize` の `t_us`・32 件） | 22,297 … 73,104 |
| 窓が動く（`kind=write` の `t_us`・32 件） | 253,890 … 339,998 |
| **その隙間**（遷移ごとに `visualize` の最遅 → `write` の最速） | **210,241 … 271,320µs（210〜271 ミリ秒）** |
| 判定器の `visualize_to_write_us`（8 遷移 × 4 窓＝32 窓） | 210,329 … 306,301µs（上限 16,667µs の **12.6〜18.4 倍**・上限以下の窓は 1 つも無い） |
| `flush_total_us`（8 遷移） | 143,231 … 231,910µs |

> 初稿が書いていた「絵が新寸 22,297…48,845 ／ 窓が動く 272,491…272,551 ／ 隙間 約 224 ミリ秒」は、**遷移 #1 だけ**の値である（8 遷移の域ではない）。遷移 #1 の隙間 223,646µs はその 1 本の値として正しい。

**B-2b（`DeferWindowPos` 一括）は隙間を縮めなかった**。`flush_total` の平均は **192,247 → 188,711µs（−1.8%）**で、
OS 側のコストは一括化では減らない（atom 確定台帳 L7「過半は OS 側」）。

むしろ全窓の書込がバッチ末尾へ揃ったため、**早い窓の `visualize_to_write_us` は伸びた**（平均 **201,478 → 255,345µs**・+27%。
是正前は `atom-71-recapture-1` の 26 窓、是正後は `atom-73-signoff-1` の 32 窓）。
これは欠陥ではなく**同時性と引き換えの設計上の帰結**だが、要件 4.2 の観点では隙間が均一に長くなったことを意味する。

> **その裏取りに使った Σ`call_us`／`total_us` ＝ 99.82〜99.92% は「是正前」の値である**（`atom-71-recapture-1` の遷移 7 回・atom 確定台帳 §10.3 ⑵）。**是正後の同比は 6.0〜18.1%**（`atom-73-signoff-1` の遷移 8 回）だが、**これは OS 側のコストが減ったという意味ではない**——B-2b 以後の `call_us` は `DeferWindowPos` への**投入だけ**の所要であり、窓が実際に動く時間は 1 区間ぶんまとめて `flush stage=end` の `total_us` に入る（atom 確定台帳 §10.6.1）。**`call_us` の意味が是正前後で違うので、この比を是正前後で比べてはならない。** 比べられるのは `total_us` と `in_batch` である。

**目視の症状と機械判定は食い違ったまま引き継ぐ。** 判定器の `mismatch_frames` は 32 窓すべて 0 だが、
それは**アプリ側の tick 内順序**を測る量であり、目視が見ている**合成器の提示順序**とは別の量である（台帳 §11.2）。
さらに**見え方の順序そのものが未特定**である——ログの時刻順は「寸が先・位置が後」だが開発者の申告は「位置が先・寸が後」で、
向きが逆のまま是正前後の 2 採取で再現している（台帳 §3.9 と §11.3）。**本 spec でもこの順序を断定しないこと。**

## Desired Outcome

拡大率の遷移で、**絵と窓が同じ提示フレームで揃う**——旧寸の絵が新しい窓矩形の中に見える中間フレームが、実機の目視で認められない。

機械側の完了条件は atom の判定器がそのまま使える: **`visualize_to_write_us` が実機専用の上限（16,667µs＝提示 1 コマ）以下**。
判定器・観測語彙・サインオフ手順書はすべて atom が着地させたものを流用する（新設不要）。

## Approach

**atom 設計 C8 の候補表の梯子を、B-2b の次の段から続ける。** 候補は表の内側に限る（atom 要件 3.4 の縛りを引き継ぐ）。

- **B-3 可視化の 2 相化**（第一候補・隙間を直接潰す唯一の手）
  `Present`／`set_visible`／`set_bounds` を窓書込の直前まで遅らせる。**アプリ側のログ上の順序**は「絵を出してから 210〜271ms 後に窓を動かす」（8 遷移の域・台帳 §11.3）なので、これを近接させる。**画面での見え方の順序は未特定**なので「反転させる」とは言わない（同 §3.9）。
  atom 設計は **「最後の手段」** と位置づけ、**「採用が tick 構造の大改造に及ぶ場合は要件 9.3 に従い分割を再裁定する」** と条件を付けている。
  本 spec の要件段階で**まず規模を見積もり、大改造なら分割を再裁定する**こと。
- **B-4 窓内下端中央補償**（第二候補・隙間は消さず見た目を無害にする）
  遷移中、サーフェスの visual を窓内で下端中央に置く（オフセット `((win_w−surf_w)/2, win_h−surf_h)`）→ 窓書込後に原点へ戻す。
  キャラが足元で固定されるので「跳ね」に見えなくなる。**当たり判定の原点（αマスク）と `mount.rs` の配置契約に触れる**ため、
  採用時は atom 要件 10.1 の再確認と `collision-dpi-hittest`（completed）の成果を壊していないことの確認が要る。

**推奨は「まず B-3 の規模を測ること」。** 隙間を消せるのは B-3 だけで、B-4 は緩和に過ぎない。
ただし B-3 が tick 構造の大改造に及ぶなら、開発者裁定 2026-08-22（「大改造が必要なら無理に治さなくて良い」）に従い **B-4 へ落とすか、着手そのものを見送る**。

## Scope

- **In**:
  - 可視化（`set_visible`／`set_bounds`／`Present`）と窓書込のあいだの提示順序
  - `crates/areka-emo-present/src/presenter/show.rs` の `apply_show` 末尾（可視化の段）
  - B-4 を採る場合は visual のオフセット（窓内配置）と、その一時性（窓書込後に原点へ戻す）
  - 実機での `visualize_to_write_us` の再採取と、是正前後の比較
- **Out**:
  - **窓書込そのものの回数・順序・同時性**（atom が着地済み。B-2a 合流・B-2b バッチ・整合ゲート・連鎖再解決はいずれも本 spec の対象外）
  - **OS 側の `SetWindowPos`／`EndDeferWindowPos` の所要**（確定台帳 L7＝過半は OS 側。減らす手は候補表に無い）
  - 観測チャネル・レコード語彙・判定器・サインオフ手順書の**新設**（atom の着地物をそのまま使う）
  - フレーム駆動の CPU 負荷（`draw-load-parity` の担当）
  - バルーン追従オフセットの k 倍（`balloon-offset-dpi` の担当）

## Boundary Candidates

- **提示の順序**（`apply_show` 内で可視化をいつ行うか）と **窓書込の駆動**（`emo2_frame_system` の相順）は別の責務——前者だけを本 spec が持つ
- **visual の窓内オフセット**（B-4）と **当たり判定の原点**（`collision-dpi-hittest` が確定済み）は別の権威——後者は読むだけで変えない
- **遷移中の一時的な配置**と **定常の配置契約**（`mount.rs`）は別——一時性の解除点を明示的に持つ

## Out of Boundary

- atom が確定させた窓書込の形（`SetWindowPosCommand` のタグ・合流・バッチ・`in_batch`／`via` の語彙）を変えない
- 当たり判定の原点（αマスク）と追従の基準（窓左上相対・追従オフセット非補正）を変えない
- 定常状態のアロケーション 0 と段階別計時ログの発行を壊さない（`recompose-budget` の成果）
- tick 構造（13 スケジュールの順序）へ手を入れる場合は、**本 spec の内側で勝手に決めず要件 9.3 で分割を再裁定する**

## Upstream / Downstream

- **Upstream**:
  - `areka-P0-dpi-transition-atomicity`（**候補表 C8・観測チャネル・判定器・サインオフ手順書・確定台帳の全部が前提**。
    B-3／B-4 の内容・接触集合・7.3 の対テストの形は同 spec の `design.md` の C8 と `mechanism-ledger.md` §10 に書かれている）
  - `areka-P0-recompose-budget`（completed・`apply_show` の予算域と段階別計時。**隣接**）
  - `areka-P0-collision-dpi-hittest`（completed・当たり判定の原点。B-4 を採る場合の制約）
  - `areka-P0-test-cage-determinism`（W6.9・`apply_show` 鎖の檻。**本 spec は cage の後**——同じ鎖を触るため）
- **Downstream**:
  - `areka-P0-emo2-conformance-e2e`（W7・適合一周走行。本 spec が M1 後なら e2e は現状の見た目で走る）

## Existing Spec Touchpoints

- **Extends**: なし（atom は本 spec の起票をもって 4.2 の実機側を手放す。atom 側には「実機で未達・引受先は本 spec」を登記する）
- **Adjacent**:
  - `areka-P0-test-cage-determinism`（W6.9・`presenter/show.rs` `apply_show` 鎖の最後尾を自称。**本 spec は cage の後に置く**）
  - `areka-P0-draw-load-parity`（W8・フレーム駆動の負荷。同 brief が「`presenter/show.rs` は対象外」と明記しているので責務は重ならない）
  - `areka-P0-balloon-offset-dpi`（W6.75・バルーン追従オフセットの k 倍。**同じ「100% で見た目が変」でも機序が別**——あちらは offset が k 倍されない話、こちらは提示のタイミングの話）

## Constraints

- **開発者裁定 2026-08-22**: 「大改造が必要なら無理に治さなくて良い」。**M1 の完成を妨げない優先度**（`draw-load-parity` と同格）。
- atom 要件 3.4 の縛りを引き継ぐ——**採用候補を C8 の表の外へ広げない**。
- atom 要件 9.3——**tick 構造の大改造に及ぶなら分割を再裁定する**。
- 実機でしか測れない量（`visualize_to_write_us`・`flush_total_us`）が主たる判定量。**決定論だけでは合否が出ない**ので、
  atom のサインオフ手順書（`signoff-procedure.md`）に従った実機採取が要件に入る。
- 実機採取は **OS 設定で拡大率を往復させる人手の作業**であり、エージェントには実行できない（atom の task 7.1／7.3 で実証済み）。

---

## 申し送り（areka-P0-draw-load-parity・2026-08-23）

`areka-P0-draw-load-parity`（W6.9）が tick の周期・構造に加えた変更の報告（同 spec 要件 8.3）。本 spec（W6.95）は着手時にこの形へ rebase し、実機の µs 判定を読むときの前提として扱うこと。

**⑴ tick に「門」が入った（既定 OFF）**

- 画面更新 1 回ごとに、変化を示す旗が 1 つも立っていなければ 13 本のスケジュールを回さない、という判断を手前に挟む。判断は純関数 `tick_gate::should_run`（`crates/wintf/src/ecs/world/tick_gate.rs:154`）・つなぎは `EcsWorld::decide_tick`（`crates/wintf/src/ecs/world/mod.rs:551`）・入口は `tick_one_frame_with`（`crates/wintf/src/runtime/tick_bridge.rs:230`）。
- **既定は OFF**（`world/mod.rs:405` の `tick_gate_enabled: false`）＝門を入れる前と同じ挙動。`AREKA_TICK_GATE=1|0`（`crates/areka/src/tick_gate_config.rs:25`）で同じ実行体のまま切り替えられ、A/B 比較と安全弁を兼ねる。既定を ON にするかは改善ループの周 1 の A/B が決める。
- 必ず回す条件＝門が無効／起動から 600 回未満（`TICK_GATE_WARMUP_FRAMES`）／旗が 1 つでも立っている／期限到来／前回回してから 30 回（`TICK_HEARTBEAT_FRAMES`＝省略 30 回の次＝31 回目が心拍で回る・約 3.9 回/秒）。未知の窓メッセージは「疑わしいときは回す」側へ倒す。

**⑵ 省略した回に何が起きないか（前提が変わる箇所）**

- `FrameCount`／`FrameTime`／`TickStart` は**進まない**し、スケジュールは 1 本も回らない（`EcsWorld::note_skipped_tick`＝`world/mod.rs:593`）。フレーム番号や `FrameTime` を時間の代わりに読む観測・判定は、門が ON のとき意味が変わる。
- `flush_window_pos_commands()` は**省略した回も必ず呼ぶ**（`tick_bridge.rs:258`）＝窓書込指令の一括 flush の駆動は不変。13 本の順序と `try_tick_world` の中身も不変（門は手前にある）。
- 変化が生じたら次の画面更新までに反映する（遅れの上限は 1 画面更新周期＝120Hz の実機で約 8.3ms）。

**⑶ 旗を立てる側（起床の生産者・全て 1 行）**

- wintf: 窓メッセージ配送点（`ecs/window_proc/mod.rs`）・ポインタ投入（`ecs/pointer/buffers.rs`）・窓書込指令の積み上げ（`ecs/window/command.rs`）・Z 順（`ecs/window/zorder_pair_maintain.rs`）・ドラッグ（`ecs/drag/systems.rs`）・dola アニメ（`ecs/dola/mod.rs`）・GraphicsCore 無効（`ecs/graphics/systems/init.rs`）・表示構成の変化（`ecs/app.rs`）。
- areka: 表示指令の到着（`emo2_boot/adapter.rs`・`move_cue.rs`・`talk_lifecycle.rs`）・文字の進行（`emo2_boot/frame/scale_text.rs`）・バルーンの待ち時間（`emo2_boot/balloon_visibility_phase.rs`）・`emo2_boot/hover_inject.rs`。旗は `tx.send` の**後**に立てる。`sinks` の順序（clocked_text_sink → lifecycle_sink）を保つこと。

**⑷ 観測の口が増えた（いずれも既定 OFF・点けなければ費用 0）**

- `[tick] kind=window frame= t_ms= ticks= skipped= heartbeat= wall_us= max_us= ui_cpu_us=` ＋ 13 本の相別 `<相>_us=`（1 秒窓で 1 行・`crates/wintf/src/ecs/world/tick_diag.rs:133`・target `wintf::tick`）。省略率はこの行の `skipped=` で読む。
- `perf(thread)`／`perf(process)`（スレッド別・プロセス全体の CPU・`crates/areka/src/perf_thread_report.rs:51`・target `areka::perf`）。
- 既存の `perf(apply_show)`（末尾 `frame`）と `[transition]` の文言・フィールド名は不変で、新しい行とは重ならない。

**⑸ 本 spec に効きうる点**

- 判定量 `visualize_to_write_us`／`flush_total_us` は壁時計であり、門の ON/OFF で意味は変わらない。ただし**同じ遷移が跨ぐフレーム数は門が ON だと減る**ので、フレーム番号でまとめる集計（`[transition]` の `frame=` を鍵にする突合）は再確認が要る。
- 可視化→書込の隙間そのものを門は縮めない（省略した回も flush は呼ばれる＝上の⑵）。dlp は `presenter/show.rs`・`mount.rs` に触っていない。

**dlp の合否に載せない申し送り（憶測で埋めないこと）**: 遷移フレームのうち自前の窓手続きが 1 行も走っていない**未特定区間 47.5%**（639,106／1,344,271µs・中央値 18,059µs）と、文字層の再構築の所要。どちらも dlp は測るだけで合否に載せない。

**まだ動く**: dlp は改善ループ（タスク 9.x）が続いており、最終の着地（門の既定 ON/OFF・tick 構造がさらに変わったか・`Cargo.toml` に触れたか＝現時点では非接触）は同 spec のタスク 9.4 が本節を更新して報告する。
