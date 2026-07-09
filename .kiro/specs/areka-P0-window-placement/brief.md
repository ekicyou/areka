# Brief: areka-P0-window-placement

> **種別**: 本坑（main）。⓪ ghost（ゴーストエンジン）トラックのユニット（M-boot）。
> **調査日**: 2026-07-03（mock-shell/wintf コード深掘り＋ukadoc 正典）／**2026-07-05 改稿**（リジェクト教訓の織り込み）。
> **⚠️ 2026-07-05 開発順序リジェクト（教訓の正本）**: 本ユニットは旧 brief の「mock-shell lift＋fixture 観測」前提で一度着手され、**実 DPI で座標が破綻**——`Monitor.work_area`/`WindowPos`＝物理 px と `BoxStyle` Px＝論理 px の**混在**により resolve が窓を沈め・drag が DPI 再スケールで二重変換→画面外消失。**dpi=96 では自己整合するためテスト緑が欠陥を隠した**。ワークツリーごとリジェクト（記憶 areka-placement-real-ghost-first／areka-window-placement-dpi-coordinate-defect）。
> **前提依存（順序ゲート・2026-07-05 確定・2026-07-09 解消）**:
> ```
> _Depends(confirmed): completed/areka-P0-emo-present（本番ゴースト＝emo2 の実 surface 表示。これに対して実装・検証する）
> ```
> **2026-07-09 ゲート解消**: `areka-P0-emo-present` が完了（`completed/areka-P0-emo-present`）。本番ゴースト（emo2 実 surface）を実 DPI（125%）で表示する `crates/areka/examples/emo-present.rs` が観測土台として利用可能——本ユニットはこの example を「本番ゴースト先行」の実装・検証対象として着手してよい。
> **本番ゴースト先行の原則**: 窓配置/UI 位置決めは**本番ゴーストを表示した上でそれに対して**実装・検証する。単発デモ（ハードコード窓・架空 work_area）への合わせ込みは無意味＝実 DPI で即破綻することが実証済み。

## Problem

ゴーストのキャラ窓・バルーン窓のハードコード実装（位置 (400,200) 固定・2窓決め打ち・ドラッグ offset 定数）は **app-shell ✅（2026-07-05）で `crates/areka/examples/mock-shell.rs` へ example 保全**され、main.rs は骨格（構成解決＋検証用ダミー窓＋replace-me シーム `open_startup_window`）へ純化済み。しかし**ゴースト定義から本物の窓を生成し既定位置に置く「窓配置」の機構は依然存在しない**（シームの中身が空）。⓪ ghost が窓のライフサイクルを所有するというロードマップ構造（lifecycle／窓配置／位置永続化）の「窓配置」を実装で埋める。

## Current State

- **mock-shell の位置づけ（2026-07-05 格下げ・app-shell ✅ で所在確定）**: `crates/areka/examples/mock-shell.rs`（app-shell が挙動不変保全）は shell 窓＋balloon 窓＋ドラッグの動作実績を持つが、**lift は「窓生成 API の呼び方」の donor に限る**。初期位置 (400,200)・追従 offset (335,0) 等の**座標値・配置ロジックは持ち込み禁止**（デモ合わせ込み＝実 DPI 破綻の実証済み経路）。demo は DPI 処理ゼロ・work_area 非参照。**差し込み先は main.rs の `open_startup_window(&WinApp)` シーム**（ダミー窓を本物ゴースト窓生成へ差し替える・app-shell が用意した唯一の置換点——ghost-setup と分担: 結線=あちら・窓=こちら）。
- **wintf 座標系の論理/物理混在（実装前の必須確定事項）**: `Monitor.work_area`＝物理 px・`WindowPos`＝物理 px・`BoxStyle` Px＝論理 px・`GlobalArrangement` scale＝DPI 係数——**この契約を design 冒頭で型レベルに確定してから実装**（07-05 リジェクトの直接原因。混在演算を型で排除する newtype 等は design 判断）。
- **wintf 窓生成 API**: `EcsWindowFactory::create_window`（`Window`+`WindowStyle`+`WindowPos` entity → Win32 窓・ex_style 自動計算・clickthrough 統合済み）。多窓は WUC compositor 1 個共有＋窓ごと Visual ツリーで対応済み。
- **既定位置ロジックは無**: ukadoc の `seriko.alignmenttodesktop` 系を読む・work area を参照する・スコープ別配置を決めるコードは存在しない。
- **位置永続化は無**（`ghost.dat` 不存在確認済み）— M-life の `position-persist` 領分で正しい。
- **入力モデル**: `package::MountModel`（shell dir）✅・shell/ghost descript の KV（`kv::parse_kv`）✅ が supply 可能。

## Desired Outcome

ゴースト定義（descript）とスコープ数に基づき、**キャラ窓（scope0 主体＋scope1 相方）とバルーン窓を生成し、ukadoc 準拠の既定位置に配置し、全面ドラッグ（バルーン追従含む）できる**機構。窓数はハードコードでなく構成入力。

