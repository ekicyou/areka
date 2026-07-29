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

## 9. 要件ディスカッション決定（議題1・2026-07-17）

**論点**: §6-3（collision-sort の伝播）＝重なり時の優先順位規則の確定。

**決定（開発者裁定）**: 重なり時の優先順位は **emo の合成規約＝画家のアルゴリズム**（後に定義された領域が手前＝勝ち）に一貫させる。SSP の `collision-sort`（none/ascend/descend）には**忠実追従しない**。

- **根拠（実コード裏取り）**:
  - emo 合成は画家のアルゴリズム＝下層→上層（`crates/areka-emo-compose/src/blit.rs:83`「命令を順に転写（画家のアルゴリズム・下層から上層）」）＝**後定義が手前**。
  - SSP `collision-sort none` は「先に書かれている方が手前」（ukadoc `descript_shell_surfaces` collision-sort）＝**先定義が手前**＝画家のアルゴリズムと**逆向き**。
  - 既存 wintf `HitRegionMap::hit_test_region` は「定義順の先勝ち」（`crates/wintf/src/ecs/layout/hit_region/mod.rs:356`）＝SSP `none` と同じ＝**画家のアルゴリズムとは逆**。
- **要件への反映**: R2.1＝画家のアルゴリズム（後定義が手前）／R2.2＝SSP `collision-sort` 忠実解決に非依存／R2.3＝`SortOrder` 相当の型シーム予約（本 spec は画家一本のみ実装）。従来 R2.2「先書きが手前」は反転。
- **design へ持ち越す論点（更新）**:
  - §6-3（collision-sort 伝播）は**不要化**（SSP 忠実 sort を実装しないため `collision_sort` の `SurfaceMaster` 伝播は本 spec 不要）。代わりに `SortOrder` 型シームの置き場所（emo-compose 純関数近傍が候補）を design 判断。
  - **既存 wintf `HitRegionMap`（先勝ち）との不一致の扱い**: 本 spec の純関数は逆向き（後勝ち）ゆえ、A-2（HitRegionMap 再利用）はロジック不一致がさらに増える＝§4 論点A で A-1（emo-compose 新規純関数）へさらに傾く。design で「統合／別実装／将来 reconcile」を明記する。
  - 実装形: 「矩形リストを後方から評価し最初に当たった領域を返す（逆順先勝ち）」＝画家のアルゴリズム等価の決定論実装。

## 10. 隣接 spec 衝突スイープ（2026-07-17・要件確定後）

> **目的**: 本 spec と衝突しうる他仕様の全数確認。走査 **20 本**（active 10＋契約確立済み completed 10）・衝突主張 7 件は全て独立した敵対的検証で棄却＝**spec 対 spec の衝突 0 件**。
> **本節の位置づけ**: 下記 10.2 は**未決**であり、**main での総合判断（別セッション）へ渡す情報**＝本 spec 単独では決着させない（開発者方針 2026-07-17・`input-events` 側でも衝突が検知されたため両 spec を並べて裁く）。

### 10.1 結論: spec 対 spec の衝突なし

- §9 の画家則決定と `HitRegion` 正本主張は、いずれも他 spec からの**反証を受けていない**。
- `areka-P0-input-events`（brief.md:5,19,40）が「**正本は collision-geometry**／消費側は**再定義しない**」と3箇所で明記。同 spec は brief のみ＝競合定義が存在し得ない。
- `event-hit-test-*` 4本: `AlphaMask` は α 由来のみで**領域名を持たない**＝R6 の層分離は自然成立（`wintf-clickthrough-alpha-toggle` も同様・不触で両立）。
- `choice-render` の hit は `TextRegion` 行矩形＋バルーン窓（target 奇数）＝本 spec（shell・target 偶数）と互いに素。
- 走査済み（生存衝突ゼロ）: `input-events` / `seriko-loop` / `mayuna-compose` / `cue-playback-duration` / `choice-render` / `choice-select-events` / `emo2-conformance-e2e` / `idle-talk` / `position-persist` / `sakura-dialogue-tags` / `completed:` `event-hit-test-named-regions` `event-hit-test-alpha-mask` `event-hit-test` `event-hit-test-cache` `emo-compose` `emo-present` `emo2-boot` `kanade` `wintf-clickthrough-alpha-toggle` `event-mouse-basic`。

### 10.2 【✅決着済み 2026-07-17】steering「本番ゴースト先行の原則」と R7.3 の緊張

> **決着（2026-07-17 ポートフォリオ合流セッション・要件本文へ反映済み）**: 下記候補案の**2段観測分割を採択**し、input-events research §6.2 の保存選択肢 **(A') 合流サインオフ＋(C') collision-geometry 先行**と複合。R7 を全面改稿（Introduction・Adjacent expectations も追随）:
> - **(a) 純関数コア**＝全網羅 unit（現行のまま・R7.1/7.2）。
> - **(b) リゾルバ座標契約**＝本 spec 内の probe（example）で実 DPI（≠96）・本番 emo2 表示の実測証跡を必須化（R7.3 新設・丸投げ禁止）。**敵対的検証で検出した罠を封じる条件**: probe の入力点は**実表示窓の client 座標経路**から取得（サーフェス空間の collision 値から合成した点の直接注入は不可＝「dpi=96 自己整合が欠陥を隠す」と同型のトートロジー回避）＋ k=1.0（窓 client 寸法＝surface px 寸法）の assert を含む。
> - **(c) 撫で一周の統合サインオフ**＝撫でクラスタ合流サインオフとして input-events Req8.3 が1回実施（R7.4 新設・本 spec resolver の main マージが前提・mock resolver では完了と見なさない）。probe は表示側契約まで＝マウス由来座標との空間一致は合流サインオフが所有（検証の空白なし）。
> - **完了順**: collision-geometry 先行完了（probe 証跡で自前完結）→ input-events は実装並走（mock 檻）・完了のみ合流待ち＝完了順デッドロック解消。
> - 実装注記: probe example から届くよう、リゾルバ合成は `crate::` パス無しモジュールか lib crate 側へ（`emo2_boot/mod.rs` は `crate::` 参照ありで example include 不能・window-placement example 前例）。

