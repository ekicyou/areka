# Brief: areka-P0-balloon-text-bugs

> 2026-10-11 `/kiro-discovery` で起票（開発者の指示「バグの起票は粒度が小さい。修正範囲がかぶらない範囲でいくつかに集約」）。文字の層（`crates/areka-emo-text`）に集まるバグの起票 3 本を 1 本にまとめた。元の brief は下の「件 1〜3」に全文を写してある（見出しを 1 段下げただけ・中身は変えていない）。場所と行数は元の起票と棚卸㉓（main `ee3af616`）のときのもの＝着手時に引き直す。

## Problem

バルーンの文字まわりに、小さいバグが 3 件ある。どれも文字の層の同じファイル（`actor_present.rs` ほか）に触るので、別々の spec にすると互いに順番待ちになり、要件から完了までを 3 回くり返すことになる。

- **件 1（旧 `choice-balloon-timeout-stuck`）**: 選択肢を含む台詞のバルーンが、選んだ後も・選択肢の時間切れで解除した後も時間切れで消えず、`OnBalloonTimeout` も届かない。
- **件 2（旧 `balloon-text-area-collapse`）**: バルーンの文字の描画範囲が 0 以下に潰れると、文字の層が毎フレーム供給面の生成に失敗し、WARN 1 行と ERROR 2 行をゴーストが終わるまで出し続ける。
- **件 3（旧 `underline-bottom-row-clip`）**: 文字の領域（validrect）の下端に来た行の下線が出ない（作者の `\f[underline,1]` も、アンカーの下線も）。

## Current State

- 3 件とも直っていない（件 1 はソースの読み・件 2 は実機の記録・件 3 は実機の撮影 2 枚で確かめた）。細部は各件の節。
- 触るファイルの重なり（棚卸㉓と各 brief から）:
  - `crates/areka-emo-text/src/actor_present.rs` ＝ 件 2（装着の枝で 0 寸を見分ける案）と件 3（面の大きさと validrect の関係）。
  - `crates/areka-emo-text/src/{actor,state}.rs` ＝ 件 1（文字の層で直す案 B のとき）。
  - 件 1 を出し入れの側（`crates/areka/src/emo2_boot/balloon_visibility_wait.rs`）で直す案 A なら、件 1 は文字の層に触らない。
  - 件 3 はほかに `viewbox_draw_decoration.rs`・`viewbox_diff.rs`・`layout.rs`、件 2 はほかに `region.rs`・`surface.rs`（995 行＝上限の近く）・`crates/areka/src/emo2_boot/frame/scale_text.rs`。

## Desired Outcome

- 件 1: 選択が済んだ（選んだ・時間切れで解除された）バルーンは、ほかのバルーンと同じに時間切れで消え、`OnBalloonTimeout` が届く。
- 件 2: 0 寸の描画範囲では供給面を作らず描かない。記録は状態が変わったときの 1 回。普通のバルーンでもシェルの中のバルーン（箱）でも同じ。
- 件 3: 文字の領域の下端の行でも下線が見える。
- 3 件それぞれに決定論のテストが付き、実機で確かめてある。

## Approach

件ごとに要件を立て、設計で直す場所を選ぶ。直し方の候補は各件の節に在る（件 1＝案 A／案 B・件 2＝止める層 4 つ・件 3＝3 案で未決）。**まとめたことで、同じファイルを触る直しを 1 つのワークツリーの中で順に入れられる**（件 2 と件 3 の `actor_present.rs`）。順の目安は 件 2 → 件 3 → 件 1（件 2 が一番狭く、件 3 の面の大きさの議論の土台になる。件 1 は案 A なら独立）。

## Scope

- **In**: 上の 3 件の直し・それぞれの決定論のテスト・実機の確かめ・決めを変えたときの互換の記録（`doc/COMPAT_ARCHITECTURE.md` §8）。
- **Out**: 各件の節の Out のとおり。主なもの＝選択肢の見た目（`choice-marker-styling`）・範囲の選択肢（`range-choice-tag`）・アンカーの見た目の作者指定と縦書きの下線（`anchor-style-canon`）・あふれのスクロールの見せ方（`balloon-scroll-fade`）・折返し基準と描画範囲の意味論（完了 `emo-text-line-height-canon`）。

## Boundary Candidates

