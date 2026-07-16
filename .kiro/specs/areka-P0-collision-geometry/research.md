# ギャップ分析: areka-P0-collision-geometry

> 対象: 確定済み requirements.md（R1〜R7）と既存コードベースの差分分析。
> 目的: 実装戦略と設計判断項目の抽出（決定はしない・選択肢と論点の提示）。
> 調査日: 2026-07-16（validate-gap フェーズ・実装シーム全数検証）。

## 1. 分析サマリ

- **データは完全に揃っている**: parsers `Collision`（矩形＋不透明名）→ `SurfaceMaster.collisions`（正規化形）→ `EmoWorld::surface(id)` で読める、という経路は既に存在し、`surface.append` 経由の kero 側 collision も fold で `SurfaceMaster.collisions` へ合流済み（末尾連結）。本 spec が消費する正規化形の**再定義は不要**。
- **消費層だけが不在**: `SurfaceMaster.collisions` の実行時消費者はゼロ（emo-present の参照は全てテスト用 `Vec::new()`）。座標→領域名の純関数コア・現サーフェス id 読み口・resolver 合成の 3 部品が新規で、いずれも小さい。
- **既存の類似実装が wintf に存在**（重要な再利用判断）: `wintf/src/ecs/layout/hit_region/mod.rs` の `HitRegionMap::hit_test_region` は矩形＋多角形＋カラーマップの名前付き領域解決を既に持ち、**定義順先勝ち**規則も実装済み。ただし f32 DIP＋正規化座標・wintf の ECS Component・別系譜（completed `event-hit-test-named-regions`）であり、本 spec の「物理 px 等倍・整数照合・emo 帰属・不透明 String」契約とは座標/型/レイヤ帰属が食い違う。**再利用 vs 新規**が最大の設計論点。
- **現サーフェス id の読み口は真の欠落**: presenter（`EmoPresenter`/`PresentTarget`）に「その target がいま表示中の surface id」を返す API もフィールドも無い（`cache` の私有 `ComposeKey` に間接的に残るのみ・無効化で消える）。ただし additive 読み取り view の前例（`TextSlotView`）があり、それを踏襲できる。
- **collision-sort の伝播ギャップ**: `collision_sort` は parsers の `Shell` 直下フィールド（`model.rs:31`）だが、fold の `normalize_surface` は `SurfaceMaster` へ写さない（`SurfaceMaster` は id/elements/collisions/animations のみ）。R2.1（正典 collision-sort の順位）を厳密に満たすには伝播経路が要る。emo2 fixture は `collision_sort=None`（定義順既定＝R2.2）ゆえ fixture 検証は成立するが、契約としての R2.1 は決定必要。

## 2. 現状調査（既存資産・file:line）

### 2.1 collision データ経路（揃っている）
- **parsers 転記型** `crates/areka-parsers/src/shell/model.rs:149-181`
  - `Collision { index:u32, left/top/right/bottom:i64, name:CollisionName }`（`:152-165`）。座標は **i64**、ukadoc 順 始点X/始点Y/終点X/終点Y = left/top/right/bottom（`:150`）。
  - `CollisionName(String)`（`:169`）= 不透明 NewType、`as_str()` 読み取りのみ（`:178`）。意味解釈は下流（要件 6.2）。
  - `Surface.collisions: Vec<Collision>`（`:71`・出現順）。`SurfaceAppend.collisions`（`:193`・通常 surface と同一型）。
  - **collision-sort は Shell 直下**: `Shell.collision_sort: Option<SortOrder>`（`:31`・未指定 None・既定解釈は下流）。`SortOrder::{Ascend,Descend}`（`:40-45`）。
- **emo-compose 正規化形** `crates/areka-emo-compose/src/normalized.rs:70-80`
  - `SurfaceMaster { id:u32, elements, collisions: Vec<areka_parsers::shell::Collision>, animations }`（`:77` が collision 保持）。**collision_sort フィールドは無い**。
  - `EmoWorld::surface(id) -> Option<&SurfaceMaster>`（`crates/areka-emo-compose/src/world.rs:108`）で id 引き可能。純関数コアの入力源はこれ。
