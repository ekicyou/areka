# Brief: areka-P0-currentghost-property-tree

> 起票: 2026-08-27（bvc 要件ディスカッション議題 4 の開発者指示による `/kiro-discovery` 再入・プロパティ系 3 spec 分割の 2 本目）
> **本 spec は `currentghost.*` 枝（≈65 項目）の最初の実導出であり、bvc（`areka-P0-balloon-vertical-canon`）が縮退登記した `currentghost.balloon.scope(ID).*` 族——`.vertical` を含む——の指名追跡先である。**

## Problem

ゴーストスクリプトは自分自身の状態（サーフェス番号・窓座標・バルーンの寸法や書字方向・シェル一覧…）をプロパティ `currentghost.*` で照会するが、areka の sylphya には `currentghost.*` の行が **0 件**しか無い（ルート枝名の予約のみ）。sylphya の M1 設計は「`baseware.*` のみ実導出・他ルート枝は NOT_FOUND 縮退（差替シーム付き）」と明文宣言しており（`vocab/dotted.rs:3-9`）、本 spec がその差替シームを `currentghost` 枝で初めて使う。

## Current State

- 掲示機構は完備: 解決は正準文字列の map 参照 1 本（`reader.rs:80-83`・`:127-146`）・`scope(N)` は `Selector::ByIndex`、`ghostlist(名前)` 型は `Selector::ByName`（`key.rs:129-141`）も既存・値なしは `NotFound`→SHIORI 側 `SHIORI_E_PROPERTY_NOT_FOUND`（捏造しない縮退が既定で成立）・`currentghost` はルート枝ゆえ SET は自動で `NotSettable`。
- 本番の publish 縫い目は 1 箇所: `emo2_boot/mod.rs:430-465`（`BootAssets.balloons` が生きていて `sylphya_publisher()` も取れる区間）。sylphya は最下層 crate で `areka-parsers`/`areka-emo-text` へ依存できない＝値は `areka` bin 側で解決して文字列で渡す形（bvc research §3.6-2）。
- `scope(ID)` は「ID ごとに 1 行 publish」を意味する（セレクタ分岐解決は存在しない）＝scope 集合の列挙規則が本 spec の設計事項（`areka.balloon.offset.scope(N)` と同形の先例あり・`persist/mod.rs:150-161`）。

## Desired Outcome

`currentghost.*` の各項目が、実際のゴースト状態から導出されて掲示され、経路 spec（`areka-P0-property-query-channels`）の照会で読める。未解決スコープ・未導出項目は値なしのまま（捏造しない）。

## Scope

- **In**（snapshot 2.8.80 実測の枝別内訳・≈65 項目）:
  - **`currentghost.balloon.scope(ID).*` ×17＋`balloon.汎用`＋`balloon.count`＝19 項目（bvc 縮退登記の指名受け皿・全列挙）**: `background.color`／`basepos.x`／`basepos.y`／`char_width`／`count`／`lines`／`lines.initial`／`num`／`rect`／`scaling`／`validheight`／`validheight.initial`／`validwidth`／`validwidth.initial`／**`vertical`**／`x`／`y`（＋`mousecursor` 系 4 は SET 有効側）。**⚠2.8.83 改訂の適用必須**——`validwidth`＝列が並ぶ方向の幅／`validheight`＝1 列の長さ／`lines`＝収まる列数＝いずれも**画面上の向き**基準（2.8.80 と役割が逆・bvc requirements SC3/SC4/SC13 が正本・ukadoc-mcp snapshot のプロパティ節は旧意味論なので裏取りに使わない）。
  - **`.vertical` の導出規則は bvc が確定済み**——スコープに実際に適用されている書字方向（bvc Requirement 2 の共存規則の確定結果）から導く・`vertical_lr`（areka 拡張）も `1`・未解決スコープは値なし（bvc Requirement 7 の語彙登記が正本・書字方向の確定は起動時 1 回＝bvc Requirement 9）。
  - `currentghost.scope(ID).*`＋`.scope.count` ×17: `animation.num`・`currentmonitor` ×5・`name`・`rect`・`scaling`・`seriko.defaultsurface`・`surface(ID).rect`・`surface.num`/`x`/`y` 等（SET 有効 3 件を含む）。
  - `currentghost.mousecursor.*` ×6（全 SET 有効）・`currentghost.seriko.*` ×14（cursor/tooltip の当たり判定名セレクタ・**`zorder`**・`sticky-window`・surfacelist）・`currentghost.shelllist.*` ×4・`.status`・`.汎用`。
  - scope ID 集合の列挙規則と未解決スコープの表現（publish の不在 vs 明示——bvc research §6 項目 8 を引受け）。
  - `vertical` 等を `GENERIC_PROP_NAMES` に登録するかの裁定（bvc research §6 項目 9 を引受け——登録すれば件数檻 4 箇所更新・GET 経路は台帳を読まないため目的は語彙の第一級保持の側）。