- 件 1: 「選択待ち」の持ち方（文字の層）／可視性の相が読む観測（出し入れの側）。
- 件 2: 0 寸を見分ける層（`actor_present.rs`・`region.rs`・`surface.rs`・`scale_text.rs`）。
- 件 3: 下線の位置を誰が決めるか／文字の面の大きさと validrect の関係／入る行の数え方。
- 件どうしは働きの上で独立＝タスクと査読の単位は件ごとに切れる。

## Out of Boundary

- 押せる範囲の作り方（最下行のアンカーは今も押せる＝変えない）。
- バルーンの定義の読み方（余白の値そのもの）。
- 3 つのイベントの送り方（完了 `balloon-lifecycle-events`）・選択肢の時間切れそのもの（完了 `choice-timeout-directive`）。

## Upstream / Downstream

- **Upstream**: `balloon-lifecycle-events`（✅ 10-08）・`anchor-tag-canon`（✅ 10-10）・完了 `text-decoration-canon`。どれも着地済み＝今すぐ着手できる。
- **Downstream**: `anchor-style-canon`（件 3 が下線の位置を自分で決める形を選ぶなら、その上に乗る）。選択肢を使う実ゴーストの適合。

## Existing Spec Touchpoints

- **Extends**: 完了 `balloon-visibility`（抑止の条件）・完了 `choice-timeout-directive`・完了 `text-decoration-canon`（件 3 が下線の決めを改めるとき）。
- **Adjacent**: 直列の列「文字とバルーン」（`crates/areka-emo-text/src/` の `actor*.rs`・`state.rs`・`viewbox_draw*.rs`）＝同じ列の spec と同時に走らせない。近いのは `range-choice-tag`・`choice-ranges-one-function`（`choice.rs`・`actor_present.rs`・`actor.rs`）・`anchor-style-canon`（`viewbox_draw_decoration.rs`）・`balloon-font-file`（`actor_present.rs`）・`balloon-canon-residue`（`scale_text.rs`）・`seriko-talk-clock-fidelity`（式を共有にする形を選ぶと `state.rs`）・`clippy-199-lints`（`balloon_visibility_*`）。

## Constraints

- 段は**バグ**・区分 B。
- 規模の見立て: 11〜20 タスク（件 1＝3〜7・件 2＝4〜6・件 3＝4〜7）。**上限 20 を超える見込みになったら、要件の段で件 3（直し方が未決で幅が広い）を切り出す**。
- `surface.rs` は 995 行。6 行以上足すなら先に中の検査を兄弟のファイルへ出す＝`lib.rs` の純粋な一覧に触る（新しいファイルを足す spec と同じウェーブに置けない）。
- 1 ファイル 1,000 行未満・決定論のテスト必達・1 フレーム遅らせる解は取らない・GPU のテストは実 GPU・ログ無しの失敗にしない。
- 要件定義のモデル: Fable（件 1 の合図の届く順と層の分かれ目・件 3 の 3 案の選び）。

---

## 件 1: 旧 `areka-P0-choice-balloon-timeout-stuck`（元の brief の全文）

> 起票: 2026-10-05（`areka-P0-balloon-lifecycle-events` の設計で発見。範囲の外の問題として `/kiro-discovery` の決まりで起票）。ソースの読みで確かめた。実機ではまだ確かめていない。

### Problem

選択肢を含む台詞のバルーンが、選んだ後も、選択肢の時間切れで解除された後も、時間切れで消えない。利用者には、用の済んだ選択肢つきのバルーンが次のトークまで出たままに見える。ゴーストの作者には、その台詞について `OnBalloonTimeout` が届かない。

### Current State

2026-10-05 時点（`balloon-lifecycle-events` のブランチ・main `04565148` を取り込んだ後）:

- 時間切れの待ちを止める条件の 1 つ「選択肢が表示中か」は、文字の層の `TextLayerRuntime::choice_active`（`crates/areka-emo-text/src/actor.rs`）が答える。中身は「そのスコープのどこかの場所に選択肢の行が残っているか」。
- 選択肢の行は内容の消去（`\c`・次のトークの冒頭の全消去）でしか消えない。選んだこと・選択肢の時間切れで解除したことでは消えない。
- そのため、選択の応答が台本を返さなかったとき（204）や、選択肢の時間切れの応答が 204 のとき、抑止が効き続ける。可視性の相（`crates/areka/src/emo2_boot/balloon_visibility_phase.rs` が `choice_active` を観測へ写す所）は時間切れを成立させない。
- `balloon-lifecycle-events` は抑止の条件を範囲の外に置いた。実機の確認の検体は選択肢の無い台詞で組む。

### Desired Outcome

