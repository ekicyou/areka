# Brief: areka-P0-shell-balloon-scale-setting

> 出どころ（2026-10-17 開発者・`/kiro-discovery`）: 「wintf や dola の改造が終わった後のフェーズのネタ。現在、シェルやバルーンの倍率は、DPI 追従だけで倍率が決まってるが、更に拡大率を設定できるようにしてほしい。例えば、DPI 200%、拡大率 200% だったら、計 400% で描画されることになる。」
> 同日の答え: 設定する場所＝**設定画面を待つ**（それまでは記憶ファイル `sylphya.toml` を手で書く形だけ）・覚える単位＝**ゴーストごと**・バルーン＝**連携の切り替え付き**・版＝**v0.4.0**。

## Problem

利用者は、シェルとバルーンの大きさを自分の好みで変えられない。大きさは画面の DPI（Windows の表示スケール）だけで決まるので、高 DPI の画面で小さく置きたい人、低 DPI の画面で大きく見たい人、小さく作られた古いシェルを大きくしたい人の手が無い。SSP には「ゴーストごとの設定」の「シェル倍率」があり、ゴーストの作者も利用者がそれを変える前提で台本を書いている。

## Current State

- **掛け算の口はもう在る**: `crates/areka-emo-present/src/scale.rs:45-50` の `ScalePolicy { author_dpi, app_scale }` が `k = app_scale × (dpi ÷ author_dpi)` を計算する（`scale.rs:146-150`）。`app_scale` は「アプリ管理拡大率」の口として、将来の spec まで 1 に固定されている（`scale.rs:12-14`）。本番で値を入れるのは `presenter/hub.rs:81` の `attach_target` だけ（`ScaleRatio::ONE`）。
- **k を読み戻して従うもの**（`app_scale` を入れれば自動で追従する）: wintf の `render_surface` の `SetTransform`（`crates/wintf/src/ecs/graphics/systems/render.rs:96-111`）・当たり判定（`presenter/hit.rs:46-60`）・バルーンの文字の層（`presenter/read.rs:100-113` → `ScaleContract` → `viewbox_draw.rs` の `SetTransform`）・`\![move]` の dx/dy（`emo2_boot/move_cue.rs:585`）。文字の配置と折り返しは作者の画像の px で行い、k は物理の面の大きさにだけ効く。
- **DPI だけから k を自前で計算し直す場所**（同じ倍率を掛けないと食い違う）: `placement/mod.rs:330-362` の `build_measure_scaling`（起動時の窓の大きさの k₀・シェルとバルーン別）・`placement/reseed.rs:101-111`（`balloon.offsetx/y` と `windowposition`）・`placement/follow/offset_space.rs:224-252` の `rescale_follow_offset`（DPI の比だけを使う＝倍率の変化は追わない）。
- **倍率が途中で変わったときに走り直す道が無い**: 計算し直しは `Changed<DPI>` だけが起こす（`emo2_boot/frame/dpi.rs:216-245` → `refresh_scale` → 窓の大きさ直し・バルーンのずれの直し・`chain_realign.rs`）。`ScalePolicy` は attach のときに決まり以後変わらない（`scale.rs:32`）。
- **覚える場所**: sylphya の `PersistKey`（`crates/areka-sylphya/src/persist/mod.rs:123-157`）に `Ghost` の scope が在る（`<root>/sylphya.toml`）。倍率の鍵はまだ無い。
- **網羅台帳**: `currentghost.scope(ID).scaling`・`currentghost.balloon.scope(ID).scaling` は `vocabulary-only`（持ち主 `currentghost-property-tree`）。台帳 `property.toml:427` の注は、SSP の実効の倍率を「利用者がベースウェアで決める倍率 × surfaces.txt の `scaling` × タグの倍率」と書く＝本 spec の値はその最初の項。

## Desired Outcome

- ゴーストごとに「表示倍率」（％）を覚え、シェルの描く倍率が **表示倍率 × DPI の倍率** になる（DPI 200%・表示倍率 200% なら 400%）。
- バルーンは既定でシェルと同じ表示倍率で描く。「バルーンも連携する」を切ると、バルーンは DPI の倍率だけで描く（SSP の「バルーンも連携する」と同じ）。
- 当たり判定・バルーンの文字・バルーンの位置のずれ（`balloon.offsetx/y`・`windowposition`）・キャラクターどうしの詰め・`\![move]` の量が、描いた大きさと食い違わない。
- 設定の入口は**設定画面（windows-reactor 待ち）**。それまでは `sylphya.toml` の値を手で書いて起動すれば効く。ゴーストを切り替えると、切り替えた先のゴーストの表示倍率になる。

## Approach

既存の口 `ScalePolicy.app_scale` に、ゴーストごとの表示倍率を入れる。`placement` の 3 か所の自前の k の計算も同じ倍率を掛ける形へそろえ、k を 1 か所から引く。値は sylphya の `Ghost` scope の新しい鍵として覚え、起動とゴーストの切り替えのときに読む。