- **Out**:
  - 照会経路（`areka-P0-property-query-channels` 所有）。
  - `currentghost.sound.*` ×3＋サウンド語彙族 ≈18 葉（音再生基盤に依存＝`areka-P0-property-catalog-lists` へ・SET 経路の台帳追随は channels spec）。
  - `system.*`・カタログ群・`.ext.*`（同上）。
  - SET 有効項目の**書込効果の実装**（mousecursor 差替・tooltip 文言変更等は各機能基盤が要る——値の保持と GET までを本 spec・効果は機能側 spec が解禁時に接続）。

## Boundary Candidates

- publish シーム（`emo2_boot` の 1 箇所・値の解決は bin 側）と枝別の導出タスク（balloon／scope 幾何／seriko／shelllist）が自然な分割。
- balloon.scope 19 項目は bvc の座標意味論テスト資産（R3）と同じ数値源＝先行スライス候補。

## Out of Boundary

- 書字方向の解決規則そのもの（bvc 所有・確定済み）。
- バルーン定義の解析（bvc）・窓配置（placement 系 spec）。

## Upstream / Downstream

- **Upstream**: `areka-P0-property-query-channels`（照会の end-to-end 証明はこれ待ち・**publish と決定論テストは単独で可能**）・bvc（`.vertical` 導出規則＋2.8.83 意味論の登記・balloon.scope 族の縮退登記元）・`ghost-window-zorder`／`scope-zorder-pinning`（seriko.zorder の値源）。
- **Downstream**: ゴーストスクリプトの状態照会全般・bvc の「ゴーストが縦書きを判定して字数計算を変える」ユースケースの最終成立。

## Existing Spec Touchpoints

- **Extends**: sylphya の M1 縮退宣言（`vocab/dotted.rs:3-9`）の差替シームを初行使＝宣言文の改訂を伴う（完了 spec 正典の追随規律）。
- **Adjacent**: **⚠合流裁定必須＝`areka-P0-zorder-property`**（2026-08-27 に並走 zsp ブランチで起票・`currentghost.seriko.zorder` 単独 spec・本ブランチには不在）は本 spec の `seriko.*` 範囲の真部分集合——**マージ時に ⑴ 本 spec へ吸収 or ⑵ 本 spec から `seriko.zorder` を切り出しの二択を、クロス spec 合流セッションで裁定すること**（二重所有のまま放置しない・記憶 portfolio-convergence-decided-in-separate-session）。`areka-P0-balloon-canon-residue`（M2 ゲート・系列解決と表示寿命＝プロパティ族は収載外で非交差）。

## Constraints

- ウェーブ配置: **M2 解禁ゲート**（channels spec の後段・bvc 完了後が自然）。
- 正典参照はライブ ukadoc（2.8.83 現行）——**ukadoc-mcp snapshot のプロパティ節は 2.8.80 意味論で逆**（bvc Requirement 11.7 登記済みの罠）。
- 値の捏造禁止（未導出は NotFound のまま）・決定論テスト必達（scope 2 体・縦書き/横書き双方の `.vertical` 一致を含む）。
