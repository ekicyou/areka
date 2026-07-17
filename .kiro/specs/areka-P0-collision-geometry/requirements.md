# Requirements Document

## Introduction

emo2 の撫で反応（`OnMouseMove` の actor×region ルーティング）とメニュー（`OnMouseDoubleClick` の Reference4=当たり判定名）を成立させるには、「マウス座標がどの当たり判定（Head/Bust）に居るか」を解決する層が必要である。現状、parsers→emo-compose の正規化形 `SurfaceMaster` は collision（矩形＋領域名）を保持しているが、どこにも露出・消費されていない。wintf のヒットテストは合成ビットマップ由来の `AlphaMask`（画素の不透明性）しか知らず、部位名を知らない。「見える・触れるは emo が窓口＝kanade へは解決済みイベントだけ渡す」という原則における、その解決役が不在である。

本 spec は、マウス座標（窓 client 物理 px）と scope から当たり判定名（不透明 String・例 "Head"/"Bust"）を**決定論で解決する純粋層**を emo に確立し、`input-events`（③kanade）が消費できる I/O 契約 `HitRegion { scope, region: Option<String> }` を**正本として**立てる。層は (1) hit→region 純関数コア（含端・重なり優先・None 経路）、(2) 現在表示中サーフェス id の読み口（emo-present に対する additive）、(3) それらを束ねる UI スレッド用リゾルバ合成（emo2_boot 結線層）から成る。統合（撫で一周の実機サインオフ）は input-events 側へ委譲し、本 spec の観測は純粋層で独立に完結する。

正典は ukadoc（`collision*,始点X,始点Y,終点X,終点Y,ID` の矩形定義／`OnMouseMove` の Reference4=当たり判定の識別子）であり、emo2 は最小適合 fixture にすぎない。ただし当たり判定が重なった際の優先順位は、SSP の `collision-sort`（既定 none＝先書きが手前）に忠実追従せず、emo の合成規約である**画家のアルゴリズム**（後に定義された領域が手前＝勝ち・`crates/areka-emo-compose/src/blit.rs:83` の「下層→上層」合成と一貫）に揃える（要件ディスカッション議題1で決定・SSP とは逆向き）。

## Boundary Context

- **In scope**:
  - マウス座標（窓 client 物理 px）＋scope→当たり判定名を返す純関数コア（含端・重なり優先・None 経路）。
  - 対象ウィンドウ（target）が現在表示中のサーフェス id を引く読み口（emo-present に対する additive）。
  - scope→target→現サーフェス id→純関数を束ねる UI スレッド同期リゾルバ（emo2_boot 結線層）。
  - region/actor I/O 契約 `HitRegion { scope, region: Option<String> }` の正本確立。
  - emo2 fixture 実 collision 値による全網羅の単体検証（GPU/表示不要）。
- **Out of scope**:
  - SHIORI への配信・Reference の組立（`input-events` の責務）。
  - 撫で意味論（連打・撫でカウンタ等の解釈は SHIORI 側の領分）。
  - バルーン側の choice ヒット（`M-dialogue` の `choice-render`）。
  - 不定形当たり判定 `collisionex`（円/楕円/多角形/region・M2・型シームのみ・emo2 不使用）。
  - `AlphaMask`・クリック透過の変更（完了済み基盤・不触）。
  - マウスイベントの取得・配信経路（wintf 完了基盤＋`input-events`）。
- **Adjacent expectations**:
  - Upstream `areka-P0-emo-compose` の `SurfaceMaster.collisions` を消費する（正規化形の再定義はしない）。
  - Upstream `areka-P0-emo-present`（presenter・target）／`areka-P0-emo2-boot`（target_map・結線層）に依存する。
  - Downstream `areka-P0-input-events` が `HitRegion` 契約の第一消費者であり、本節を参照して再定義しない。region が `None` の場合を空文字 Reference4 へ転写するのは `input-events` の責務である。
  - 座標契約は emo-present の「物理 px 等倍」に整合する（窓 client 物理 px をサーフェス px と同一空間で照合）。DPI 契約の明文確認は design で行う。
  - 含端規則（境界の内外）は design 冒頭で ukadoc collision 節から確定表にする。重なり時の優先順位は**画家のアルゴリズム（後に定義された領域が手前）で確定済み**（議題1）＝SSP `collision-sort` の忠実解決は行わず、差し替え用の型シーム（`SortOrder` 相当を予約）のみ備える。既存 wintf `HitRegionMap` は先勝ち（逆向き・`crates/wintf/src/ecs/layout/hit_region/mod.rs:356`）ゆえ、統合可否は design で判断する。
  - 実装制約: Rust 2024・新規依存なし・tokio 不使用・emo-present 本体無改変（additive の読み口のみ）。

## Requirements

### Requirement 1: 当たり判定名の決定論的解決（純関数コア）
**Objective:** As a emo（見える・触れるの UI 層窓口）, I want マウス座標から当たり判定名を決定論で得られる純関数, so that input-events が surface を知らずに解決済み領域名だけを受け取れる

#### Acceptance Criteria
1. When 与えられた点がいずれかの当たり判定矩形の内側にある, the 当たり判定解決関数 shall その矩形に対応する領域名を不透明 String として返す。
2. When 与えられた点がどの当たり判定矩形の内側にもない, the 当たり判定解決関数 shall None を返す。
3. When 対象サーフェスに当たり判定が1つも定義されていない, the 当たり判定解決関数 shall None を返す。
4. When 点が当たり判定矩形の境界上にある, the 当たり判定解決関数 shall 単一の正典含端規則（design で ukadoc collision 節から確定）に従い決定論的かつ一貫して解決する。
5. While 同一の入力（サーフェスの当たり判定集合と点）が与えられている間, the 当たり判定解決関数 shall 常に同一の結果を返す。