- **fold（append 合流）** `crates/areka-emo-compose/src/fold.rs:92-127`
  - `fold_append` が `master.collisions.extend(append.collisions...)`（`:122`・末尾連結・転記のまま）。→ **kero 側 `surface.append10` の Head/Bust も対象 surface の `SurfaceMaster.collisions` へ入る**（存在条件付き・非存在 id は新設せず warn）。
  - `normalize_surface`（plain）は `Shell.collision_sort` を参照せず SurfaceMaster を生成（collision-sort 非伝播の起点）。

### 2.2 消費層の不在（＝本 spec のスコープ）
- **emo-present に collision の実行時消費なし**: `presenter.rs` の `collisions:` 参照は全てテスト補助の `Vec::new()`（`:509` / `:1165` / `:1172`）。production 経路は collision を一切見ない。
- **現サーフェス id 読み口なし**: `PresentTarget`（`presenter.rs:52-69`）のフィールドは `emo_world/atlas/composer/cache/window/mount/chain/visible` のみ。**現在表示中 surface id の保持フィールドが無い**。`apply_show`（`:201-`）が surface_id を受けて表示するが、その値は `cache` の私有 `ComposeKey{surface_id,binds}`（`cache.rs:95`）に残るだけ（`get`/`insert`/`invalidate_all` のみ公開・`ComposeKey` は非公開・`invalidate_all` で消える）。
- **additive read-only view の前例**: `TextSlotView`（`presenter.rs:80-115`）＝スナップショット値＋accessor のみ・状態変更手段を持たない読み取り専用 view。`EmoPresenter::text_slot_view()` が返す。R3 の「現サーフェス読み口（additive）」はこのパターンを踏襲できる。

### 2.3 AlphaMask（α 由来・別層である根拠）
- **AlphaMask は wintf 所有**: `crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs:16-104`。`from_pbgra32`（`:33`・閾値 128 で 2 値化）・`is_hit(x,y)`（`:67`）。**α のみから導出**され領域名を持たない。
- emo-present は合成結果からこれを生成: `cache.rs:82-99`（`insert` 内で `AlphaMask::from_pbgra32` を 1 回だけ・R2.1）。
- wintf ヒットテスト側の消費: `AlphaMaskResource`（`hit_test/mod.rs:158-177`）＋ `alpha_mask_hit`（`:199-`）。**マスク原寸＝surface 原寸物理 px で恒等写像・任意 DPI で座標一致**（`:150-153` 座標契約）。→ R6（透明画素と当たり判定の層分離）は「AlphaMask は不触・collision は別純関数」で自然に成立する。

### 2.4 現サーフェス id の追跡経路（wiring）
- **指令発行** `crates/areka/src/emo2_boot/adapter.rs`: `map_display_command`（`:34`）が seriko の `DisplayCommand::{Show,ShowBalloon}` → `PresentCommand::ShowSurface{target,surface_id,binds}`（`:41`/`:54`）。surface_id は発行時点で既知（seriko 解決済み数値・alias 非再適用）。
- **搬送** `PresentBridge`（`adapter.rs:78-`）が `std::sync::mpsc::Sender<PresentCommand>` で送信 → UI スレッドで `EmoPresenter::apply` が適用。→ **「最後に適用された surface id」を UI スレッドで唯一知り得るのは presenter**（bridge は送信側・resolver は UI スレッド読み側）。

### 2.5 scope→target 写像（既存正本・そのまま利用）
- `crates/areka/src/emo2_boot/target_map.rs`: `shell_target(scope)=TargetId(2*scope)`（`:19`）／`balloon_target=TargetId(2*scope+1)`（`:27`）／`scope_of(&ActorKey)->Option<u32>`（`:36`・非数値 None）。純粋・std のみ・偶奇分離で互いに素。resolver の scope→target 段はこれを消費。

