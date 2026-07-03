# Brief: areka-P0-window-placement

> **種別**: 本坑（main）。⓪ ghost（ゴーストエンジン）トラックのユニット（M-boot）。
> **調査日**: 2026-07-03（mock-shell/wintf コード深掘り＋ukadoc 正典）。

## Problem

ゴーストのキャラ窓・バルーン窓は現状 `crates/areka/src/main.rs` に**ハードコード**（位置 (400,200) 固定・2窓決め打ち・ドラッグ offset 定数）であり、**ゴースト定義から窓を生成し既定位置に置く「窓配置」の機構が存在しない**。⓪ ghost が窓のライフサイクルを所有するというロードマップ構造（lifecycle／窓配置／位置永続化）の「窓配置」を実装で埋める。

## Current State

- **mock-shell 実績（lift 素材）**: `crates/areka/src/main.rs` — shell 窓（`WS_POPUP`・`CompositionMode::DComp`・`BoxPosition::Absolute`・320×420）＋ balloon 窓（200×350・縦書き Typewriter）生成、`DragConfig`＋`on_shell_drag`（`SetWindowPosCommand`・balloon 追従 offset (335,0)）、クリックスルー登録、double-click 終了。**2窓生成＋ドラッグ＋追従は動作実績あり**。
- **wintf 窓生成 API**: `EcsWindowFactory::create_window`（`Window`+`WindowStyle`+`WindowPos` entity → Win32 窓・ex_style 自動計算・clickthrough 統合済み）。多窓は WUC compositor 1 個共有＋窓ごと Visual ツリーで対応済み。
- **既定位置ロジックは無**: ukadoc の `seriko.alignmenttodesktop` 系を読む・work area を参照する・スコープ別配置を決めるコードは存在しない。
- **位置永続化は無**（`ghost.dat` 不存在確認済み）— M-life の `position-persist` 領分で正しい。
- **入力モデル**: `package::MountModel`（shell dir）✅・shell/ghost descript の KV（`kv::parse_kv`）✅ が supply 可能。

## Desired Outcome

ゴースト定義（descript）とスコープ数に基づき、**キャラ窓（scope0 主体＋scope1 相方）とバルーン窓を生成し、ukadoc 準拠の既定位置に配置し、全面ドラッグ（バルーン追従含む）できる**機構。窓数はハードコードでなく構成入力。

**✔ 観測（単一 pass/fail）**: emo2 fixture 起点でキャラ窓が work area 基準の既定位置（`alignmenttodesktop` 既定＝bottom）に出現し、ドラッグ移動＋バルーン追従が動く（ロードマップ表記「むらさき/エモ窓が出てドラッグ移動」）。

## Approach

1. **窓生成の機構化**: mock-shell の窓生成コードを「N 窓を構成から生やす」形へ持ち上げ（`EcsWindowFactory` はそのまま・entity 構成の組立を ghost 側が所有）。
2. **既定位置（ukadoc カスケード）**: `seriko.alignmenttodesktop`（既定 `bottom`＝work area 下端・タスクバー除外）を実装。優先順位カスケード **ghost 全体 < ghost スコープ別（`sakura.seriko.*`/`kero.seriko.*`）< shell 全体 < shell スコープ別（`char*.seriko.*`）** の解決器を最小実装（emo2 が使う値のみ実挙動・他はシーム）。scope0/scope1 の相対配置（並び）は SSP de-facto を design で確定。
3. **ドラッグ**: mock-shell の `on_shell_drag` を lift（全面ドラッグ・修飾キー規則なし＝ukadoc de-facto）。バルーン追従は暫定 offset を維持し、正式なバルーン位置規則は balloon 表示系の後続へ委ねる。
4. **z-order**: 既定は非 topmost（SSP de-facto）。`seriko.zorder`/`sticky-window` はシームのみ（emo2 未使用なら実装しない）。
5. **kero 窓の扱い**: 生成機構は scope 数ぶん一般化して持つ（構造は最初から）。**二人立ちの surface 連動・本格結線は M-dual（`dual-surface`＋`dual-window`）**＝本ユニットは「窓が生えて置ける・動かせる」まで。

## クロスユニット契約（後続を詰ませない事前考慮・2026-07-03 fixture 実測反映）

- **emo-present への窓引き渡し契約**: 生成した **Window entity（handle）を emo-present が受け取り surface を装着**する——この受け渡し口を機構の公開 API として切る（どちらが先に完了しても M-boot 統合で結線可能・emo-present brief と対の契約）。
- **emo2 fixture 実測（2026-07-03・shell descript）**: `seriko.alignmenttodesktop,bottom`（既定と一致）・**`sakura.defaultx,0`／`kero.defaultx,0` を使用**（`defaulty` は無し）→ **`defaultx`/`defaulty` キーの解決を design 対象に含める**（alignmenttodesktop カスケードに加えて。x=0 の意味論＝work area 基準の解釈は SSP de-facto を design で確認）。`sakura.balloon.alignment,left`／`kero.balloon.alignment,right` も存在（バルーン配置系の後続ユニット向け・本ユニットは記録のみ）。
- **ulw-removal との API 変動調整**: `CompositionMode` collapse は本ユニットの窓生成コードにも波及（emo-present brief と同旨・順序調整 or 追随）。

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

- **Upstream**: `areka-P0-package-mount` ✅（shell dir・descript 供給）／`areka-P0-parser-foundation` ✅（KV）／wintf 窓・ドラッグ・clickthrough 基盤 ✅。
- **Downstream**: `areka-P0-ghost-setup`（lifecycle 統括が本機構を呼ぶ）／`position-persist`（既定位置の上書き）／`dual-window`・`\![move]`（結合クラスタ）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-mock-shell`（窓生成・ドラッグの donor）。
- **Adjacent**: `areka-P0-emo-surface`（**同じ `crates/areka/src/main.rs` 起点＝並行着手時はファイル衝突注意・順次推奨**。境界: Window entity=本ユニット／Visual ツリー=emo-surface）／`areka-P0-ghost-setup`（⓪ 同エンジン・同時着手回避）。

## Constraints

- Rust 2024・`windows` 0.62.2・tokio 禁止。window/render は UI スレッド固定（既存アフィニティ不変）。
- 最小実装＋薄い拡張シーム（emo2 が使う配置値のみ実挙動・カスケードの**構造**は最初から）。
- 正典は ukadoc・de-facto 挙動（z-order・ドラッグ規則・scope 相対配置）は design で SSP 実挙動を確認して確定。