選択が済んだ（選んだ・時間切れで解除された・中断された）台詞のバルーンは、選択肢の無い台詞と同じ決まりで時間切れになり、`OnBalloonTimeout` が送られる。選択を待っている間は今までどおり消えない。

### Approach

「選択肢が表示中か」を「行が残っているか」でなく「その選択がまだ生きているか」で答える形にする。候補は 2 つあり、要件・設計で選ぶ。

- 表示の側で、選択が済んだ合図（選んだ・解除された）を受けたら「選択待ち」の印を下ろす（行は残してよい）。
- 選択が済んだら選択肢の行を選べない字へ変える（見た目をどうするかは ukadoc に記述があるかを先に調べる）。

### Scope

- **In**: 選択が済んだ後の抑止の解除・それを固定する決定論のテスト・実機での確認。
- **Out**: 選択肢の時間切れそのもの（完了 `choice-timeout-directive`）・時間切れの判断の他の規則・3 つのイベントの送り方（`balloon-lifecycle-events`）。

### Boundary Candidates

- 文字の層の「選択待ち」の持ち方／可視性の相が読む観測／kanade の選択の帳簿との対応。

### Out of Boundary

- 選択肢の見た目の変更（`choice-marker-styling`）・範囲の選択肢（`range-choice-tag`）。

### Upstream / Downstream

- **Upstream**: `areka-P0-balloon-lifecycle-events`（計測の成立条件を「トークの終わりが届いてから」に変える。その後の形の上で直す）。
- **Downstream**: 選択肢を使う実ゴーストの適合。

### Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-balloon-visibility`（抑止の条件）・完了 `areka-P0-choice-timeout-directive`。
- **Adjacent**: 文字とバルーンの列（`crates/areka-emo-text/src/actor*.rs`・`state.rs`）＝同じ列の spec と同時に走らせない。

### Constraints

- 要件で決める未知: 選択が済んだ合図を表示の側がどこから受け取るか（選んだときは UI が自分で知っている。時間切れの解除と中断は kanade が止める）。
- 規模の見込み: S（4〜7 タスク）。
- 1 ファイル 1,000 行未満・決定論のテスト必達・1 フレーム遅らせる解は取らない。

### 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**:
  - `balloon-lifecycle-events` が着地した（10-08）。時間切れの計測は「トークの終わりが表示の側へ届いてから」だけ始まる（`crates/areka/src/emo2_boot/balloon_visibility_wait.rs` の `decide_timeout`）。
  - kanade は選択待ちを進行中のトークに結び付けているので、選択を待つ間はトークが終わらない見立て（設計で確かめる）。そうなら、待っている間は「選択肢が表示中」の抑止が無くても計測は始まらず、この抑止が効くのはトークが終わった後＝本 brief の不具合の場面だけになる。
  - 不具合そのものは残っている。`crates/areka-emo-text/src/actor.rs` の `TextLayerRuntime::choice_active` は今も「選択肢の行が残っているか」で答える。
- **触るファイル**:
  - 案 A（今回見えた直し方）＝抑止の数え方だけを直す: `crates/areka/src/emo2_boot/balloon_visibility_wait.rs`（289 行・抑止を数える `observe_suppression`）と可視性の検査（`balloon_visibility_timeout_suppression_cases.rs` ほか）。emo-text には触らない。
  - 案 B（本 brief の 2 案）＝文字の層の「選択待ち」の持ち方を直す: `crates/areka-emo-text/src/{actor.rs, state.rs}` と、同じ問いを使う入力の 4 か所（`crates/areka/src/input_events/` の `balloon_moved.rs`・`balloon_pressed.rs`・`balloon_exit.rs`・`shell_box_handler.rs`）。
- **規模**: S（案 A 3〜5・案 B 5〜7 タスク）。
- **先に要るもの**: `balloon-lifecycle-events`（着地済み）。
- **優先度の区分**: B（バグ。用の済んだバルーンが消えず、`OnBalloonTimeout` も届かない）。
- **要件定義のモデル**: Fable（合図の届く順と時刻の話・どの層で直すかの分かれ目）。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**:
  - 済んだ選択肢の行は、マウスを乗せると光り続け、押すと選択の知らせが kanade へ飛んで警告 1 行（`choice_rejected_no_wait`・`crates/areka-kanade/src/schedule/steady.rs` の `on_choice`）で捨てられる。案 A だけではこれが残る＝要件で扱うかを決める。
  - 案 A なら `anchor-tag-canon` と触るファイルの重なり 0＝同じウェーブに置ける。案 B は `actor.rs`・`state.rs` が重なる＝前か後。
  - roadmap の「文字とバルーンの列」という置き場所は、案 A なら当たらない。`clippy-199-lints` が `balloon_visibility_*` に触る。実機ではまだ確かめていない。