以下は決着前の記録（履歴保存）:

- **相手**: `.kiro/steering/roadmap.md`（**steering＝spec より上位の権威**・spec スイープの走査面外だった）。
- **該当文（本 spec を名指し）**: 「上記規約〔fixture/mock で観測を独立化〕は**純粋層**にのみ適用。**UI 位置決め・座標系ユニット（window-placement・collision-geometry 等）は逆**——本番ゴースト（emo2 実 surface 表示）＋**実 DPI（≠96）実行**が観測条件であり、単発デモへの合わせ込みは**無効**」（2026-07-05 追記・**window-placement リジェクトの教訓**・記憶 `areka-placement-real-ghost-first`／`areka-window-placement-dpi-coordinate-defect`）。
- **本 spec の R7.3**: 「統合（撫で一周の実機サインオフ）を input-events 側へ委譲し、**本 spec の観測は純粋層で独立に完結する**」＝steering が本 spec に**名指しで禁じた姿勢**そのもの。
- **評価**: R1/R2 の純関数コアは**真の純粋層**＝unit 檻で正当。緊張は **R4 リゾルバ＝座標系シーム**（窓 client 物理 px→サーフェス px）に集中＝「dpi=96 の自己整合が欠陥を隠す」層。
- **未採用の候補案（判断材料）**: 観測を2段に分割＝(a) 純関数＝全網羅 unit（現行のまま）／(b) **リゾルバの座標契約＝実 DPI（≠96）・本番 emo2 表示での証跡を必須**とし input-events へ丸投げしない＋`input-events` 側にも DPI≠96 証跡義務を明記する coordination note。最悪形＝**双方が「相手がやる」と書いて誰も検証しない**。
- **判断者への申し送り**: 本項は R7.3（および R7 全体の観測契約）の書き換えを要する可能性がある。`input-events` 側の衝突と**同時に**裁くこと。

### 10.3 未走査の座標系隣接（design で吸収・衝突ではない）

- **`completed/areka-P0-surface-resize-resnap`**（main 直近 `9412e467`・スイープ 20 本に**不在**＝最も近い隣接なのに未走査だった）:
  1. `crates/areka/src/emo2_boot/frame.rs:563` が既に `presenter.text_slot_view(shell_target(scope))` を **shell target に対し read-only 消費**（`:545`「balloon_target は読まない」明記）→ R3 は「**`TextSlotView` へ現 surface id を additive 追加**」が §4 論点B の**第4案**として有力（B-1 の新フィールドより既存規律と整合）。
  2. `crates/areka-emo-present/src/presenter.rs:110-112` の `TextSlotView::scale()` doc＝「将来 DPI スケーリング（k＝モニタDPI÷author_dpi）を導入したら、供給値の変更点は**ここ1点**」→ **R4.3「サーフェス px で照合」は k=1.0 への暗黙依存**。design で「k=1.0 限定契約＋再検証トリガ＝この1点」を明記（`crates/areka-emo-text/src/actor.rs:83` が `view.scale()` と `view.surface_size()` を**対で**受ける前例）。
  3. resize-resnap は実行時に窓寸を変える→**現 surface id・surface_size・窓 bounds を同一フレーム／同一適用点で一致**させないと hit がズレる。
- **他の座標正本**: `completed/areka-P0-window-placement`（物理 px 単一通貨）・`completed/event-drag-system`。design 冒頭の確定表に「**物理/論理・原点・scale k**」の3列を作り、この3 spec を典拠として引くこと。

### 10.4 collision の順序不変条件が未約束（画家則の足場・design で固める）

- `crates/areka-emo-compose/src/normalized.rs:74` は `elements` に「**layer 昇順・同 layer は登場順**」と不変条件を宣言。対して `:76` の `collisions` は「当たり判定領域（**転記のまま**）」のみで**順序の約束が無い**。画家則（§9）は「偶然の転記順」に意味論を載せている。
- `fold.rs:121-122` が `surface.append` の collision を**末尾連結**→画家則では **append 由来が base 由来に常に勝つ**（SSP `none` 則なら逆＝base が勝つ）。
- **emo2 では観測不能**: collision は sakura `surfaces.txt:23-24`（surface1000）と kero `:417-418`（`surface.append10,2100-2110,2200-2210`）の2箇所のみ・**kero の base surface(2100〜) に自前 collision 無し**＝重複も重なりも発生しない。檻でも e2e でも露見しない。
- **ukadoc**: `collision*` の ID は「同じ surface 内で**重複しない通し番号**」＝append で index が重複する形は正典外領域。画家則なら後勝ちで決定論縮退する（長所）が**明記が要る**。
- **design で**: (a) `SurfaceMaster.collisions` の順序不変条件（登場順・append は末尾）を **doc レベルで明文化**（additive＝「正規化形の再定義をしない」に抵触せず）、(b) **fold 出力を入力にした檻を1本**（append 由来が勝つことを意図として固定）、(c) 重複 index 時の後勝ちを design 表に記載。