**✔ 観測（単一 pass/fail・2026-07-05 是正）**: **本番ゴースト（emo2 の実 surface 表示＝emo-present 経由）に対し、実 DPI（per-monitor v2・dpi≠96 の実行が必須）**で、キャラ窓が work area 基準の既定位置（`alignmenttodesktop` 既定＝bottom）に出現し、ドラッグ移動＋バルーン追従が画面内で正しく動く。**dpi=96 のみのテスト緑は不合格**（自己整合で欠陥を隠すことが実証済み——07-05 リジェクトの教訓）。純粋 resolver の単体テストは **DPI をパラメタ化**（96/120/144/192）して物理/論理変換を固定する。

## Approach

0. **（必須先行）wintf 座標契約の確定**: 論理/物理の単位契約（物理 px＝`WindowPos`/`Monitor.work_area`・論理 px＝`BoxStyle`・scale＝`GlobalArrangement`）を design 文書で確定し、resolver・drag の入出力単位を固定（07-05 リジェクトの再発防止・実装より先）。
1. **窓生成の機構化**: mock-shell の窓生成 API の呼び方のみ donor に、「N 窓を構成から生やす」形を新造（`EcsWindowFactory` はそのまま・entity 構成の組立を ghost 側が所有。**demo の座標値・offset 定数は持ち込まない**）。
2. **既定位置（ukadoc カスケード）**: `seriko.alignmenttodesktop`（既定 `bottom`＝work area 下端・タスクバー除外）を実装。優先順位カスケード **ghost 全体 < ghost スコープ別（`sakura.seriko.*`/`kero.seriko.*`）< shell 全体 < shell スコープ別（`char*.seriko.*`）** の解決器を最小実装（emo2 が使う値のみ実挙動・他はシーム）。scope0/scope1 の相対配置（並び）は SSP de-facto を design で確定。
3. **ドラッグ**: mock-shell の `on_shell_drag` を lift（全面ドラッグ・修飾キー規則なし＝ukadoc de-facto）。バルーン追従は暫定 offset を維持し、正式なバルーン位置規則は balloon 表示系の後続へ委ねる。
4. **z-order**: 既定は非 topmost（SSP de-facto）。`seriko.zorder`/`sticky-window` はシームのみ（emo2 未使用なら実装しない）。
5. **kero 窓の扱い**: 生成機構は scope 数ぶん一般化して持つ（構造は最初から）。**二人立ちの surface 連動・本格結線は M-dual（`dual-surface`＋`dual-window`）**＝本ユニットは「窓が生えて置ける・動かせる」まで。

## クロスユニット契約（後続を詰ませない事前考慮・2026-07-03 fixture 実測反映）

- **emo-present への窓引き渡し契約**: 生成した **Window entity（handle）を emo-present が受け取り surface を装着**する——この受け渡し口を機構の公開 API として切る（どちらが先に完了しても M-boot 統合で結線可能・emo-present brief と対の契約）。
- **emo2 fixture 実測（2026-07-03・shell descript）**: `seriko.alignmenttodesktop,bottom`（既定と一致）・**`sakura.defaultx,0`／`kero.defaultx,0` を使用**（`defaulty` は無し）→ **`defaultx`/`defaulty` キーの解決を design 対象に含める**（alignmenttodesktop カスケードに加えて。x=0 の意味論＝work area 基準の解釈は SSP de-facto を design で確認）。`sakura.balloon.alignment,left`／`kero.balloon.alignment,right` も存在（バルーン配置系の後続ユニット向け・本ユニットは記録のみ）。
- **ulw-removal ✅ 完了（2026-07-05）＝新 API 前提で書く**: `CompositionMode` は撤去済み（factory は常時 `WS_EX_NOREDIRECTIONBITMAP`）。窓生成は**現行 API（collapse 後）を最初から前提**にする。
- **通信モデル**: 窓操作（生成・移動・z-order）は **UI スレッド専有**。本ユニットは `areka-actor` 非依存（UI スレッド内で完結）だが、**他アクターからの窓移動指令**（将来の `\![move]`＝sakura 発・二人立ち連動等）は実装済みの **UI 配送ブリッジ（`spawn_ui`/`UiSender`）**経由で届く前提——窓移動の公開 API を「UI スレッド上で呼ばれる関数」として切っておく（ブリッジが後からその関数を呼ぶだけで済む形）。

## ukadoc 正典要点（design の前提事実）

- **`seriko.alignmenttodesktop`**: 既定位置指定。既定値 `bottom`（work area 右下基準・デスクトップ下端整列）。ghost descript／shell descript の両方に書け、**shell 側が ghost 側に優先・スコープ別が全体に優先**。
- **座標系**: Windows work area 基準（タスクバー除外）。
- **ドラッグ**: キャラ窓は surface 全面からドラッグ可（修飾キー規則は正典に無し＝de-facto）。
- **z-order／sticky**: `seriko.zorder`・`seriko.sticky-window`（SSP 2.4+）。既定 topmost ではない（de-facto・design で SSP 実挙動確認フラグ）。
- **保存位置の復元**は起動時挙動として存在するが、**M1 では position-persist（M-life）へ分離済み**——本ユニットは「保存が無いときの既定位置」のみ。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-03 総ざらい）