### 2.6 既存の名前付きヒット領域（wintf・再利用候補）
- `crates/wintf/src/ecs/layout/hit_region/mod.rs`（completed `event-hit-test-named-regions`）
  - `HitRegionMap`（ECS Component）: `Shape::Rect{x,y,width,height:f32}` / `Shape::Polygon` / `ColorMap`（`:87-97`）。
  - `hit_test_region(rel_x,rel_y, entity_size:&Size) -> Option<&str>`（`:349-385`）: **正規化座標（0.0〜1.0）→ DIP ローカル**・**定義順先勝ち**（`:356-357` コメント＋ループ）・矩形は**両端含む**（`local_x >= x && local_x <= x+width`・`:365-368`）。
  - `ColorMapData::hit_test(pixel_x,pixel_y)`（`:220`）は画素→領域。多角形は Ray Casting（`:477`）。
  - **食い違い**: 座標が f32 DIP＋正規化（本 spec は物理 px 等倍・整数 i64）／wintf の ECS Component（本 spec は emo 帰属・純関数）／`Shape::Rect` は `x,y,w,h`（本 spec の `Collision` は left/top/right/bottom）。**依存方向**: emo-compose は wintf 非依存が憲章（`lib.rs:25`「wintf の visual/window/WUC/描画 API には依存しない」）。emo-present は wintf 依存済み。

### 2.7 emo2 fixture 実 collision 値（全網羅テストの素材）
`crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/surfaces.txt`
- **さくら側 surface1000**（`:23-24`）: `collision0,93,62,271,130,Head` / `collision1,133,270,229,326,Bust`。
- **kero 側 `surface.append10,2100-2110,2200-2210`**（`:417-418`）: `collision0,52,38,156,80,Head` / `collision1,82,163,140,186,Bust`。
- **矩形のみ・領域は Head/Bust の 2 種・collisionex 不使用・collision-sort 宣言なし**。→ (a)矩形内→名前 (b)矩形外→None (c)境界 on/off (d)重なり（Head と Bust は非重なりだが人工重なり値で檻） (e)collision 未定義 surface→None を素の値で網羅可能。

## 3. 要件→資産マップ（ギャップ tag）

| 要件 | 依存資産（既存） | ギャップ | tag |
|---|---|---|---|
| R1 純関数コア（内/外/未定義/含端/決定論） | `SurfaceMaster.collisions`（normalized.rs:77）・`Collision` 矩形（model.rs:152） | 純関数 `hit_region` が未実装。含端規則は未確定（ukadoc） | Missing / Unknown |
| R2 重なり優先（collision-sort／既定=定義順） | `Shell.collision_sort`（model.rs:31）・wintf 定義順先勝ち前例（hit_region:356） | `collision_sort` が `SurfaceMaster` へ非伝播。R2.1 の正典順位を満たす経路が無い（R2.2 の定義順は素で可） | Missing / Constraint |
| R3 現サーフェス読み口（additive） | presenter `apply_show`・`cache` slot・`TextSlotView` 前例 | 現 surface id を返す API/フィールドが無い。Hide/invalidate 時の意味論が未定 | Missing / Unknown |
| R4 resolver（UI スレッド同期） | `target_map`（既存正本）・`EmoWorld::surface`（world.rs:108） | scope→target→現 id→純関数を束ねる resolver が未実装。EmoWorld は PresentTarget 私有ゆえ到達経路が要る | Missing / Constraint |
| R5 HitRegion I/O 契約（正本） | emo-present は「指令 API 契約正本」の前例（lib.rs:10） | `HitRegion{scope,region:Option<String>}` 型が未定義。定義先 crate 未定 | Missing |
| R6 透明画素と当たり判定の層分離 | `AlphaMask`（wintf・α 由来）・`AlphaMaskResource` 不触方針 | 既存基盤で自然成立。collision を α と独立に解く純関数が要るのみ | Constraint |
| R7 決定論テスト網羅（GPU 不要） | emo2 実値（2.7）・in-crate `#[cfg(test)]` 慣行 | 純関数化できれば全網羅可。現 id 読み口の檻は additive 観測で | Constraint |

