# Brief: areka-P0-collision-geometry

> **種別**: 本坑（main）増分。⑥ emo 帰属（M-life「撫でクラスタ」の片側）。roadmap 増分「⑥ emo: `collision-geometry`（collision→region/actor 写像）」の brief 化。
> **調査日**: 2026-07-16（再入精査⑦・実装シーム偵察＋ukadoc 裏取り）。
> **クロスエンジン結合**: 撫で＝`collision-geometry`（⑥入力解決）⟷ `input-events`（③SHIORI 配信）——roadmap「結合クラスタは I/O 契約を先決してから並列実装」の適用。**region/actor I/O 契約の正本は本 brief**（下記・input-events brief と同時制定 2026-07-16）。
> **並走性**: cue モデル非接触＝実装中の `areka-P0-cue-playback-duration` と**並走可**。

## Problem

emo2 の撫で反応（`dic/touch.pasta:7` が OnMouseMove の actor×region でルーティング）とメニュー（OnMouseDoubleClick も Ref4=当たり判定名を運ぶ）には、**「マウス座標がどの当たり判定（Head/Bust）に居るか」を解決する層**が必要。ところが現状:

- parsers→emo-compose の正規化形 `SurfaceMaster` は collision を**保持している**（`crates/areka-emo-compose/src/normalized.rs:77`・parsers `shell/model.rs:71`）が、**どこにも露出・消費されていない**（emo-present の collision 参照はテスト用 `Vec::new()` のみ＝`presenter.rs:509,1165,1172`）。
- wintf の hit-test は合成ビットマップ由来 `AlphaMask`（`crates/areka-emo-present/src/cache.rs:82-99`）＝「キャラの不透明画素か」しか知らず、**部位名を知らない**。
- 「見える・触れるは emo が窓口＝kanade へは解決済みイベントだけ渡す」（roadmap「emo の責務範囲」節）——その**解決役が不在**。

## Current State（2026-07-16 実装偵察）

- **データは揃っている**: `Surface.collisions`（parsers `shell/model.rs:71`・矩形＋領域名）→ `SurfaceMaster.collisions`（`normalized.rs:71-79`・コメント `:63-66`「seriko が再利用」）。emo2 fixture 実物: side0 surface0 に Head/Bust（`fixtures/emo2/shell/master/surfaces.txt:23-24`）・kero 側 `:417-418`。**矩形のみ・2種のみ**（collisionex 不使用＝scope doc §2）。
- **「現在表示中の surface id」の追跡者**: emo-present の presenter が target 別に ShowSurface を受けて表示（`PresentCommand::ShowSurface`）。scope→target 写像は `crates/areka/src/emo2_boot/target_map.rs`（shell=2*scope 偶数）。**hit 時に「その窓がいま何の surface か」を引く読み口が未整備**（design で presenter の状態読み or 装着側での記録を確定）。
- **座標系**: 窓 client 物理 px ⟷ surface px は**等倍**（emo-present の座標契約＝合成は物理 px 等倍・window-placement の物理 px 単一通貨と同系）。hit 座標→collision 矩形照合は素直な同一空間比較で成立する見込み（design で DPI 契約を明文確認）。

## Desired Outcome

マウス座標（窓 client px）から**当たり判定名（不透明 String・例 "Head"/"Bust"）を決定論で解決する純粋層**が emo に確立し、input-events（③kanade）が消費できる契約が立つ。

**✔ 観測（単一 pass/fail）**: 純関数の全網羅 unit（GPU/表示不要）＝emo2 fixture 実 collision 値で (a) 矩形内→領域名 (b) 矩形外→None (c) 境界 on/off（含端規則） (d) 複数矩形重なり時の優先順位（正典規則） (e) collision 未定義 surface→None。＋統合は M-life 統合（input-events 側の実機サインオフ）へ委譲＝**観測の独立化**（純粋層）。

## region/actor I/O 契約（正本・input-events が消費・2026-07-16 制定）

- **`HitRegion { scope: usize, region: Option<String> }`** 級（最終形は design）: 領域名は**不透明 String**（kanade は解釈せず Reference4 へ転写のみ＝[[areka-surface-args-opaque-string-downstream-resolve]] と同精神）。collision 外は `None`（Ref4 空文字転写・SSP 挙動は ukadoc で確認）。
- **解決の入力**: `(scope, 窓 client 物理 px 座標)`。**「現在の surface id」は emo 側が内部で引く**（呼び手＝input-events は surface を知らなくてよい——emo が UI 層の窓口・kanade へは解決済みイベントのみの原則）。
- **提供形**: UI スレッドで同期呼出可能な resolver（純関数＋現 surface 読み口の薄い合成）。channel 化は不要（入力ハンドラ＝UI スレッド・resolver＝UI 所有データ）。

## Approach