### 10.5 型ドリフト（衝突ではない・design で1本化）

- `HitRegion.scope: usize`（brief）⇔ `target_map.rs:36 scope_of -> Option<u32>`／seriko `ActorKey`＝**型を1本化**すること。
- `HitRegion` に**窓種別（shell/balloon）の識別子が無い**。target 偶奇で互いに素ゆえ実害は結線ミス時のみだが、「リゾルバは shell 専用」を契約に明記するか要判断。ukadoc `OnMouseMove` の Reference に**バルーン識別子は無い**（Ref0/1=ローカル座標・Ref2=ホイール回転量・Ref3=本体0/相方1・Ref4=当たり判定の識別子・Ref5=常に0・Ref6=入力デバイス種別）＝区別は areka 側の結線規律で担保するしかない。

### 10.6 残る blind spot

- **画家則の SSP 逸脱は e2e で永久に検出されない**: emo2 には重なり collision も `collision-sort` 宣言も無い（fixture 実測）。`emo2-conformance-e2e` が主張するのは「SSP 完全適合」ではなく「**emo2 適合**」である旨を、design か e2e brief のどちらかに1行残す（coordination note）。
- **`Hide` 時の現 surface id 意味論**が未定（§6-4）: seriko `output.rs:36 DisplayCommand::Hide{scope}` に対し R3.2「未表示＝無し」をどう倒すか。なお `Show{surface_id,binds}`（`output.rs:28-34`）＝アニメは binds 表現ゆえ **seriko-loop 実装後も base surface id は安定**＝brief:45「読み口契約は不変」は実型で裏取り済み。
- **collision 非保有 surface への切替**: emo2 sakura は **surface1000 のみが collision 保持**＝`\s[1010]` 等へ切り替わると region=None が正しい挙動。R7.2 の檻 (e) は**この実データを典拠にできる**（人工 fixture 不要）。

---

## 11. 設計フェーズ discovery（2026-07-17・kiro-spec-design）

- **Discovery Scope**: Extension（light discovery＝既存 3 crate への増分・新規依存なし・外部 API 調査不要）。
- **Key Findings**（詳細は下記各節）:
  1. **ukadoc は本 spec の中核論点に沈黙している**（含端規則・Ref4 非該当値・ローカル座標の空間・DPI）＝要件 R1.4「design で ukadoc collision 節から確定」は**正典からは確定できない**。設計判断＋根拠＋再検証トリガとして決着させた（design 正典確定表 C2/C7/C9）。
  2. **emo2 `surface0` は collision を1つも持たない**（collision 行はリポジトリ全体で4行のみ）＝probe の表示対象は `surface1000` でなければ空集合しか観測できない。
  3. **`surface1000` は静的 element を持たず全パーツが bind 制御**＝`BindSet::default()` では全透明窓が「成功」として出る。probe は有効 bind 実値と `compose(1000, binds)` 由来の窓寸を要する。
  4. **`emo2_boot/mod.rs` は非テストコードに `crate::` 実呼出を持つ**（`:305`）＝example から `#[path]` include 不能。前例の機構は `include!` ではなく **`#[path]` モジュール宣言**（リポジトリ内に `include!` は0件）。

### 11.1 ukadoc 正典調査（第一手段＝ukadoc MCP・§7 の持ち越し研究項目の決着）

- **Context**: brief/要件が「design 冒頭で collision 節から確定表にする」と指示した4点（含端・優先・Ref4 非該当値・透明画素との関係）の裏取り。
- **Sources Consulted**: `descript_shell_surfaces`（`collision`／`collision-sort`／`collisionex`／`animation*.collision*`／`base`）・`dev_shell`・`list_shiori_event`（`OnMouseMove`／`OnMouseClick`／`OnMouseDoubleClick`）・`descript_shell` `seriko.dpi`。
- **Findings**:
  - **STATED**: 座標の意味（`dev_shell`＝サーフェス画像左上原点・左上/右下）／`collision-sort` の既定 `none`＝「IDによらず先に書かれている方が手前」（ソート対象は collisionID であって名前ではない）／collisionID は surface 内で重複しない通し番号／**名前の重複は明示的に許容**（同じ名前を複数行に分けてよい＝Ref4 は多対一のラベルでキーではない）／1 surface あたり collision 最大 **256 個**／`collisionex` は rect/ellipse/circle/polygon/region（引数順が `collision` と逆＝ID が第2引数）。
  - **SILENT（実地確認・見落としではない）**: ①**含端規則**（幾何記述は「囲まれた範囲」のみ）②**Reference4 の非該当時の値**（全マウスイベントで「当たり判定の識別子」の一文のみ・空文字は里々/YAYA 等の**事実上の慣行**）③**Ref0/1 の「ローカル座標」の定義**（collision 座標空間との同一性は未記述）④**DPI と座標の関係**（`seriko.dpi` は宣言ノブのみで座標意味論なし）⑤**透明画素と collision の関係**（正典は透過色と「窓としてのクリック可否」しか述べない）。
  - **追加発見（M1 外だが契約に効く）**: `base` 描画メソッドは「collision もコマのサーフェスに定義されたものに**更新される**」／`animation*.collision*` は「アニメーション動作中限定」の collision を持つ ⇒ **collision 集合は将来 base surface id だけでは決まらなくなる**。
- **Implications**: 沈黙4点は「正典から確定」できないため、設計判断として決着し**根拠と単一変更点（再検証トリガ）を明記**する方式へ切替（design 正典確定表）。`base`/`animation*.collision*` は Revalidation Trigger 5＋Coordination Note C-6 として seriko-loop/mayuna-compose へ申し送り。