## 4. 実装アプローチ選択肢

### 論点A: 純関数コアの配置と再利用
- **A-1: emo-compose に新規純関数（`SurfaceMaster.collisions` を直接照合）**
  - `hit_region(&SurfaceMaster, point) -> Option<&str>` 級を emo-compose に置く。整数 i64 照合・不透明 String・wintf 非依存を保つ。
  - ✅ 憲章（emo-compose=wintf 非依存の純粋層）と整合／整数照合で物理 px 等倍を厳密に／全網羅 unit が GPU 不要で自然。
  - ❌ wintf `HitRegionMap` の矩形照合ロジックと二重化（ただし規則は自明・小さい）。emo-compose の責務が「合成」から「hit 解決」へ拡張（スコープ判断）。
- **A-2: emo-present に純関数＋wintf `HitRegionMap` を再利用**
  - `Collision` → `ShapeRegion`（`Shape::Rect`）へ適合し `hit_test_region` を呼ぶ。emo-present は wintf 依存済みゆえ可能。
  - ✅ 多角形/カラーマップ/定義順先勝ちを無償で獲得（将来 collisionex の布石）。
  - ❌ **f32 DIP＋正規化座標へ変換**が必須＝物理 px 等倍の整数厳密性が崩れる（丸め・境界の非決定リスク）。`Collision(i64 left/top/right/bottom)` → `Rect(f32 x/y/w/h)` の変換層が要る。wintf の含端（両端含む）が正典と一致する保証なし。emo-compose ではなく emo-present へ核が寄る。
- **A-3: 新規モジュール/薄クレート（emo-compose の型のみ import）**
  - hit 解決専用モジュールを新設し `SurfaceMaster`/`Collision` を消費。
  - ✅ 責務が最も明確・emo-compose 合成憲章を汚さない。
  - ❌ ファイル/クレート増。配置先（areka-emo-* か emo2_boot 内か）自体が判断。
- 所見: **物理 px 等倍・整数・不透明 String・wintf 非依存**という要件群は A-1（emo-compose に純関数）へ強く傾く。wintf `HitRegionMap` 再利用（A-2）は座標系・依存方向・含端の不一致コストが大きく、M2 collisionex（型シームのみ）まで多角形は不要。ただし「emo-compose の責務拡張の是非」は要件ディスカッション判断。

### 論点B: 現サーフェス id 読み口（R3）
- **B-1: `PresentTarget` に `current_surface_id: Option<u32>` を追加、`apply_show` 成功時に更新、`EmoPresenter::current_surface_id(target)->Option<u32>` を additive 公開**（`TextSlotView` 前例踏襲）。
  - ✅ 本体ロジック無改変の純増（R3.4）・UI スレッド同期読み取り・Hide/invalidate の意味論を明示制御できる。
  - ❌ 新フィールド 1 個＋更新点 1 個（apply_show）。Hide 時に None へ倒すか last を保つかを決める必要（R3.2 は「未表示→無し」）。
- **B-2: `cache` slot の `ComposeKey.surface_id` から導出**（読み取り専用アクセサを足す）。
  - ✅ 追加状態ゼロ。
  - ❌ `invalidate_all` で消える・Hide で残る＝現サーフェスの意味と乖離。`ComposeKey` 私有の公開が要る。脆く R3.3 の切替追従が不安定。
- **B-3: wiring 層（emo2_boot）で ShowSurface 発行時に scope→現 id を記録**（UI スレッド所有の `BTreeMap<TargetId,u32>` 等）。
  - ✅ presenter 無改変。
  - ❌ 記録点（bridge 送信側）と読み取り点（UI スレッド resolver）の同一化が要り、二重真実源になりやすい（presenter が実際に適用したか＝失敗時の乖離）。log-first 規律で失敗経路と齟齬。