1. **純関数コア（emo-compose 消費層）**: `hit_region(&SurfaceMaster, surface_id, point) -> Option<&str>` 級の純関数（含端・重なり優先の規則を ukadoc で確定し全網羅テスト）。配置は emo-compose の公開形消費（emo-present か emo-compose か新モジュールかは design 判断・**正規化形の再定義はしない**）。
2. **現 surface 読み口（emo-present additive）**: target 別「現在表示中 surface id」の読み API（presenter 内部状態の公開 or 装着層の記録・**本体ロジック無改変の additive**）。
3. **resolver 合成**: scope→target（target_map）→現 surface id→純関数、を束ねた UI スレッド用 resolver を emo2_boot 結線層（`crates/areka/src/emo2_boot/`）に配置——input-events がここだけ握る。
4. **alpha mask との関係整理（design 明文化）**: クリック透過（画素 α）と当たり判定（collision 矩形）は**別層**——透明画素上でも collision 矩形内なら region は解決される（SSP 挙動を ukadoc/実機で確認し規則を檻に）。

## クロスユニット契約（並走を詰ませない事前考慮・2026-07-16）

- **input-events との契約先決**: 上記 I/O 契約が正本＝input-events brief は本節を参照し**再定義しない**（lifecycle→kanade→sakura の「契約の正本連鎖」パターンの再演）。両 spec は並走可（本 spec＝emo 側純粋層＋読み口／input-events＝kanade 側＋UI 配線。結線点は resolver 1 個）。
- **cue-playback-duration と交差面ゼロ**: dola/sakura/emo-text/seriko 不触。emo-present は additive 読み口のみ（cue-playback は emo-present 不触）。
- **将来消費者**: M-dialogue の choice はバルーン側（本 spec はシェル collision のみ）／M2 collisionex（円/多角形）は**型シームだけ**（矩形 enum の余地・実装しない）。seriko-loop が将来 surface 切替を高频度化しても「現 surface 読み口」契約は不変。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-16 裏取り済み）

- **`ukadoc:list_shiori_event:OnMouseMove:1`**（裏取り済み）: Reference4＝**当たり判定の識別子**（本 spec の出力がここへ載る）・Ref0/1＝ローカル座標・Ref3＝本体 0/相方 1。
- **`descript_shell_surfaces` の collision 定義**: `collisionN,x1,y1,x2,y2,名前` の座標意味（左上/右下・含端）・**同一 surface 内の複数 collision の優先順位**（定義順 or ID 順——ここが (d) の檻の根拠）・collision の継承（append/範囲定義との関係）。collisionex は M1 外（emo2 不使用）を確認のみ。
- **具体指示**: design 冒頭で collision 節を読み「矩形照合規則（含端・優先）」「collision 無し時の Ref4 値（空文字か省略か）」「透明画素と collision の関係」の3点を確定表にすること。

## Scope

- **In**: hit→region 純関数（含端・重なり優先・None 経路）／現 surface id 読み口（emo-present additive）／UI スレッド resolver 合成（emo2_boot 結線層）／I/O 契約 `HitRegion` の正本確立／fixture 実値の全網羅 unit。
- **Out**: SHIORI への配信・Reference 組立（**input-events**）／撫で意味論（連打解釈は SHIORI 側の領分）／バルーン側の choice ヒット（M-dialogue `choice-render`）／collisionex（M2・型シームのみ）／AlphaMask・クリック透過の変更（完了済み基盤・不触）。

## Boundary Candidates

- 純関数コア（全網羅・GPU 不要）／現 surface 読み口（additive）／resolver 合成（結線層）。

## Out of Boundary

- マウスイベントの取得・配信経路（wintf 完了基盤＋input-events）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-emo-compose`（`SurfaceMaster.collisions` 正本保持）／`completed/areka-P0-emo-present`（presenter・target）／`completed/areka-P0-emo2-boot`（target_map・結線層）。
- **Downstream**: **`areka-P0-input-events`**（契約の第一消費者）／M-life 統合（撫で一周）／`emo2-conformance-e2e`。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-emo-compose`（正規化形の消費・再定義禁止）。
- **Adjacent**: `areka-P0-input-events`（契約先決の相方・同時 brief 化 2026-07-16）／`areka-P0-cue-playback-duration`（**交差面ゼロ**）。

## Constraints

- Rust 2024・新規依存なし・tokio 不使用・emo-present 本体無改変原則（additive 読み口のみ）。
- **決定論**: 純関数全網羅・GPU/表示不要（[[test-only-decision-branches-not-proven-wiring]]・[[deterministic-test-coverage-mandate]]）。
- 領域名は不透明 String（[[areka-surface-args-opaque-string-downstream-resolve]]）。正典は ukadoc・emo2 は最小適合 fixture（[[ukadoc-mcp-preferred-source]]）。