### 11.2 コードベース実測（ドリフト検証・§2 の file:line 主張の再突合）

- **Context**: 記憶「並走briefは陳腐化する・設計前にrebase」に従い、gap 分析の主張を settled コードへ再突合。
- **Findings**: `git log HEAD..origin/main` ＝ roadmap 追記㉚（#67）の**1件のみ・crates 差分ゼロ**＝実装面のドリフトなし。§2 の主張は A1–A7/B8/B10–B12/C13/C15–C17/E21/F23 が **CONFIRMED**。修正を要した3点:
  - **§4 論点 B「B-9 相当」= NOT FOUND の確認**: `EmoPresenter` の公開面は `new`/`attach_target`/`apply`/`text_slot_view`/`read_back` の5つのみ＝現サーフェス id の読み口は**真の欠落**（新シーム必須）。
  - **§10.2 実装注記の機構名が誤り**: 前例は `include!` ではなく **`#[path]` モジュール宣言**（`window-placement.rs:99-113`）。成立条件＝対象モジュールの非テストコードが `super::`／外部 crate のみを参照すること。
  - **§10.3 の acceptance-record 前例は1件のみ**: `completed/areka-P0-window-placement/acceptance-record.md` のみ実在（`surface-resize-resnap` には無い）。
  - **`EmoWorld::collision_sort()` が world 面に既存**（`world.rs:150-152`）＝gap 分析は「`SurfaceMaster` へ非伝播」を指摘したが、**world 面には露出済み**。将来 reconcile 時にデータ配管は不要（`RegionPriority` へ variant を足せば届く）。
- **Implications**: design の Revalidation Trigger 4 と Coordination Note の根拠。

### 11.3 Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | 採否 |
|--------|-------------|-----------|---------------------|------|
| **A-1 emo-compose に新規純関数** | `SurfaceMaster.collisions` を直接照合する純関数 | wintf 非依存憲章と整合／i64 整数厳密／GPU 不要の全網羅 unit／画家則の定義元（`blit.rs:83`）と同居 | emo-compose の責務が「合成」から「hit 解決」へ拡張 | **採択** |
| A-2 wintf `HitRegionMap` 再利用 | `Collision`→`Shape::Rect` へ適合し `hit_test_region` を呼ぶ | 多角形/カラーマップを無償で獲得 | **f32 DIP＋正規化座標**へ変換必須＝整数厳密性が崩れる／**定義順先勝ち＝画家則と逆**（`hit_region:356`）／emo-compose→wintf の依存方向違反 | 棄却 |
| A-3 新規モジュール/薄クレート | hit 解決専用の新設 | 責務が最も明確 | crate 増／M1 に不要な構造 | 棄却 |

### 11.4 Design Decisions

#### Decision: 含端規則＝閉区間（両端含む）
- **Context**: R1.4 は「正典含端規則に従う」と要求するが、**正典は沈黙**（11.1）。
- **Alternatives**: (1) 閉区間 (2) 右下排他（半開区間） (3) 要件へ差し戻す。
- **Selected**: **閉区間**（`left <= x <= right && top <= y <= bottom`）。
- **Rationale**: 正典が沈黙ゆえリポジトリ内前例へ揃える＝wintf `hit_region/mod.rs:365-368` が両端含む。「囲まれた範囲」に除外の明示が無い読みとも整合。(3) は R1.4 の規範内容（＝「単一規則で決定論的かつ一貫」）が既に満たせるため過剰。
- **Trade-offs**: 正典の裏付けが無い＝SSP 実挙動と乖離する可能性が境界1px に残る。
- **Follow-up**: 変更点を純関数の比較式1箇所に閉じ、境界檻（4辺4隅）で固定（Revalidation Trigger 3）。

#### Decision: `HitRegion` の定義箇所＝areka bin `emo2_boot/hit_region.rs`
- **Context**: 5.4 の正本性と、bin crate は型を輸出できない制約の両立。
- **Alternatives**: (i) emo-present（「指令 API 契約正本」の前例あり） (ii) emo-compose (iii) areka bin 結線層。
- **Selected**: **(iii)**。
- **Rationale**: `scope` は結線層の概念（`target_map` が正本を持つ層）であり、emo-present は `TargetId` しか知らない＝(i) は下流固有概念を上流へ漏らす。第一消費者 input-events の消費点も **areka bin の結線層**（brief:33「UI 配線（emo2_boot 結線層）: 窓ハンドラ→resolver→`KanadeMsg` 送出」・kanade が受けるのは素のフィールド）＝lib 境界を跨ぐ必要がない。
- **Trade-offs**: bin crate ゆえ外部から `use` 不能。将来 kanade 側が型を要する場合は lib crate へ昇格（契約の形は不変）。
- **Follow-up**: Revalidation Trigger 1・Coordination Note C-1。

#### Decision: 現サーフェス読み口＝`PresentTarget` の private フィールド＋accessor（B-1）
- **Context**: R3 と「emo-present 本体無改変（additive）」の両立。
- **Alternatives**: B-1（新フィールド）／B-2（`ComposeKey` 由来）／B-3（wiring 層で記録）／第4案（`TextSlotView` へ追加）。
- **Selected**: **B-1**＋専用 accessor 2本（`current_surface_id`／`hit_region`）。
- **Rationale**: B-2 は `invalidate_all` でキーが消える一方で表示は残る＝画面と乖離（実測で棄却根拠を確認）。B-3 は送信側記録と適用結果の乖離（失敗時）＝二重真実源。第4案は text 専用 view への意味論過積載。**書き込み3点が既存の `visible` 更新点と同一**（`:352`/`:237`/`:382`）で、全失敗経路が `:352` より手前で early return するため「失敗＝前値保持」が**分岐追加ゼロで成立**する＝3.4 と綺麗に噛み合う。
- **Follow-up**: Hide→`None`／`EmptyComposition`→`None`／`attach_target` 再登録→`None` を doc と檻で固定。