---

## 件 2: 旧 `areka-P0-balloon-text-area-collapse`（元の brief の全文）

> 2026-10-07 `animated-image-playback` の完了時の棚卸で起票（`completed/areka-P0-animated-image-playback/tasks.md` の Implementation Notes の 6.3、`research.md`「実機の確かめ」の「範囲外（要起票）」1）。

### Problem

バルーンの文字の描画範囲が 0 以下に潰れると、文字の層が**毎フレーム**供給面（swap chain）を作ろうとして失敗し、WARN 1 行と ERROR 2 行をゴーストが終わるまで出し続ける。バルーンを隠しても止まらない。記録が洪水になり、ほかの記録が読めなくなる。

### Current State

- 実機の再現（10-06）: 8×8 のバルーンの面に `emo2-kakukaku` の余白（36〜56 px・面より大きい）を当てると、`WARN areka_emo_text::region: 解決後の validrect が退化している（幅/高さ ≤ 0）… left=36.0 top=46.0 right=-36.0 bottom=-48.0` に続いて、`ERROR areka_emo_text::surface: D3D/DXGI/WUC 呼び出しが失敗 hresult=-2005270527 context="create_composition_swap_chain"`（0x887A0001＝DXGI_ERROR_INVALID_CALL）と `ERROR areka::emo2_boot::frame::scale_text: emo2 text: present_frame が失敗（…次フレーム再試行…）` がフレームごとに 1 組出る。
- 供給面は `crates/areka-emo-text/src/surface.rs` の `create` が `create_composition_swap_chain(&d3d, &dxgi, w, h)` で作る。0 寸の面を作らずに済ます分岐は無い。
- `animated-image-playback` の前の実行体でも出る（本 spec が持ち込んだものではない）。
- 別の不具合: `emo-text-canon-residue` の項目 14（折返し基準が描画範囲の外に解決される）は折返しの話で、こちらは供給面の生成の話。

### Desired Outcome

- 描画範囲が 0 以下なら供給面を作らず、文字を描かない（正典の validrect の外には描かない）。
- 記録は「潰れた」「戻った」の状態が変わったときに 1 回だけ出す。毎フレームの ERROR は 0 行。
- 範囲が戻れば（面やバルーンの切り替え）、次のフレームから普通に描ける。

### Approach

0 寸を「失敗」ではなく「描くものが無い状態」として持つ。どの層（`region` の解決・`surface` の生成・`scale_text` の再試行）で止めるかは要件で決める。決定論のテストで「潰れた範囲では供給面を作らない・記録は 1 回」を固定する。

### Scope
- **In**: 文字の層の 0 寸の扱い・記録の回数・決定論のテスト。
- **Out**: 折返し基準と描画範囲の意味論（`emo-text-line-height-canon` で確定）・項目 14。

### Boundary Candidates
- `crates/areka-emo-text/src/{region,surface}.rs`
- `crates/areka/src/emo2_boot/frame/scale_text.rs`（再試行）

### Out of Boundary
- バルーンの定義の読み方（余白の値そのもの）

### Upstream / Downstream
- **Upstream**: なし
- **Downstream**: 小さい面のバルーンを持つすべてのゴースト

### Existing Spec Touchpoints
- **Extends**: なし
- **Adjacent**: `emo-text-canon-residue`（項目 14）・`balloon-canon-residue`（項目 14）

### Constraints
- ログ無しの失敗にしない（error! か warn! を 1 回は出す）。
- 規模の見立て: S（4〜7）。

### 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**:
  - `mcp-dump-images-residue` が供給面のファイル `crates/areka-emo-text/src/surface.rs` に読み戻しを足し、995 行になった（上限は 1,000）。ここへ 6 行以上足すなら、先に中の検査を兄弟のファイルへ出す＝`lib.rs` の一覧に触る。
  - 不具合はそのまま残っている。面の大きさは `crates/areka-emo-text/src/actor_present.rs` の `present_actor` の「初めての装着」の枝で領域から計算し、0 でもそのまま装着を呼ぶ。失敗すると装着済みにならないので、次のフレームでまた同じ枝に入る。