- **必読**: `descript_ghost` と `descript_shell` の **`seriko.alignmenttodesktop`**（全体）・**`sakura.seriko.alignmenttodesktop`/`kero.seriko.*`/`char*.seriko.*`**（スコープ別・ghost/shell 両所在＝4層カスケードの各定義）＋ **`(sakura|kero|char*).defaulttop`**（**free 時のみ有効**と明記あり——重要）。
- **⚠️ 重要修正（総ざらいで発見）**: ① **`defaulttop` は alignmenttodesktop=free のときだけ効く**——bottom 整列（emo2）では Y は下端固定＝defaulttop 無視の分岐を design に反映 ② **`defaultx` は ukadoc で確認できず**（`defaulttop`/`defaultleft` 系は確認）——emo2 fixture は `sakura.defaultx,0` を実使用＝**SSP de-facto キー**。design で `defaultx`⇔`defaultleft` の関係（同義/別義・X 座標系の原点）を SSP 実挙動で確定し、**両表記を受ける寛容実装**にすること。
- **brief 未網羅→design で埋める項目**: ① `alignmenttodesktop` の**値域全量**（bottom 既定は確認済み・top/free の正確な挙動）② 座標系の原点（work area か モニタか・複数モニタ時の既定）③ 起動時可視性・スケール系キーの有無（あればシームのみ）④ ghost↔shell の優先度表を design.md に明文化（ghost 全体＜ghost スコープ＜shell 全体＜shell スコープ）。
- **具体指示**: design 冒頭で `descript_ghost`/`descript_shell` の placement 系キーを `get_doc` し、**「キー×所在×優先度×有効条件（free 限定か）」の1枚表**を design.md に載せること。emo2 実測値（bottom・defaultx,0）をその表の検証行に使う。

## Scope

- **In**: 窓生成の機構化（scope 数対応・balloon 窓含む）／`alignmenttodesktop` 既定位置（カスケード解決・emo2 使用値）／work area 計算／全面ドラッグ＋バルーン追従（暫定 offset）／既定 z-order（非 topmost）。
- **Out**: 位置永続化 `ghost.dat`（**position-persist**・M-life）／二人立ち surface 連動（**M-dual**）／`\![move]` キャラ移動（**sakura-dialogue-tags** 結合クラスタ）／バルーンの正式配置規則（balloon 表示系）／surface 描画内容（**emo-surface**）／メニュー・chrome（M2）。

## Boundary Candidates

- 既定位置解決器（descript KV→配置座標・純粋関数＝単体テスト可）と窓生成結線の分離
- 窓構成の組立（ghost 所有）と `EcsWindowFactory`（wintf 所有・不改変）の境界
- ドラッグ追従＝将来の「2窓/surface 契約」（M-dual 結合クラスタ）の片側

## Out of Boundary

- Visual ツリー・surface 合成の中身（**emo-surface**＝Visual 層。本ユニットは Window entity 層のみ所有）。

## Upstream / Downstream

- **Upstream**: **`completed/areka-P0-emo-present` ✅（順序ゲート解消・本番ゴースト実表示＝検証対象そのもの）**／`areka-P0-package-mount` ✅（shell dir・descript 供給）／`areka-P0-parser-foundation` ✅（KV）／wintf 窓・ドラッグ・clickthrough 基盤 ✅（ただし座標契約の確定＝Approach 0 が先）。
- **Downstream**: `areka-P0-ghost-setup`（lifecycle 統括が本機構を呼ぶ）／`position-persist`（既定位置の上書き）／`dual-window`・`\![move]`（結合クラスタ）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-mock-shell`（窓生成・ドラッグの donor）。
- **Adjacent**（2026-07-09 更新③）: `completed/areka-P0-app-shell` **✅**（main.rs 骨格化済み＝本ユニットは**骨格の上で**窓機構を実装・main.rs 衝突は構造ごと解消済み）／`completed/areka-P0-emo-present` **✅**（順序ゲート解消済み。境界: Window entity=本ユニット／表示供給＋emo ランタイム=emo-present）／`areka-P0-ghost-setup`（⓪ 同エンジン・**並走中**——境界: エンジン結線・終了統括=あちら／窓生成・配置=こちら。`open_startup_window` シームの中身を両者が触る点は結線時に調停）。

## Constraints

- Rust 2024・`windows` 0.62.2・tokio 禁止。window/render は UI スレッド固定（既存アフィニティ不変）。
- **本番ゴースト先行・実 DPI 検証必須**: デモ（ハードコード窓・架空 work_area）への合わせ込み禁止。受け入れは実 DPI（≠96）実行を経ること。
- 最小実装＋薄い拡張シーム（emo2 が使う配置値のみ実挙動・カスケードの**構造**は最初から）。
- 正典は ukadoc・de-facto 挙動（z-order・ドラッグ規則・scope 相対配置）は design で SSP 実挙動を確認して確定。