#### Decision: 重なり優先の型シーム＝`RegionPriority`（引数化・`_` アーム禁止）
- **Context**: R2.3 の「型シーム予約」と design-synthesis「投機的抽象の排除」の緊張。
- **Selected**: 単一 variant `Painter` の enum を**純関数の引数**に取り、内部 match に `_` を置かない。
- **Rationale**: 要件が明示要求する構造であり投機ではない。単一 variant でも**引数化により署名がシームになる**＝variant 追加時にコンパイルエラーで漏れを検出。`#[non_exhaustive]` は定義 crate 内では効かない＝検出機序は網羅 match の方である（design に明記）。
- **Trade-offs**: 呼び手が `RegionPriority::Painter` を明示する冗長性。

#### Decision: `collisionex` の形状シームは新設しない
- **Context**: 要件 Out of scope の「型シームのみ」／brief「矩形 enum の余地」。
- **Selected**: **形状 enum を作らない**。純関数の署名 `hit_region(&SurfaceMaster, x, y, RegionPriority) -> Option<&str>` が**形状に言及しない**＝既にインターフェース側で余地が開いている。
- **Rationale**: design-synthesis「インターフェースを一般化し、実装は一般化しない」。形状語彙は upstream parsers の転記型が持つべきもの＝本 spec が先取りすると (a) 正規化形の再定義（要件が禁止）か (b) 単一 variant の投機的抽象、のいずれかになる。

#### Decision: probe は donor（`emo-present.rs`）から3点で逸脱する
- **Context**: 7.3 の証跡取得を donor 流用で済ませられるかの検証（**敵対的レビューで2件の BLOCKER を検出**）。
- **Findings（実測）**: ①`surface0`=434×687 vs `surface1000` extent=382×547（`compute_extent` は bind 非依存＝`plan.rs:366-383`）／donor は `compose(..., 0, default)` の寸を `WindowPos.size` へ渡す（`emo-present.rs:375-376,514-516`）一方 `chain.upload` は composed 寸へ resize（`chain.rs:173-193`）⇒ **k=1.0 assert が構成上必ず失敗**。②`surface1000` は静的 element ゼロ・全 bind 制御（`surfaces.txt:17-19`）⇒ `BindSet::default()` では ops 空・extent 非ゼロゆえ `EmptyComposition`（0×0 のときのみ＝`plan.rs:448-455`）に該当せず**全透明窓が「成功」として表示**（`visible=true`・info ログ）。
- **Selected**: probe の Input 契約で **表示 id=1000／有効 bind 実値（donor `:170` の `from_ids([1101,1206,1302,1502,1800])` 相当）／窓寸=`compose(1000, binds)` の extent** を固定する。
- **Follow-up**: design Batch 契約 Input 1.–3.／Open Risk 6。

#### Decision: probe の反トートロジー条件＝狙点を目視（または read_back 由来）に限る
- **Context**: 7.3(a) は「collision 値から合成した点の直接注入は証跡と認めない」と要求。
- **Findings**: `ClientToScreen`／`ScreenToClient` は client 原点の**厳密な逆写像**⇒ collision 由来の点を `SetCursorPos` で撃って `GetCursorPos`→`ScreenToClient` で読み戻すと **p がそのまま返る**＝「文面上は client 経路・実質は自己注入」。各検査の欠陥クラス: **k=1.0 assert**＝OS 真実（`GetClientRect`）と emo 真実（`chain.size()`）の独立2源突合せ＝論理 px 窓の欠陥を捕捉（原点ずれ・マウス経路スケーリングは取り逃がす）／**read_back anchor**＝collision 値と実描画画素の対応を捕捉（マウス非依存）／**目視**＝本命（collision と見た目の対応）／**マウス経路（`WM_MOUSEMOVE` lParam）のスケーリング**＝3者とも取り逃がす＝合流サインオフへ帰属（C-4・Open Risk 3）。
- **Selected**: 狙点は**目視**（または read_back 由来の描画由来点）に限り、collision 値から合成した screen 座標への `SetCursorPos`/`SendInput` を**証跡として禁止**。read_back による描画一致 anchor を自動検査として新設（プロトコル 4.）。

### 11.5 Synthesis Outcomes

- **Generalization**: 純関数の署名を形状非依存（正規化形→領域名）に保つことで、M2 `collisionex` を**インターフェース変更なし**に受けられる＝「インターフェースを一般化し実装はしない」。
- **Build vs Adopt**: wintf `HitRegionMap`（既存の名前付き領域解決）を**棄却**＝座標系（f32 DIP 正規化 vs i64 物理 px）・依存方向（emo-compose は wintf 非依存が憲章）・優先規則（先勝ち＝画家則と逆）の3点で不一致。「既に解かれた問題」に見えるが**別の問題**である。
- **Simplification**: 失敗経路・エラー型・ログ・索引構造・形状 enum・`collision_sort` 配管を**すべて作らない**。全ての「解決できない」は `None`（正常結果）。走査は最大 256 矩形（正典上限）の線形逆順＝索引不要。