- **触るファイル**:
  - 止める場所の候補は、本 brief の 3 つに加えて **`actor_present.rs`**（装着の枝の手前で 0 寸を見分けるのが一番素直）。
  - `crates/areka-emo-text/src/region.rs`（486 行・潰れたときの警告）・`surface.rs`（995 行）・`crates/areka/src/emo2_boot/frame/scale_text.rs`（326 行・失敗の記録と次のフレームの再試行）。
  - 検査は既存のファイル（`actor_tests.rs` 178 行など）に足せば新しいファイルは要らない。`actor_region_warn_tests.rs` は 894 行で足しにくい。
- **規模**: S（4〜6 タスク）。
- **先に要るもの**: 無い。
- **優先度の区分**: B（バグ。記録が毎フレーム出続けて、ほかの記録が読めなくなる）。
- **要件定義のモデル**: Opus。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**:
  - Boundary Candidates に `actor_present.rs` が無い（装着を呼ぶのはここ）。
  - `anchor-tag-canon`・`choice-ranges-one-function` と `actor_present.rs` が重なる＝そこで止める設計なら同じウェーブに置けない（前か後）。`surface.rs` の側で止めるなら検査の切り出しで `lib.rs` に触る＝新しいファイルを足す spec と同じウェーブに置けない。
  - 重なりを 0 にできるのは、領域を登録する所（`actor_attach.rs`・`actor_box.rs`）か `scale_text.rs` で止める形だけ＝設計で選ぶ。`balloon-canon-residue` も `scale_text.rs` に触る。
  - シェルの中のバルーン（箱）も同じ枝を通るので、同じ直しが箱にも効くことを検査に入れる。

---

## 件 3: 旧 `areka-P0-underline-bottom-row-clip`（元の brief の全文）

> 起票: 2026-10-10（`areka-P0-anchor-tag-canon` の実機の確かめ〔タスク 7.2〕で発見。範囲の外の問題として、完了時の棚卸で `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-anchor-tag-canon/tasks.md` の Implementation Notes「7.2（範囲外・起票する）」と「7.2（結果）」。実機の撮影 2 枚で見た。原因はコードの読みで、コードを変えて確かめてはいない。区分 B（バグ）。

### Problem

- **利用者・ゴーストの作者**: バルーンの文字の領域（validrect）の下端に来た行では、下線が出ない。作者が `\f[underline,1]` で引いた下線も、アンカー（`\_a`）の下線も同じ。
- アンカーは、マウスが乗っていないときの目印が下線だけ（`doc/anchor-compat.md` の 2.1 の 8）。最下行のアンカーは、押せるのに、押せる所だと見分けられない。
- アンカーに固有の問題ではない。下線の描画は完了 `areka-P0-text-decoration-canon` が入れたもので、`anchor-tag-canon` より前からある。`anchor-tag-canon` でアンカーの既定の見た目が下線になり、利用者に見える場面が増えた。

### Current State

2026-10-10 時点（`anchor-tag-canon` のブランチ）:

- **見たこと**（実機・開発機 1 台・emo2 と同梱のバルーン `emo2-kakukaku`・文字の領域に 4 行が入る台詞）:
  - 4 行目（文字の領域の最下行）: 作者の `\f[underline,1]` の下線も、アンカーの下線も出ない（0 本）。
  - 3 行目: 両方出る。別の撮影では 1〜3 行目のアンカーの下線が出ている。
  - 撮影は 2 枚（MCP の `dump_balloon`）。画像は追跡外の作業用フォルダに置いたので残らない＝要件の段で撮り直す。
  - 最下行のアンカーは、下線が無くても押せた（実マウスで複数回・押下の記録 `anchor_selected` あり）。押せる範囲は描画と別に作るからである。
- **原因の見立て**（確度は高い。コードを変えて確かめてはいない）:
  - 下線の位置は DirectWrite に任せている。`crates/areka-emo-text/src/viewbox_draw_decoration.rs` の `apply_font_ranges` は、区間に `SetUnderline` の真偽を渡すだけで、線の位置を自分で決めない（完了 `text-decoration-canon` の要件 5.7）。
  - Yu Gothic UI の 28 画素では、下線は行の上端から 33 画素の所に出る（文字の大きさの箱 28 より 5 画素下・行送り 30 より 3 画素下）。
  - 文字の面は validrect ちょうどの大きさで作る（高さ 122。`actor_present.rs` の `present_actor` が `ScaleContract::physical_extent` で出す `physical_size`）。描き替える範囲も面の大きさで切る（`viewbox_diff.rs` の `expand_guard_clamp`）。
  - 4 行目の下線は 90 + 33 = 123 で、面（高さ 122）の外になる。途中の行の下線は次の行の帯に落ちるので見える。
  - 「あふれたか」は行の矩形（文字の大きさの箱）の下端で数える（`layout.rs` の `visible_window`）。4 行目の下端は 90 + 28 = 118 で、122 に収まると数えられ、スクロールは起きない。
