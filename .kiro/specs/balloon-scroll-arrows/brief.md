# Brief: balloon-scroll-arrows

> 2026-10-23 `areka-P0-pasta-in-windows-verification` のタスク 5.3 で `/kiro-discovery` により起票（本 spec からの申し送り（2026-10-23））。開発者の決め（同日のチャット）「0.1.0までに解決しない場合、独立して起票せよ」「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」。`balloon-markers`（版に縛らない列）が棚卸㉑から持っていた切り方の前半（矢印と手動スクロール・装飾の系列の土台）を、v0.1.0 の spec として切り出した。版は **v0.1.0**（ゴールの 2 つ目「ぱすたが SSP と同じに動く」）。

## Problem

- **ゴーストの作者（ぱすた）とその利用者**: ぱすたの縦書きのバルーン `pasta_tategaki` は、スクロールの矢印 `arrow0.png`・`arrow1.png` と位置 `arrow0.x,-12`・`arrow0.y,4`・`arrow1.x,-12`・`arrow1.y,-12` を持つ。areka はこの鍵を読まず、押し出された列が残っていても矢印を出さず、矢印のクリックやホイールで読み返すこともできない。SSP は矢印を出し、戻して読める。

## Current State

- **証拠**: `verification.md` 10 節の拾った一覧（縦書きのバルーンの descript の `arrow0.x/y`・`arrow1.x/y`）と突き合わせの表の「バルーン `arrow0.*`・`arrow1.*`」の欄（absent・持ち主 `balloon-canon-residue`・本 spec の範囲の外のスクロールの矢印）。書庫の中身は `target\pasta-verify\root\ghost\pasta-in-windows\pasta_tategaki\`（`descript.txt` の 69〜72 行目・`arrow0.png`・`arrow1.png`）。
- **SSP の基準**: ぱすたの測り（`ssp-measurement.md`）では、矢印の出る長さの台詞を出していない＝見えた答えは無い。基準は ukadoc と、要件の段で SSP の MCP で測る値。
  - `arrow0.x,座標`「スクロールマーカ上矢印のX座標。」・`arrow0.y`・`arrow1.x`・`arrow1.y`（下矢印）。`arrow.filename`「スクロール用の矢印の画像ファイル名を指定する。拡張子は不要。例えばarrowと設定すると、arrow0(上)・arrow1(下)を自動的に読み込む。」（旧名の段 `arrows`・`arrowk`・`arrowp*def`・SSP 2.5.51）。
  - `vertical,0/1`: 縦書きにすると descript の座標の意味が変わる（列は右から左へ送る）。縦書きの矢印の向き（右／左）は ukadoc の明文が無く、完了 `balloon-vertical-canon` の要件 SC10 が「矢印の説明から間接に推すだけ」と疑義を登記した。
- **areka**: バルーンの読み手 `crates/areka-parsers/src/balloon/` に鍵が無い（`validation_tests.rs` が「accessor が無い」ことを記録）。系列の解決（emo-present の `balloon.rs` の `SeriesFamily`）は本体の系列だけ。スクロールは自動だけで、見える範囲は `crates/areka-emo-text/src/layout.rs` の `LayoutEngine::visible_window`。ホイールは入力の口から無い（wintf の `PointerState.wheel` はあるが areka が読まない）。
- **持ち主の今**: `balloon-markers`（版に縛らない列・L 18〜22）が矢印・手動スクロール・オンラインの印・数字・SSTP の印・装飾の系列を持ち、20 をまたぐので要件の段で「矢印と手動スクロール＋装飾の系列の土台」と「状態の印」に切ると決めていた。網羅台帳の `arrow*` の行の持ち主は `balloon-canon-residue`（`balloon-markers` が 2026-10-01 に項目 1 を引き取った後も台帳は直っていない）。

## Desired Outcome

- 押し出された文字が残っているとき、`arrow0`（横書きは上・縦書きは右の見込み）と `arrow1`（下・左の見込み）の印を、バルーンの絵の左上を原点（負の値は反対の端から）に出す。
- 矢印のクリックとホイールで手動に戻して読める。新しい文字が来たとき・時間切れのときの扱いは要件の段で決める。
- ぱすたの縦書きのバルーン（ゴーストのフォルダの中）で、SSP と同じ所に同じ矢印が出る。
- シェル内バルーンの箱でも同じ規則で出る。

## Approach

- `balloon-markers` の棚卸㉑〜㉓の見立て（触るファイル・議題）をそのまま使い、前半だけを本 spec にする。装飾の系列の土台（`SeriesFamily` に装飾の系列の行・旧名の段・`clickwait*` も同じ仕組みで解ける）を本 spec が作り、後半（`balloon-markers` に残る状態の印）と `talk-fast-forward` が使う。
- 印は文字の面の中で描くか emo-present で重ねるかを要件の段で決める（MCP の `dump_balloon` に写るかが変わる）。

## Scope

- **In**: `arrow.filename`・`arrow0.x/y`・`arrow1.x/y` の読み取り・装飾の系列の土台（`arrow*` の旧名の段）・矢印の表示（横書きと縦書き・普通のバルーンとシェル内バルーン）・矢印のクリックとホイールの手動スクロール（ホイールの受け口を新しく作る）・決定論のテスト・ぱすたの実機の確かめ（`ghost-inner-balloon` の着地の後）・網羅台帳 `assets.toml` の行（持ち主を本 spec へ）。
- **Out**: `onlinemarker.*`・`number.*` と `\![set,balloonnum]`・`sstpmarker`／`sstpmessage`（`balloon-markers` に残す）・`clickwaitmarker` の振る舞い（`talk-fast-forward`）・押し出しのフェード（`balloon-scroll-fade`）・`balloon-canon-residue` の項目 2〜6・自動スクロールの規則そのもの（正典どおり）。

## Boundary Candidates

- バルーンの読み手の鍵（areka-parsers・新しいファイル）
- 装飾の系列の解決（emo-present の `balloon.rs`）
- 印の配置と描画・見える範囲の手動の位置（emo-text）
- 矢印のクリックとホイールの受け口（`crates/areka/src/input_events/`）

## Out of Boundary

- バルーンの面の選び方・寿命（`balloon-canon-residue`・完了 `balloon-lifecycle-events`）

## Upstream / Downstream

- **Upstream**: 完了 `shell-balloon`（箱の形）。実機でぱすたの縦書きのバルーンを見るには: `ghost-inner-balloon` は完了（main #364）。ぱすたの縦書きのバルーンは、ぱすたの書庫の直し（ghost_dev の `c32229e`・未リリース）で入る。それまでは手順書 `doc/pasta-real-run.md` の入れ方で入れる（働きの前提ではない）。名前から絵を探す所を `.webp` も受ける形にそろえる `balloon-webp-names`（v0.1.0 から外した 5 本）は待たない（ぱすたの矢印は `.png`）。
- **Downstream**: `balloon-markers`（後半の状態の印が系列の土台と印の配置に乗る）・`balloon-scroll-fade`（v0.3.0・前半だけを待てばよいと決めていた＝本 spec の後）・`talk-fast-forward`（`clickwait*` の系列）。

## Existing Spec Touchpoints

- **Extends**: `balloon-markers`（前半を本 spec へ切り出した＝同 brief に 1 行）・`balloon-canon-residue`（網羅台帳の持ち主＝同 brief に 1 行）。
- **Adjacent**: ✅ `ghost-inner-balloon`（main #364 で完了・ゴーストの中の `balloon\<名>\`。ぱすたの縦書きのバルーンはぱすたの書庫の直しで入る）・`balloon-font-file`・`text-typesetting`・`talk-fast-forward`（emo-text の `actor*.rs`・`viewbox*.rs` を分け合う＝並べない）・`balloon-webp-names`（emo-present の `balloon.rs`）。

## Constraints

- 1 ファイル 1,000 行。`crates/areka-parsers/src/balloon/model.rs` は上限に近い＝鍵は新しいファイルへ。emo-text は `lib.rs` の一覧にファイルを足す。
- 決定論のテストは必達。縦書きの矢印の向きは SSP で測ってから決める（ukadoc に明文が無い）。

## 起票時の見立て（2026-10-23・ブランチ `claude/areka-p0-pasta-windows-d4ed7a`）

- 触るファイル（`balloon-markers` の棚卸㉓の前半の一覧）: `crates/areka-parsers/src/balloon/{model.rs, parse.rs, validation_tests.rs}` と鍵の新しいファイル・`crates/areka-emo-present/src/balloon.rs`・`crates/areka-emo-text/src/{layout.rs, viewbox.rs, viewbox_diff.rs, viewbox_draw_render.rs, actor.rs, actor_present.rs, actor_box.rs, lib.rs}`＋印の配置の新しいファイル・`crates/areka/src/input_events/{balloon.rs, balloon_pressed.rs, shell_box.rs, shell_box_handler.rs, user_break.rs}`＋ホイールの受け口・`doc/ukadoc-coverage/ledger/assets.toml`。行数は着手のときに測り直す。
- 規模: M（11〜13）。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール・ぱすた）。段: 優先（バルーン）。
- 要件定義のモデル: Opus。
- 議題（`balloon-markers` の棚卸㉑〜㉓の議題のうち前半の分）: ⑴ 箱の印の絵をどのフォルダから引くか。⑵ 手動で戻している最中に新しい文字が来たとき・時間切れのときの扱い。⑶ 印を文字の面の中で描くか emo-present で重ねるか。⑷ 縦書きの矢印の向きと出る条件（SSP の MCP で測る）。

## 2026-10-23 棚卸㉙の再測定（main aa8da8a7）
- **前提の変化**: 起票の後に着地した C10 の 8 本は、この spec の触るファイルをほとんど動かしていない（`crates/areka/src/input_events/user_break.rs` に `window-link-directives` が 6 行、`crates/areka-parsers/src/balloon/model.rs` と emo-text の数ファイルは説明の字の直しだけ）。`ghost-inner-balloon` は着地済み＝Scope の「`ghost-inner-balloon` の着地の後」は済んだ話。網羅台帳の `arrow0.x/y`・`arrow1.x/y` の 4 行は、持ち主がもう本 spec（`doc/ukadoc-coverage/ledger/assets.toml` 717〜778 行）。`arrow.filename` の行（同 701 行）だけが `balloon-canon-residue` のまま。ホイールの値は wintf の `PointerState.wheel`（`crates/wintf/src/ecs/pointer/systems.rs` が毎フレーム消す）にあり、areka はまだどこでも読まない。ぱすたの縦書きのバルーンは `arrow0.png`・`arrow1.png` と descript の 69〜72 行目を持つ（このワークツリーの `target\pasta-verify\` で確かめた）。
- **触るファイル**（行数）: バルーンの読み手 `crates/areka-parsers/src/balloon/{mod.rs 26, model.rs 774, parse.rs 252, validation_tests.rs 207}`＋鍵の新しいファイル・系列の表 `crates/areka-emo-present/src/balloon.rs`（701）・emo-text の `layout.rs`（502）・`viewbox.rs`（460）・`viewbox_diff.rs`（429）・`viewbox_draw_render.rs`（657）・`actor.rs`（768・バルーンごとの値 `ResolvedBalloonText` に矢印の位置を置く）・`actor_present.rs`（435）・`actor_box.rs`（470）・`lib.rs`（545）＋印の配置の新しいファイル・入力の `crates/areka/src/input_events/{balloon.rs 495, balloon_pressed.rs 198, shell_box.rs 203, shell_box_handler.rs 386, user_break.rs 395, mod.rs 576}`＋ホイールの受け口の新しいファイル・台帳 `assets.toml` の 5 行と `doc/ukadoc-coverage/roadmap-draft.md` の本 spec と `balloon-canon-residue` の数の行。読み返す間の時間切れを止めるなら `crates/areka/src/emo2_boot/balloon_visibility*.rs` も。上限に近いのは `model.rs`（774）と `actor.rs`（768）＝鍵は新しいファイルへ。
- **規模**: 13〜16 タスク（起票の 11〜13 に、普通のバルーンと箱の両方のホイールの受け口と、SSP での縦書きの矢印の測りを足した）。20 は超えない見込み。
- **先に要るもの**: 働きの上では無い。同じウェーブに置けない相手（v0.1.0）: `balloon-communicatebox`（読み手の `mod.rs`・`model.rs`・`parse.rs`）・`mouse-click-wheel-events` と `shell-mouse-cursor`（`input_events/mod.rs`）・`choice-ranges-one-function`（`actor_present.rs`）・`placement-measure-bake-once`（案 B なら emo-present の `balloon.rs`）・`vertical-choice-hover-band`（あちらの直しが `viewbox_draw_render.rs` へ届いたときだけ）。**1 本だけの席**: emo-text に新しいファイルを足す＝使う。依存の追加は無い見込み。静かな机は要らない。人の待ち: 縦書きの矢印の向きと出る条件を SSP の MCP で測る（SSP を動かし画面を操作する＝開発者の許しを取る）・ぱすたの実機の確かめは排他の机を持って。ぱすたの書庫の直し（未リリース）は待たない（手順書 `doc/pasta-real-run.md` の入れ方で入れる）。
- **優先度の区分**: A（開発者「0.1.0までに解決しない場合、独立して起票せよ」・v0.1.0 のゴール「ぱすたが SSP と同じに動く」）。
- **要件定義のモデル**: Opus。ホイールは wintf に今ある 1 フレームだけの値を読むだけ、時間切れは今のバルーンの時間の数え方の上に乗るだけ＝新しい並行や時刻の約束ではない。議題が多いこと・実機の確かめは Fable の理由にならない。
- **分割の案**: 切らない（`balloon-markers` から一度切り出した spec）。
- **見つけた穴・古くなった記述**:
  - Scope の「ぱすたの実機の確かめ（`ghost-inner-balloon` の着地の後）」は古い（着地済み）。
  - 台帳の `arrow.filename` の行（`assets.toml` 701〜704 行）は持ち主が `balloon-canon-residue` のまま＝本 spec のタスクで移す。移すときは `roadmap-draft.md` の数の行も一緒に直す（直さないと台帳の検査が赤）。
  - `balloon-markers` の brief は末尾に 1 行足しただけで、本文（Problem・Desired Outcome・Scope・Upstream）は矢印と手動スクロールをまだ持ち、上流に本 spec が無い。roadmap の台帳の `balloon-markers` の行と依存の一覧の行も「`arrow*` と手動スクロール」「L（18〜22）」「上流は `shell-balloon`」のまま。`balloon-scroll-fade` の台帳の行の上流も `balloon-markers` のまま。
  - `talk-fast-forward` の brief は印の系列（`clickwait*`）を分け合う相手を `balloon-markers` と書く＝本 spec へ読み替える（系列の土台は本 spec が作る）。
  - 手動で戻した位置は、選択肢の当たりへ今の「面に反映したスクロールの量」（`crates/areka-emo-text/src/choice.rs` の `to_window_physical` が読む値）を通して届けば `choice.rs` に触らずに済む＝そう作ると `vertical-choice-hover-band` と重ならない。