### 11.6 敵対的レビュー（design 生成中・独立エージェント）

7軸で design 草案を実コードへ全数突合。**純粋層側（`#[path]` スキーム・additive 書き込み点・bin crate 配置・R3.4 適合）は全て「設計どおり正しい」と実測で確認**され、**破綻は probe に集中**（BLOCKER 2件・MAJOR 2件）。全件を design へ反映済み:

| # | 重大度 | 指摘 | 反映先 |
|---|--------|------|--------|
| 1 | BLOCKER | donor 流用だと k=1.0 assert が必ず失敗（434×687 vs 382×547） | Batch Input 3.／Open Risk 6 |
| 2 | BLOCKER | `surface1000` は bind 無しだと全透明＝狙う絵が出ない | Batch Input 2. |
| 3 | MAJOR | 狙点が collision 由来なら OS 往復はトートロジー | プロトコル 4./5.＋禁止条項 |
| 4 | MAJOR | W2 が `spawn.rs` へ resolver 呼出を足すと `window-placement.rs` の `#[path]` include が壊れる（completed 成果物の破壊） | **Coordination Note C-8** |
| 5 | MINOR | `#[non_exhaustive]` は定義 crate 内で無効＝検出機序は網羅 match | `RegionPriority` doc／テスト#10 を実装制約へ格下げ |
| 6 | MINOR | 不変条件「画面に出ているもの」が R6.1（α 非参照）と矛盾／典拠行ずれ／`attach_target` 再登録 | 状態遷移 Key decisions／Preconditions |

### 11.7 Risks & Mitigations（design 反映済み）

- 含端規則が正典の裏付けを持たない — 変更点を1箇所に閉じ境界檻で固定（Trigger 3）。
- 画家則の SSP 逸脱は emo2 では**永久に検出不能**（重なり collision も `collision-sort` 宣言も fixture に無い） — e2e の主張範囲を「emo2 適合」と明記（C-5）。
- probe の証拠力の中核が目視 — 本番ゴースト先行の原則の必然（機械生成した狙点はトートロジー）。自動化可能な2検査（k=1.0・read_back）で周囲を固める。
- W2 の `crate::` 罠 — C-8 で事前申し送り（顕在化は W2 が壊してから）。

### 11.8 Follow-up（tasks フェーズ以降）

- **roadmap.md:210 の義務チェックポイント**: 先行 spec は cue-playback マージ後に `/kiro-validate-design` を settled コードへ再実行してから tasks へ進む。本 spec の design は **cue-playback マージ後の main（`653ae3ea` 時点）へ実測突合せ済み**（11.2＝crates 差分ゼロを確認）であり、かつ roadmap:207 が「cue-playback は emo-present 不触」と明言するためドリフト risk は低いが、義務は無条件ゆえ tasks 前に再実行すること。
- 実装時、`hit_region.rs` の `crate::` パス禁止規律をファイル冒頭 doc に明記（機械的強制がないため）。

## 12. 設計ディスカッション決定（2026-07-17）

### 12.1 議題1: リゾルバ到達性（design-validation Critical Issue 1）＝実装繰延＋証明焼き込み

- 4並列敵対検証（dispatch 交錯／モジュールグラフ／シムのテスト成立性／台帳スティールマン）の結果、**技術面の障害はゼロ**: ポインタハンドラは排他 system（Input schedule・`dispatch/mod.rs:209-253`）＝`emo2_frame_system`（FrameFinalize）と同一 tick 内で完全直列・remove 窓との交錯は構造的に不可能（wndproc 再入は try_borrow スキップ＋`IS_TICK_FLUSH_IN_PROGRESS`＋SetWindowPos 遅延 flush の三重防御）。`client_point` は窓 client 物理 px・`CharWindowMarker{scope}` で entity→scope O(1)。
- 一方**戦略面のスティールマンが繰延優位を立証**: ①ポインタ配線は roadmap 台帳（:183）が W2 割当 ②保持の方式と粒度（per-scope／per-window）は input-events design の予約事項（同 requirements:30） ③W1 内のシムは呼び手ゼロの dead code（存在チェック不能） ④C-7 堅持で W1「共有ファイル0」が sibling（dialogue-tags の Move 配線未釘留め）に頑健。
- **決定**: アクセサ＋ハンドラの実装は W2。線引き＝「呼び手が spec 内に居るもの＝実物・次ウェーブに居るもの＝地図」。design へ (a) Resolver 節「リゾルバ到達性」（検証事実の台帳）(b) C-8 を推奨形（emo2_boot 側で装着）へ格上げ (c) C-9 新設（presenter private・アクセサは W2 の作業・参考ハンドラ形）(d) Revalidation Trigger 8（tick 固定順への依存）を焼き込み。
- 副産物の実測: `adapter.rs:16` は `crate::emo2_boot::target_map` 形ゆえ include 不能の先例（`hit_region.rs` は必ず `super::target_map` 形で書く）。placement 全7ファイルの `crate::` 出現13箇所は全て `#[cfg(test)]` 内（C-8 前提の brace 実測確認）。

### 12.2 議題2: probe の証拠力（design-validation Critical Issue 2）＝本番窓寸規則の駆動へ改稿（検証により確定・議題撤回）