- 所見: **B-1 が本体無改変・単一真実源・意味論明示で最有力**。B-3 は「emo が現 surface を内部で引く」原則（brief）と、失敗時の乖離リスクで劣位。

### 論点C: resolver 合成と HitRegion 契約の帰属（R4/R5）
- EmoWorld は `PresentTarget` 私有ゆえ、resolver は presenter を経由せざるを得ない。二案:
- **C-1: presenter が結合メソッドを公開** `EmoPresenter::hit_region(target, point) -> Option<String>`（内部で 現 id → `emo_world.surface(id)` → 純関数）。emo2_boot resolver は `scope→target_map→presenter.hit_region` の薄い glue のみ。
  - ✅ EmoWorld 私有を漏らさない・resolver が最小・UI スレッド同期。
  - ❌ presenter に hit 解決の口が増える（純関数コアは別 crate でも呼び出しは presenter 内）。
- **C-2: presenter は `current_surface_id` と `surface_master(id)` の 2 read を公開**、resolver が純関数を呼ぶ。
  - ✅ 純関数の呼び出し主体が emo2_boot に集約・presenter は薄い。
  - ❌ `&SurfaceMaster` の借用を presenter 外へ出す寿命設計が要る（EmoWorld 借用の露出）。
- **HitRegion 型の定義先**: (i) emo-present（既に「指令 API 契約正本」・R5 の正本性と整合） (ii) emo-compose（純関数の戻り値近傍） (iii) emo2_boot（結線層・input-events が握る 1 点）。brief は「resolver を emo2_boot に配置＝input-events がここだけ握る」。→ 契約型（`HitRegion`）は emo-present か emo2_boot が候補、純関数の戻りは `Option<&str>`/`Option<String>` で十分。要件ディスカッション判断。
- 所見: **C-1（presenter 結合メソッド）＋ HitRegion は emo2_boot もしくは emo-present に定義**が最小結線。純関数コアの配置（論点A）と契約型の配置は分離してよい。

## 5. 工数・リスク

- **総工数: S（1〜3日）**。純関数・additive 読み口・薄い resolver の 3 部品はいずれも小さく、上流データ・target_map・EmoWorld アクセサが既存。
- **リスク: Low〜Medium**。実装難度は低い。Medium 要因は**実装でなく設計判断**に集中:
  - collision-sort 伝播の是非（R2.1 の契約解釈・正規化形不改変原則との緊張）。
  - 現 id 読み口の Hide/invalidate 意味論。
  - 含端規則・collision 無し時の Ref4 値・透明画素との関係の ukadoc 確定（design 冒頭）。
- 個別:
  - 純関数コア（論点A）: S / Low（全網羅 unit・GPU 不要）。
  - 現 id 読み口（論点B-1）: S / Low〜Medium（意味論設計）。
  - resolver 合成（論点C）: S / Low（glue・target_map 既存）。

## 6. 設計判断項目（要件ディスカッションへ送る）