### Requirement 2: 重なり領域の優先順位（画家のアルゴリズム）
**Objective:** As a 複数の当たり判定が重なりうるシェルを扱う emo, I want 重なり時の優先順位を emo の合成規約（画家のアルゴリズム）と一貫させる, so that 撫で先が曖昧にならず、見た目で手前の層と撫で解決が一致する

#### Acceptance Criteria
1. When 点が複数の当たり判定矩形に同時に含まれる, the 当たり判定解決関数 shall 画家のアルゴリズム（後に定義された矩形が手前）に従い、最も後に定義された領域の名前を決定論で返す。
2. The 当たり判定解決関数 shall この優先規則を単一の決定論規則として適用し、SSP の `collision-sort` 宣言（none/ascend/descend）の忠実解決には依存しない（emo2 は `collision-sort` 未宣言）。
3. The 当たり判定解決層 shall 優先順位規則を将来差し替え可能な型シーム（`SortOrder` 相当の enum を予約）として構成し、本 spec では画家のアルゴリズム（後定義が手前）のみを実装する。

### Requirement 3: 現在表示中サーフェスの読み口（emo-present additive）
**Objective:** As a 当たり判定リゾルバ, I want 対象ウィンドウ（target）が現在表示中のサーフェス id を引く読み口, so that 呼び手が surface を知らなくても正しい当たり判定集合を選べる

#### Acceptance Criteria
1. When ある target にサーフェスが表示された後にその target の現サーフェス id が問い合わされる, the 現サーフェス読み口 shall 直近に表示されたサーフェスの id を返す。
2. When ある target がまだ一度もサーフェスを表示していない状態で問い合わされる, the 現サーフェス読み口 shall 「現サーフェス無し」を表す値を返す。
3. When 表示中サーフェスが別の id へ切り替わった, the 現サーフェス読み口 shall 以後の問い合わせに対し新しいサーフェス id を返す。
4. The 現サーフェス読み口 shall 既存の表示（present）本体ロジックを変更しない追加的（additive）な読み取りのみで提供され、既存の表示挙動を退行させない。

### Requirement 4: 座標とスコープからの当たり判定リゾルバ（UIスレッド同期）
**Objective:** As a input-events（③kanade）, I want (scope, 窓 client 物理 px) を渡すと解決済み結果を同期で得られる単一の窓口, so that surface 解決の詳細を持たずに撫で/メニューを配信できる

#### Acceptance Criteria
1. When (scope, 窓 client 物理 px 座標) が与えられる, the 当たり判定リゾルバ shall scope→target→現サーフェス id→純関数を束ねて当たり判定名の解決結果を返す。
2. The 当たり判定リゾルバ shall UI スレッド上で同期呼出可能であり、channel 化や非同期化を要さない。
3. The 当たり判定リゾルバ shall 入力座標を当たり判定矩形と同一の座標空間（サーフェス px）で照合する。
4. When scope に対応する現サーフェス id が解決できない（未表示等）, the 当たり判定リゾルバ shall region を None とした解決結果を返す。

### Requirement 5: region/actor I/O 契約（HitRegion 正本）
**Objective:** As a 契約の第一消費者 input-events, I want region/actor の I/O 契約が本 spec を正本として一意に定まる, so that 両 spec が再定義せず並走できる

#### Acceptance Criteria
1. The 当たり判定解決層 shall 解決結果を `HitRegion { scope, region: Option<String> }` 相当の形（最終形は design）で提供する。
2. The 当たり判定解決層 shall region を不透明 String として扱い、意味解釈（撫で意味論・Reference 組立）を行わない。
3. When 点がどの当たり判定にも該当しない, the 当たり判定解決層 shall region を None として返す。
4. The 当たり判定解決層 shall この I/O 契約を正本として提供し、input-events がこれを参照し再定義しないことを前提とする。

### Requirement 6: 透明画素と当たり判定の層分離
**Objective:** As a emo（見える・触れるの窓口）, I want 透明画素（クリック透過）と当たり判定（領域）を別層として扱う, so that 透明部分でも正しく撫で領域を解決でき SSP 挙動に整合する

#### Acceptance Criteria
1. When 点が透明画素上にありかつ当たり判定矩形の内側にある, the 当たり判定解決関数 shall 画素の α に関わらず領域名を解決する。
2. The 当たり判定解決層 shall 既存の `AlphaMask`（クリック透過・is_hit）を変更せず、当たり判定の解決をそれとは独立に行う。

### Requirement 7: 決定論的テスト網羅（品質要件）
**Objective:** As a 品質を担保する開発者, I want GPU/表示なしで当たり判定解決を全網羅テストできる, so that 回帰檻を決定論で構築できる

#### Acceptance Criteria
1. The 当たり判定解決関数 shall GPU・実描画・実表示を要さず、純関数として全経路を単体テスト可能である。
2. Where emo2 fixture の実 collision 値が用いられる, the 当たり判定解決層 shall (a) 矩形内→領域名、(b) 矩形外→None、(c) 境界 on/off、(d) 重なり優先、(e) collision 未定義→None の各判断分岐を網羅検証できる。
3. The 当たり判定解決層 shall 統合（撫で一周の実機サインオフ）を input-events 側へ委譲し、本 spec の観測は純粋層で独立に完結する。