- 敵対検証2本の帰結: (1) 旧主張「独立な2源」は実は**2経路1値**——extent 事前計算と `chain.size()` は同一関数 `compute_extent` の出力（`plan.rs:444,366`・`chain.rs:173-194`）＝事前計算に固有の証拠価値なし。(2) 旧形は**resize 経路の DPI 混入**（window-placement v1 を廃案にした `wire_drag` dpi/96 再スケールと同じ実在欠陥クラス）を完全に取り逃がす。(3) `resize_window_to` は pub（`follow.rs:551`）で placement `#[path]` include（`window-placement.rs:107-113` 前例）から呼べる。(4) completed `surface-resize-resnap` の実機証跡は目視・単一 DPI(125%)・数値記録なし（acceptance-record.md 不存在）＝**本番 resize 経路の数値 k=1.0 実測は本 probe が初**＝option 1（文面のみ）のスティールマンは証拠粒度で崩壊。
- 決定: probe＝「placeholder 誤寸で `spawn_ghost_windows`（Anchored 付与）→ ShowSurface(1000,実binds) → `surface_size()` を本番 `resize_window_to` で適用 → **次フレーム**に実窓 `GetClientRect` assert」。檻の注意: `WindowPos.size` は enqueue 時 bypass ミラー（`follow.rs:760-767`）ゆえ WindowPos での assert は偽緑。C-7 維持（placement include は read-only・編集ゼロ）。

### 12.3 議題3: マウス経路の空間照合（design-validation Critical Issue 3）＝プロトコル5へ追加（検証により確定・議題撤回）

- 検証帰結: probe 窓への `OnPointerMoved` 配送は donor 構造のまま成立（hit test 既定 Bounds＋surface 子 entity の alpha_mask ヒット・`dispatch_pointer_events` は `PointerState` 存続中**毎 tick 発火**＝静止サンプル取得可・`dispatch/mod.rs:221-229`）。装着は spawn バンドル1行（`OnPointerPressed` 前例＝`emo-present.rs:523`）。
- 形式: インライン hard assert は不可（キュー配送 coalesce＋クリック透過 poll 12ms の過渡 race）→ **記録行ペア列（client_x/y・s2c_x/y・Δ）＋静止500ms＋Δ=(0,0) 厳密一致＋1px 再静止1回**。ペア列は不透明画素行（Head/Bust）のみ＝背景 None 行の欠測はクリック透過の正しい挙動（C-3 整合）。
- 捕捉クラス: per-monitor v2 の awareness 経路間不一致・wintf 配管の f32→i32 歪み・HWND 取り違え・狙点での本番イベント配送の実在証明。C-4/Open Risk 3 を「空間一致は本 spec 確認済み・合流サインオフ残余は配送〜SHIORI 一周」へ精緻化。

### 12.4 議題4: 含端規則の SSP 実挙動突合＝机上不能と確定・閉区間維持（検証により確定・議題撤回）

- 実査: **SSP ソースは非公開**（作者 GitHub 全14リポ・ukatech org 全18リポ・公式サイト・コミュニティ一覧2件・GitHub 全域コード検索の5系統が独立一致・2026-07-17）。「SSP はオープンソース化された」は**フォークロア**（ssp-i18n 等の周辺リポジトリの誤認）。互換クローン ninix-kagari は crossing-number 法（閉/半開いずれとも不一致の非対称境界）で先例にならない。
- 帰結: Revalidation Trigger 3 の発火条件（SSP 実挙動の排他境界判明）は机上で到達不能＝**閉区間（wintf 前例）が立ったまま・設計変更なし**。将来の唯一の実証経路は実 SSP バイナリへの境界1pxブラックボックス突合（1px 境界プローブ用ゴーストで OnMouseMove Reference4 を観測・撫でクラスタ合流サインオフと同席が自然）。Open Risk 1 へ実査結果を焼き込み。

## 13. DPI追従下の当たり判定＝Task 4.2 受け入れ却下と新 spec 分割（2026-07-18・引き継ぎ）

> **この節は別セッションへの引き継ぎ記録**（開発者指示 2026-07-18「開発は別セッションで行う・課題が漏れなく引き継がれるよう網羅」）。実装成果物 Task 1-4.1 は完成・コミット済みだが、Task 4.2（実 DPI 受け入れ）は**却下**され、DPI追従下の当たり判定は新 spec 2 本へ分割した。

### 13.1 経緯: 実 DPI 受け入れ却下

`collision-probe` を実 DPI≠96 の2水準（DELL S3221QS 3840×2160 **dpi=120/125%** primary ＋ 副 2880×1800 **dpi=192/200%**）で実走し、②③④自動 assert（surface1000=382×547・k=1.0・read_back Head/Bust 中心不透明）＋⑤目視解決（Head/None/Bust・静止 Δ=(0,0)）＋192 の外部 `GetClientRect`=382×547 を実測（`acceptance-record.md` に証跡保持）。**しかし開発者が却下**——理由:

- areka の**基本設計は DPI追従**（画面 DPI に追従してマスコット拡大縮小・SSP の固定px等倍とは別思想）。k=1.0（非拡大）は**現実装の途中状態であって設計目標ではない**（[[areka-dpi-following-core-design]]）。
- 現状 `TextSlotView::scale()` は常に 1.0 ゆえ、**両 DPI 水準ともマスコットは同一物理寸（382×547）＝ヒットテストの座標経路が完全に同一**。「モニタ DPI を2水準変える」は k=1.0 plumbing の DPI-clean 性（dpi/96 誤再スケールが無いこと）を確認したにとどまり、**DPI追従の核心＝「マスコットが scale≠1.0 で拡大表示された状態で、拡大後の窓 client 点を k で縮約（÷k）して当たり判定が正しいか」は未実装かつ未検証**。
- ＝「拡大率設定が異なるときに正しくヒットテストされるか」が全く未検証。**これでは受け入れできない**（開発者言明）。