1. **純関数コアの配置**: emo-compose に新規純関数（A-1・wintf 非依存/整数厳密）か、emo-present で wintf `HitRegionMap` 再利用（A-2・座標変換コスト）か、新規モジュール（A-3）か。emo-compose の責務を「合成」から「hit 解決」へ広げてよいか。
2. **wintf `HitRegionMap` 再利用の是非**: 既存の多角形/カラーマップ/定義順先勝ちを取り込むか、物理 px 整数厳密性と依存方向（emo→wintf）を優先して新規矩形照合にするか。M2 collisionex を見据えた型シームの持たせ方。
3. **collision-sort の伝播**（R2.1）: `SurfaceMaster`（または EmoWorld resource）へ `collision_sort` を運ぶか（正規化形不改変原則との整合を design で判断）、R2.1 は将来 spec / 型シームへ委ね本 spec は R2.2 の定義順既定に閉じるか。emo2 は `collision_sort=None` ゆえ fixture は後者で成立。
4. **現サーフェス id 読み口の形と意味論**（R3）: presenter へ `current_surface_id` フィールド＋additive accessor（B-1）か、cache 由来（B-2）か、wiring 層記録（B-3）か。**Hide/InvalidateCache 時に現 id を None へ倒すか last を保つか**（R3.2「未表示＝無し」との整合）。
5. **resolver 合成の口**（R4）: presenter が結合メソッド `hit_region(target,point)`（C-1）か、`current_surface_id`＋`surface_master` の 2 read（C-2）か。EmoWorld 借用を presenter 外へ露出するかどうかの寿命設計。
6. **HitRegion 契約型の最終形と定義 crate**（R5）: `HitRegion{scope,region:Option<String>}` を emo-present（契約正本の前例あり）／emo-compose／emo2_boot のどこに置くか。純関数の戻りは `Option<&str>` か所有 `Option<String>` か（input-events への搬送は所有形が要る）。
7. **座標/点型と DPI 契約**: 入力点の型（i64 / u32 / `PointF`）と `Collision` i64 の照合空間。物理 px 等倍（`CURRENT_COMPOSE_SCALE=1.0`・presenter.rs:118／AlphaMaskResource 座標契約 hit_test/mod.rs:150-153）を明文で固定する。

## 7. Research Needed（design 冒頭で ukadoc 確定表化・brief 指示）

- **含端規則**: `collisionN,始点X,始点Y,終点X,終点Y,名前` の矩形が境界を含むか（左上含む/右下含む等）。wintf `HitRegionMap` は両端含む（`hit_region:365-368`）が前例にすぎず、正典は ukadoc `descript_shell_surfaces` collision 節。
- **同一 surface 内の複数 collision の優先順位**: 定義順か ID 順か（R2 の (d) 檻の根拠）。`collision-sort` 宣言時/非宣言時の差。
- **collision 無し時の Ref4 値**: 空文字か省略か（R5.3・input-events が None を空文字転写する前提の裏取り・SSP 挙動）。
- **透明画素と collision の関係**: 透明画素上でも collision 矩形内なら region 解決（R6・SSP 実挙動）。
- **collisionex（M1 外の確認）**: emo2 不使用・M2 型シームのみであることの確認（`OnMouseMove` Reference4＝当たり判定識別子・Ref0/1＝ローカル座標・Ref3＝本体0/相方1 は brief で裏取り済み）。
- 参照手段: ukadoc MCP `search_docs`/`get_doc`（`list_shiori_event:OnMouseMove` ・`descript_shell_surfaces` collision 定義）。外部依存の追加調査は不要（新規依存なし・std＋既存基盤のみ）。

## 8. design フェーズへの推奨

- **推奨アプローチ（暫定・ディスカッションで確定）**: 純関数コア＝A-1（emo-compose に整数厳密・wintf 非依存の純関数）／現 id 読み口＝B-1（presenter additive フィールド＋accessor・`TextSlotView` 踏襲）／resolver＝C-1（presenter 結合メソッド＋emo2_boot 薄 glue）。理由: 物理 px 等倍・整数・不透明 String・wintf 非依存・本体無改変・単一真実源の全要件に最小コストで整合し、全網羅 unit を GPU 不要で成立させられる。
- **design 冒頭で必ず確定**: (a) ukadoc collision 節の含端・優先・None 値・透明画素関係の確定表（§7）、(b) collision-sort 伝播の可否と R2.1 のスコープ（§6-3）、(c) 現 id 読み口の Hide/invalidate 意味論（§6-4）、(d) HitRegion 型の定義 crate と点型/DPI 契約（§6-6/7）。
- **持ち越す研究項目**: §7 の ukadoc 5 点のみ。外部依存調査は無し。