- 最下行の下線を見る検査は無い（0 本）。
- 確かめていないこと: 拡大率が 100% でないとき・ほかのフォントと大きさ・縦書き（端の列で同じ形になるか）・シェルの中のバルーンの箱・打ち消し線（行の中に出るので当たらない見込み）。

### Desired Outcome

- 文字の領域のどの行でも、下線を引いた字には下線が見える。最下行も同じ。作者の下線とアンカーの下線の両方。
- 途中の行の見た目は変えない。行の位置と「何行入るか」は、選んだ直し方が求める分だけ変え、変えたら互換の記録に残す。
- 決定論のテストで「最下行の下線が面の中に描かれる」を固定し、実機で撮り直して確かめる。

### Approach

直し方は決めていない（**未決**）。候補は 3 つあり、要件・設計で選ぶ。

- **面を広げる**: 文字の面を、下線がはみ出す分だけ validrect より大きく作る。はみ出しの量をどこから得るか、面の位置・当たりの矩形・箱の写しがずれないかを先に確かめる。
- **下線を自分で決めた位置に描く**: DirectWrite 任せをやめ、行の箱の中に収まる位置へ線を引く。完了 `text-decoration-canon` の要件 5.7（線を描き分けない）を改めることになる。
- **入る行の数え方を変える**: 行の下端を、下線まで含めた高さで数える。最下行に下線が無い台詞でも、入る行が減りうる。

### Scope

- **In**: 文字の領域の端の行の下線が見えるようにする直し・それを固定する決定論のテスト・実機の撮り直し・決めを変えたときの互換の記録。
- **Out**: アンカーの見た目の作者指定と、縦書きの下線の位置の決め直し（`anchor-style-canon`）・選択肢の印の「本物の下線」（`choice-marker-styling`）・文字の描画範囲が 0 以下に潰れたときの扱い（`balloon-text-area-collapse`）・あふれのスクロールの見せ方（`balloon-scroll-fade`）。

### Boundary Candidates

- 下線の位置を誰が決めるか（DirectWrite か areka か）。
- 文字の面の大きさと validrect の関係（`actor_present.rs`・`viewbox_diff.rs`）。
- 入る行の数え方（`layout.rs` の `visible_window`）。

### Out of Boundary

- 押せる範囲の作り方（最下行のアンカーは今も押せる＝変えない）。
- 下線の太さ・色・形の作者指定。

### Upstream / Downstream

- **Upstream**: 完了 `areka-P0-text-decoration-canon`（下線の描画）・`areka-P0-anchor-tag-canon`（アンカーの既定の見た目を下線にした。互換の記録は `doc/anchor-compat.md`）。
- **Downstream**: `areka-P0-anchor-style-canon`（アンカーの下線の解決を差し替え、縦書きの下線の位置を決め直す。本 spec が下線の位置を自分で決める形を選ぶなら、その上に乗る）。

### Existing Spec Touchpoints

- **Extends**: なし（完了 `text-decoration-canon` の下線の決めを改めることがある。改めたら `doc/COMPAT_ARCHITECTURE.md` に記録する）。
- **Adjacent**: 文字とバルーンの列（`crates/areka-emo-text/src/` の `actor*.rs`・`viewbox_draw*.rs`）＝同じ列の spec と同時に走らせない。近いのは `anchor-style-canon`（`viewbox_draw_decoration.rs`・`viewbox_draw_render.rs`）・`balloon-text-area-collapse`（`actor_present.rs`）・`choice-ranges-one-function`（`actor_present.rs`）。

### Constraints

- 段は**バグ**。先に要るものは `anchor-tag-canon` の着地だけ（アンカーの下線で確かめるため。作者の下線だけなら今の main でも再現する見込み）。
- 要件で決める未知: 直し方の 3 案のどれにするか・下線のはみ出しの量を拡大率とフォントごとにどう得るか・縦書きと箱で同じ形が起きるか。
- 規模の見込み: S（4〜7 タスク。面を広げる案は当たりの矩形と箱の写しまで確かめるので上の側）。
- 1 ファイル 1,000 行未満・決定論のテスト必達・1 フレーム遅らせる解は取らない・GPU のテストは実 GPU で回す。