### 13.2 配管調査（2026-07-18・7エージェント workflow・実測 file:line）

- emo 層は **k=1.0 がコンパイル時定数でハードワイヤ**: `CURRENT_COMPOSE_SCALE: f32 = 1.0`（`presenter.rs:126`）→`TextSlotView.scale`（`:427`）。`hit_region`（`:449-453`）は点を無変換で純関数へ（÷k なし）。純 `hit.rs:57-62` は scale/k を取らない。`compute_extent`（`plan.rs:366-383`）・`blit.rs:69-163`（1:1 整数コピー・リサンプラ不在）。下流寸法（swapchain/visual/窓）は composed extent 従属。
- **per-window DPI は入手可能・未消費**: wintf `DPI` component（`dpi.rs:21-28`・`GetDpiForWindow` `window_handle.rs:223-238`・`WM_DPICHANGED` `window_pos.rs:285-343`・re-export `ecs/mod.rs:46`）。presenter は `window: Entity`＋`&mut World` を持つが `world.get::<DPI>()` を読んでいない。単一変更点＝`TextSlotView::scale()`。
- **wintf は既に k≠1.0 レンダリング実績あり**（`taffy_systems.rs:214-225`→`arrangement.rs:196-234`→`render.rs:95-111` `SetTransform`）。emo-present は意図的バイパス（`mount.rs:60-72`）＝greenfield でない。
- **必要なピース（end-to-end）**: (a) k× でレンダ〔emo-compose A案リサンプラ or emo-present B案 transform〕 (b) `scale()` が k を返す〔emo-present・単一変更点〕 (c) `hit_region` が点÷k〔本 collision 系・caller 境界〕 (d) 窓/swapchain = k×surface〔A案は自動追従〕。
- **正直な限界**: (c) の point÷k＋fake-k 決定論 unit は今・GPU 不要で書けるが、実機 7.3 は render が実際に scale しないと満たせない（÷k を no-op としてしか観測できず正誤を実機で判別不能）。fake-k 注入 unit は 7.3 の「合成点注入は証跡と認めない」に同型ゆえ回帰檻には足るが実機証跡には数えない。＝point÷k 単独は**必要だが不十分**。

### 13.3 分割決定（新 spec 2 本・brief 済 2026-07-18）

- **`areka-P0-emo-dpi-scaling`**（⑥ emo・render 基盤・broad）: emo が surface を k=monitorDPI÷author_dpi で実拡大レンダ・`scale()` が k を返す・窓/swapchain 追従。wintf `DPI` を consume。全 emo 消費者が波及。
- **`areka-P0-collision-dpi-hittest`**（⑥ emo・collision 後続・**`emo-dpi-scaling` に依存**）: `EmoPresenter::hit_region` の point÷k＋fake-k 決定論 unit＋本 spec の k=1.0 契約改訂＋scale≠1.0 実機受け入れ。純 `hit.rs` 不変。
- 両者とも本 spec（`collision-geometry`）の純関数・resolver・probe を土台に使う。DPI追従波及の既存消費者（`window-placement` 窓寸・`emo-text-layer` 行寸・balloon・`choice-render`）は Revalidation Trigger。

### 13.4 本 spec（collision-geometry）の状態

- **Task 1-4.1 完成・コミット済み**（k=1.0 純関数 `hit_region`＋`RegionPriority`／`current_surface_id` 読み口／`HitRegion` 契約＋`resolve_hit_region`／collision-probe）。これらは DPI追従でも**土台として不変**（÷k は caller 境界で吸収・純核は DPI 非参照のまま）。`HitRegion` 契約は `input-events`（W2）が今すぐ必要とする実物。
- **Task 4.2 は却下＝未完了**（tasks.md で `[ ]`＋`_Blocked_`・`acceptance-record.md` 冒頭に REJECTED 明示）。k=1.0 plumbing の実測値は有効ゆえ保持。DPI追従 hit-test 受け入れは `collision-dpi-hittest` へ移管。
- design の k=1.0 限定契約（`design.md:50`）と Revalidation Trigger 2（`:86`）は「将来 optional」の書き方だが、DPI追従を基本設計とみなすと**この限定契約自体が要見直し**＝`collision-dpi-hittest` が改訂する。

### 13.5 別セッションで決める未決事項（再開トリガ・[[portfolio-convergence-decided-in-separate-session]]）

1. **M1/M2 配置**: DPI追従は基本設計だが emo2 は k=1.0 でも E2E 実走する。M1 blocker（M-life に組込）か M2 送りか。
2. **collision-geometry の合流/マージ**: Task 1-4.1（k=1.0 resolver＋`HitRegion` 契約）を暫定確定として先に merge（DPI追従 hit-test を新 spec で追跡）するか、`collision-dpi-hittest` 着地まで開けておくか。input-events（W2）が `HitRegion` を必要とする点が判断材料。
3. **Strategy A（emo-compose 鮮明ラスタ・`blit.rs` リサンプラ）vs B（WUC transform・軟い/低コスト）**・**author_dpi 定義**（k の分母・ukadoc 正典）・**整数倍/連続**・**÷k 丸め**（floor/round・境界1px）・**WM_DPICHANGED ライブ再スケール**・**seriko/mayuna collision との相互作用**（各 brief の open questions）。

> 正本参照: 新 spec 2 本の brief（`specs/completed/areka-P0-emo-dpi-scaling/brief.md`・`specs/areka-P0-collision-dpi-hittest/brief.md`）／roadmap 追記㉚／[[areka-dpi-following-core-design]]。