- **採らなかった案**: wintf の窓や D2D の変換に別の掛け算を足す（k を読み戻す当たり判定・文字の層・`\![move]` に届かず、二重の真実になる）。DPI の値そのものを偽って大きくする（Windows の DPI の道と `WM_DPICHANGED` の扱いを壊す）。
- **途中で変わる道**: 設定画面ができるまでは値が変わるのは起動とゴーストの切り替えだけ。設定画面から値が変わったときに走り直す道（`Changed<DPI>` と同じ鎖を倍率の変化でも起こす）をこの spec で作るか、設定画面の spec に任せるかは要件の段で決める（入口の無い道を先に作らない決まり＝「語彙完備・配線ゼロの追跡」との兼ね合い）。
- **呼び名**: このプロジェクトの文書で「拡大率」は Windows の表示スケール（DPI）を指してきた（例: `dpi-transition-two-tick-bounce`）。混ざらないよう、利用者が決めるほうは「**表示倍率**」と呼ぶ（SSP の画面の語は「シェル倍率」）。

## Scope

- **In**: ゴーストごとの表示倍率（シェル）と「バルーンも連携する」の 2 つの値の覚え方（sylphya の `Ghost` scope）・起動とゴーストの切り替えで読む・`app_scale` への流し込み・`placement` の自前の k の 3 か所をそろえる・覚えている窓の位置と倍率の関係（倍率を変えた次の起動で、下端中央の原点のまま大きさだけ変わる）・決定論のテストと実機（実 emo2・DPI ≠ 96）での確かめ。
- **Out**: 設定画面（windows-reactor 待ち・予約「設定画面と設定ファイル」が入口を持つ）・右クリックメニューの倍率の段（開発者が「設定画面を待つ」を選んだ）・`\![set,scaling]` のタグ（台帳の持ち主は覚え書き「`sakura-time-directives` から戻した C・D 群」の D 群）・surfaces.txt の描画メソッド `scaling` と element定義の `--scaling`（`wintf-d2d-composite-rules` の下流）・`OnShellScaling`／`OnBalloonScaling`（タグや画面で倍率が変わる道とまとめて起票する）・`currentghost.scope(ID).scaling` のプロパティの値（`currentghost-property-tree`）・拡大や縮小のときの補間の質の切り替え（SSP の「補完処理」）。

## Boundary Candidates

- 倍率の値を持つ層（sylphya の鍵・ゴーストごと・既定 100%）
- k を決める層（`ScalePolicy` と `placement` の自前の計算を 1 つの出どころへ）
- 倍率が変わったときに走り直す鎖（DPI の変化と同じ鎖を共有するか）

## Out of Boundary

- 設定画面の UI と、そこから値を書く手順
- 台本や SERIKO から倍率を変える指令とイベント
- 合成と描画の作り（D2D への移しは v0.2.0）

## Upstream / Downstream

- **Upstream**: v0.2.0 の wintf・dola の基幹の整備（特に `wintf-d2d-composite-rules`＝k を当てる `render_surface` を作り直す・`draw-methods-canon`＝合成を D2D へ移す・`compose-f16-precision`＝k ≠ 1 の丸め）。この 3 本の着地の後の形に乗せる。`dpi-realign-remembered-chain`（拡大率が変わった後の詰め直しの穴・同じ `chain_realign.rs`・`persist.rs`）が先に着地していれば、その形に乗る。
- **Downstream**: 予約「設定画面と設定ファイル」（表示倍率と「バルーンも連携する」を項目として持つ）・`\![set,scaling]` を受ける spec（表示倍率 × タグの倍率の掛け算に乗る）・`currentghost-property-tree`（`scope.scaling` に実効の倍率を出す）・`hit-id-pass`（逆変換に表示倍率も入る）。

## Existing Spec Touchpoints

- **Extends**: なし（`app_scale` の口は `emo-dpi-scaling` 系の完了 spec が残した）
- **Adjacent**: `dpi-realign-remembered-chain`・`dpi-transition-two-tick-bounce`（同じ `frame/dpi.rs`・`chain_realign.rs`・`refresh_scale` の道）・`move-canon`（`\![move]` の量に k を掛ける所）・`wintf-d2d-composite-rules`・`hit-id-pass`・`currentghost-property-tree`

## Constraints

- v0.4.0（v0.2.0 の wintf・dola の改造の後・v0.3.0 を重くしない）。
- 時刻と同じく倍率も正確に扱う: `ScaleRatio` の有理数のまま掛け、丸めは `areka-emo-compose` の 1 か所だけ（`scaled_extent`・`scale_len`）。
- DPI 追従が基本設計（k は D2D の変換行列で当てる・合成は元の大きさのまま）を崩さない。
- 1 ファイル 1,000 行の番人（`placement/mod.rs` 902 行・`emo2_boot` の大きいファイル）。
- 実機の確かめは実 emo2・DPI ≠ 96 で、根と検体はワークツリーの `target\` の下。
