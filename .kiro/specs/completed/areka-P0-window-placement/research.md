# ギャップ分析: areka-P0-window-placement

> 生成: 2026-07-09（kiro-validate-gap）／対象: 確定済み requirements.md（7 要件）＋ brief.md ＋実コードベース。
> 目的: 確定要件と既存コードの差分を洗い、設計フェーズの実装戦略・設計判断・調査項目を提示する（決定は下さない）。

## 分析サマリ（3–5 点）

- **窓生成の「呼び方」は 2 つの実 donor が揃っている**が、「構成から N 窓を生やす／既定位置へ置く」機構は**皆無**。`crates/areka/examples/emo-present.rs`（本番ゴースト実表示・物理 px 直渡し・第一級 donor）と `crates/areka/examples/mock-shell.rs`（ドラッグ＋バルーン追従 donor）が窓生成 API の呼び方を提供する。差し込み先は `crates/areka/src/main.rs` の `open_startup_window(&WinApp)` シーム（現在はダミー窓を開くのみ）。
- **座標契約に必要な wintf 部品はすべて実在し、brief の記述と一致**する。`Monitor.work_area`/`bounds`（物理 px・RECT）・`WindowPos.position`（物理 px・`Point{i32}`）・`BoxStyle`（論理 DIP・taffy）・`DraggingState`/`DragConstraint`（物理 px 明記）・`DPI` Component（`to_logical_*`／`to_physical_*`／`scale_x/y`・half-away-from-zero 丸め）を確認。2026-07-05 リジェクトの再発防止は「型で単位を混ぜない」設計判断に落とせる。
- **既定位置ロジック（`seriko.alignmenttodesktop` カスケード・work area 計算・スコープ別配置）は 0 から新設**。emo2 fixture の実測値（`seriko.alignmenttodesktop,bottom`／`sakura.defaultx,0`／`kero.defaultx,0`・いずれも shell descript）を確認。純粋 resolver（KV＋work_area＋DPI→物理 px 座標）として切り出せば DPI パラメタ化単体テストが成立する（R3.4）。
- **構成入力の到達（plumbing）に設計上の穴がある**。placement は「shell dir＋descript KV」を要するが、`open_startup_window(&app)` は `&WinApp` しか受けず、mount 済みの `MountModel` は `GhostRuntime.mount`（private・`#[allow(dead_code)]`）に隠れ、かつ `MountModel` は生 KV も scope 数も持たない（抽出済みフィールドのみ）。placement は emo-present と同様に descript.txt を独自に読み直す前提になる公算が高い（設計判断）。
- **実 DPI（≠96）検証は headless 不能ゆえ手動観測が必達**。emo-present の rustdoc に手順が既に蓄積済み（150%/200% で表示等倍・座標一致を目視）。純粋 resolver 側で DPI パラメタ化テスト、実 DPI は example の手動記録、という 2 層構成が既存パターン。

## 現状調査（既存資産・パターン）

### 窓生成・配置の donor と差し込み先

| 資産 | 所在 | 役割 | 持ち込み可否 |
| --- | --- | --- | --- |
| `open_startup_window(app: &WinApp)` | `crates/areka/src/main.rs:411` | replace-me シーム（現状ダミー窓 spawn＋smoke 自動 close ゲート） | **差し込み先本体**。署名は `&WinApp`（`world().borrow().spawn` 経由で ECS コマンド投函） |
| `create_shell_window` / `create_balloon_window` | `crates/areka/examples/emo-present.rs:496/529` | 窓生成の実形（`Window`＋`WindowStyle{WS_POPUP\|WS_VISIBLE, LAYERED\|TOOLWINDOW\|TOPMOST}`＋`WindowPos{position, size=物理px}`＋`HitTest::none()`） | **API の呼び方のみ**。`TOPMOST`・`SHELL_INITIAL_X/Y=400/200`・`compute_balloon_pos` の固定基準は donor 対象外 |
| `on_shell_drag` | `crates/areka/examples/mock-shell.rs:375` | 全面ドラッグ時のバルーン追従（`WindowPos` 読み→`SetWindowPosCommand::enqueue`） | **追従機構の呼び方のみ**。`BALLOON_OFFSET_X/Y=335/0` の固定 offset 値は持ち込み禁止 |
| `register_click_through_windows` | 両 example | `Added<WindowHandle>` で αマスク機構へ窓登録 | 参考（本ユニットの直接責務ではないが多窓化で要考慮） |
| `DummyWindowMarker` / `spawn_dummy_window` | `crates/areka/src/main.rs:191/307` | 現行ダミー窓（配置・座標を一切主張しない） | 置換対象（本ユニットが本物窓生成へ差し替え） |

### wintf 座標契約（実装前確定事項・全部品を実在確認）

- `Monitor`（`crates/wintf/src/ecs/window/monitor.rs:68`）: `bounds`/`work_area` は `RECT`＝**画面座標系ピクセル（物理）**、`dpi: u32`、`is_primary`。`enumerate_monitors()` で全モニタ列挙。`physical_size()`/`top_left()` あり。`work_area` はタスクバー除外（`rcWork`）。
- `WindowPos`（`crates/wintf/src/ecs/window/window_pos.rs` — `components.rs:21` が `super::window_pos::WindowPos` を re-export）: `position: Option<Point{x,y: i32}>`＝**物理 px**（`SetWindowPos` 直渡し・`to_window_coords_for_creation` で生成時算出）、`size: Option<SizeI>`＝物理 px。`on_window_add` フックが未挿入時 `WindowPos::default()`（位置＝CW_USEDEFAULT）を自動挿入。
- `BoxStyle`（`crates/wintf/src/ecs/layout/box_style.rs:136`）: taffy 論理レイアウト＝**論理 DIP**。`Dimension::Px` は DIP。emo-present は DPI 事故回避のため **taffy 非経由**で `WindowPos.size` に物理 px 直渡し（DPI 表示契約）。
- `DraggingState`（`crates/wintf/src/ecs/drag/mod.rs:72`）: `drag_start_pos: PhysicalPoint`＝**物理 px**、`initial_inset: (f32,f32)`＝物理 px（doc 明記）。`DragConstraint`（同 :84）: `min_x/max_x/min_y/max_y: Option<i32>`＝**物理 px**、`apply(x,y)` でクランプ。`DragConfig`（:35）: `threshold=5`（物理 px）・`move_window=true`（wndproc が `SetWindowPos` 直呼び）。
- `DPI` Component（`crates/wintf/src/ecs/window/dpi.rs:23`）: `dpi_x/dpi_y: u16`（既定 96）。`scale_x/y()`、`to_logical_x/y/size/point`、`to_physical_x/y/size/point`（`.round()`＝half-away-from-zero）を提供。**Window entity 専用（SparseSet）**で `GetDpiForWindow`／`GetDpiForSystem` から populate。Monitor.dpi（u32）とは別物。

### 窓生成の内部機構（不改変・wintf 所有）

- `EcsWindowFactory`（`crates/wintf/src/runtime/window_factory.rs:75`）は **`pub(crate)`**。エントリ `create_window`（:99）も **`pub(crate)`**。消費側（areka）は**直接呼べない**——`Window`/`WindowStyle`/`WindowPos` entity を spawn し、`create_windows` system（`window_system.rs:25`）が拾って Win32 窓を生成する（ex_style 自動計算＝`compute_ex_style` が `WS_EX_LAYERED` を剥がし `WS_EX_NOREDIRECTIONBITMAP` 付与・WUC 合成固定）。両 example もこの entity-spawn 経路で動く。

### 構成入力の到達経路

- `main()` は `resolve_config_inputs(args)`→`ConfigInputs{ghost_root, balloon_root}` を持ち、`areka_ghost::boot(ghost_options)`→`GhostRuntime` を得る（`WinApp::new` の前）。
- `GhostRuntime.mount: MountModel`（`crates/areka-ghost/src/runtime.rs:99`）は private・`#[allow(dead_code)]`＝現状**読み出し口なし**。
- `MountModel`（`crates/areka-parsers/src/package/model.rs:29`）は `names`（sakura/kero name）・`shiori.dir`・`shell.dir`・`bindgroups` のみ保持。**生の descript KV も scope 数も持たない**。placement が要する `seriko.alignmenttodesktop`／`sakura.defaultx` 等は `shell.dir`（＝`ghost_root/shell/master`）配下の `descript.txt` を `areka_parsers::kv::parse_kv` で**独自に読み直す**必要がある（emo-present の `read_balloon_offset` が同型の前例）。

### emo2 fixture 実測（design の検証行に使う）

`crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/descript.txt`:
- `seriko.alignmenttodesktop,bottom`（既定と一致）
- `sakura.defaultx,0` / `kero.defaultx,0`（`defaulty`/`defaulttop` は無し）
- `sakura.balloon.alignment,left` / `kero.balloon.alignment,right`（バルーン配置系の後続向け・本ユニットは記録のみ）
- ghost descript には placement 系キー無し（＝4 層カスケードのうち emo2 は「shell 全体」＋「shell スコープ別」のみ実際に触れる）。

## 実シンボル突合（brief.md ↔ 実コード）— ドリフト所見

| brief.md の記述 | 実コード | 判定 |
| --- | --- | --- |
| `EcsWindowFactory::create_window`（窓生成 API） | `pub(crate) struct EcsWindowFactory` ／ `pub(crate) fn create_window`（`window_factory.rs:75/99`） | **軽微ドリフト**。消費側 API ではなく wintf 内部。areka からは呼べず、実際の窓生成は「entity spawn → `create_windows` system」。効果の記述は正しいが「呼び出す API」ではない点を design で明確化 |
| `Monitor.work_area`/`bounds`（物理 px） | `monitor.rs:68`・RECT・docstring「画面座標系ピクセル」 | **一致** |
| `WindowPos`（物理 px・SetWindowPos 直渡し） | `window_pos.rs`（re-export）・emo-present/mock-shell が物理 px 直渡し | **一致** |
| `BoxStyle` Px（論理 DIP・taffy） | `box_style.rs:136`・taffy 論理 | **一致** |
| `DraggingState`/`DragConstraint`（物理 px） | `drag/mod.rs:72/84`・doc 明記 | **一致** |
| `DPI` Component（`window/dpi.rs`・`to_logical_x/y/size/point`・`to_physical_*`・`scale_x/y`） | `dpi.rs:23`・全メソッド実在・half-away-from-zero | **一致** |
| `EmoPresenter::attach_target(&mut self, world, target, window, emo_world, atlas)` | `presenter.rs:100`・シグネチャ完全一致（`_world: &mut World` は現状未参照だが API 一貫性のため受ける） | **一致** |
| `create_shell_window`/`create_balloon_window` donor | emo-present.rs:496/529 実在 | **一致** |
| `on_shell_drag` donor | mock-shell.rs:375 実在 | **一致** |
| `open_startup_window` シーム | main.rs:411 実在（`&WinApp`） | **一致** |
| ghost-setup が boot/shutdown を結線・ダミー窓不関与 | main.rs:238/281（`boot`/`shutdown`）・`spawn_dummy_window` に ghost 不関与 | **一致** |

**追加ドリフト（brief 未言及・design で埋める）**:
1. **scope キーの命名揺れ**: brief のカスケードは「shell スコープ別＝`char*.seriko.*`」だが、emo2 実測は `sakura.defaultx`／`kero.defaultx`（sakura/kero プレフィックス）。shell descript のスコープ別キーが `char0/char1` 系か `sakura/kero` 系か（あるいは両方）を ukadoc で確定要。
2. **`MountModel` が生 KV・scope 数を持たない**: placement は descript.txt を独自再読込する前提になる（brief の「入力モデル＝MountModel✅・KV✅ が supply 可能」は「読み直せる材料はある」の意で、そのまま scope 数が得られるわけではない）。
3. **構成入力の seam 到達**: `open_startup_window(&WinApp)` は shell dir/descript を受け取らない。plumbing（引数追加 or 別途解決）が設計必須。

## 要件→資産マップ（ギャップタグ: Missing / Unknown / Constraint）

| 要件 | 必要能力 | 既存資産 | ギャップ |
| --- | --- | --- | --- |
| R1 構成駆動の窓生成 | scope 数決定・N 窓 spawn・balloon 窓・seam 差替 | entity spawn 経路・donor 2 本・seam | **Missing**: scope 数導出ロジック（names? descript? 常時 2?）／**Missing**: 固定座標を排した生成 |
| R2 既定位置カスケード | `alignmenttodesktop` 4 層解決・work area 計算・defaultx/defaulttop・両表記受理・未使用値シーム | `Monitor.work_area`・`kv::parse_kv` | **Missing**: resolver 全体／**Unknown**: 値域全量・原点・scope 相対配置（SSP de-facto） |
| R3 座標単位契約・実 DPI | 物理/論理非混在・DPI パラメタ化純関数・実 DPI 証跡 | `DPI`・`Monitor`・`WindowPos` 部品／emo-present 実 DPI 手順 | **Missing**: 単位を型で固定する resolver／**Constraint**: 実 DPI は手動観測（headless 不能） |
| R4 全面ドラッグ＋追従 | 全面ドラッグ・暫定 offset 追従・実 DPI 非破綻 | `on_shell_drag` donor・`DragConfig`・`SetWindowPosCommand` | **Missing**: 固定 offset を排した追従／**Constraint**: 二重スケール回避（物理 px 一貫） |
| R5 既定 z-order 非 topmost | 非 topmost 既定・zorder/sticky シーム | example は `TOPMOST`（donor 対象外） | **Missing**: 非 topmost 生成（TOPMOST を外す）・シーム |
| R6 後続への窓引き渡し | scope 別キャラ窓＋balloon 窓を識別可能公開・attach は非担当 | `EmoPresenter::attach_target` 受け側実在 | **Missing**: 公開データ構造（scope 列挙＋balloon 識別キー） |
| R7 窓移動公開 API（UI スレッド） | UI スレッド上関数・actor 非依存・ブリッジ後続 | `spawn_ui`/`UiSender`（後続結線）・UI スレッド専有 | **Missing**: 窓移動関数の切り出し／**Unknown**: 署名（entity 指定 or scope 指定） |

## 実装アプローチ選択肢

### Option A: main.rs 内モジュール拡張（seam 直下に placement を実装）
`crates/areka/src/main.rs` の `open_startup_window` を本物窓生成へ差し替え、placement ロジックを `crates/areka/src/` 配下の新モジュール（例 `placement.rs`）に置く。
- ✅ 既存 seam・骨格の流儀（純粋関数＋headless テスト）にそのまま乗る。crates/areka 単独で完結し emo-text-layer と非衝突（保護規約通り）。
- ✅ resolver を純粋関数として `placement.rs` に切れば DPI パラメタ化テストが即成立。
- ❌ main.rs 骨格が肥大化しうる（現状 810 行）。モジュール分割で緩和。
- ❌ resolver と窓生成結線の責務分離を自律設計する必要。

### Option B: 独立 placement コンポーネント群（resolver / 窓組立 / ドラッグ追従を分離モジュール新設）
`placement/`（resolver・window-builder・drag-follow の 3 サブモジュール）を新設し、main.rs seam は薄い呼び出しに留める。
- ✅ 純粋 resolver（`descript KV + work_area + DPI → 物理 px`）を単体で檻に入れられる（決定論テスト網羅・記憶 deterministic-test-coverage-mandate 整合）。
- ✅ 窓組立（ghost 所有）と `EcsWindowFactory`（wintf 所有・不改変）の境界が明快（brief Boundary Candidates と一致）。
- ✅ R6 の公開データ構造・R7 の窓移動関数を独立 API 面として設計できる。
- ❌ ファイル数増。ただし brief の「境界候補」が示す通り責務は元々分かれている。

### Option C: ハイブリッド（純粋 resolver は分離・窓生成/ドラッグ結線は seam 近傍）
resolver（純粋・テスト密）＋公開引き渡し型は独立モジュール、窓 spawn とドラッグ追従の ECS 結線は seam 近傍（main.rs もしくは薄い builder）に置く。
- ✅ 「決定論でテスト可能な核（resolver）」と「実 DPI 手動観測に委ねる結線（窓生成・ドラッグ）」の検証戦略の差を構造に反映。
- ✅ 段階実装しやすい（resolver→窓生成→ドラッグ→引き渡し→公開 API）。
- ❌ 分離線（どこまで純粋・どこから結線）の設計判断が要る。

**推奨の方向性（決定ではない）**: 検証戦略（純粋核は DPI パラメタ化テスト／結線は実 DPI 手動）と brief の境界候補が Option B/C を示唆。resolver の純粋関数化は R3.4・記憶方針から実質必須。

## 工数・リスク

- **工数: M（3–7 日）**。窓生成の呼び方は donor 済みで、新規性は resolver（カスケード＋work area＋DPI 単位）と公開引き渡し・窓移動 API。ukadoc 正典確認と実 DPI 手動観測が時間を要する。
- **リスク: Medium**。技術は既知（部品全実在）だが、2026-07-05 リジェクトの直因（物理/論理混在・二重スケール）が再発すれば致命。緩和策=①設計冒頭で単位契約を型レベル確定（newtype 等は design 判断）②DPI パラメタ化純関数テスト③実 DPI（≠96）実行証跡を受け入れ必達（R3.5）。ukadoc de-facto（scope 相対配置・z-order・defaultx 意味論）の未確定が Unknown。

## design へ持ち越す設計判断項目（要件ディスカッションの種）

1. **scope 数の導出源**: `MountModel.names`（sakura/kero name の有無）か、shell/ghost descript のスコープ別キー存在か、M1 は常時 2（sakura+kero）決め打ちか。R1.3「ハードコードしない」との両立方法。
2. **構成入力の seam 到達（plumbing）**: `open_startup_window` に shell dir/descript を渡すか、seam 内で `ConfigInputs`/`MountModel` から再解決するか。`GhostRuntime.mount` を公開するか、placement が独自に descript.txt を再読込するか（emo-present 前例）。
3. **配置カスケードのキー命名**: shell スコープ別キーは `char0/char1.seriko.*` か `sakura/kero.*` か（emo2 は `sakura.defaultx`）。ghost スコープ別は `sakura.seriko.*`/`kero.seriko.*`。ukadoc `descript_ghost`/`descript_shell` を `get_doc` して「キー×所在×優先度×有効条件」の 1 枚表を design.md に載せる（brief 具体指示）。
4. **`alignmenttodesktop` 値域と有効条件**: `bottom`（Y 下端固定・`defaulttop` 無視）／`free`（`defaulttop`/`defaultleft` 有効）／`top` 等の正確な挙動。`defaulttop` は free 限定（ukadoc 明記）を分岐に反映。
5. **`defaultx`⇔`defaultleft` の同義/別義・X 原点**: emo2 は `defaultx`（ukadoc 未確認の de-facto キー）。`x=0` の意味論（work area 基準か・下端整列時も X 調整として有効）を SSP 実挙動で確定し両表記を寛容受理。**【要件討議#2 で確定】** `alignmenttodesktop,bottom`（右下基準）では `defaultx` は **work area 右端からの左方向オフセット**（`defaultx=0`＝右端密着）。要件 R2.10 に反映済み。両表記の寛容受理は設計で実装。
6. **座標単位を型で固定する手段**: newtype（物理 px / 論理 DIP）で混在を型エラー化するか、resolver の入出力を物理 px 一本に統一するか。`DPI`/`Monitor.dpi` のどちらを resolver の DPI 源にするか（Window entity DPI か Monitor DPI か）。
7. **scope0/scope1 の相対配置**: 二体の並び（左右・重なり）は SSP de-facto。M1 でどこまで実挙動を持つか（brief は「窓が生えて置ける・動かせる」まで／連動は M-dual）。**【要件討議#2 で確定】** 既定は scope0（本体）を右下基準・右端へ、scope1（相方）を **scope0 のサーフェス画像幅ぶん左**へずらす単純規則（重なり回避等の複雑ロジック無し）。最終位置は position-persist（M-life）が記憶しユーザーが調整。要件 R2.9・R2.11・R3.1 に反映済み。設計課題は「scope0 のサーフェス幅の入手経路（`WindowPos.size`＝サーフェス原寸物理 px）と DPI 一貫性」。
8. **R6 公開データ構造の形**: scope 番号列挙＋balloon 識別キー（スコープ番号のみだと balloon が取り出せず emo2-boot が詰む＝最終精査検出）。`TargetId` との対応付けは統合側裁量に残す。
9. **z-order 非 topmost の実現**: donor の `WS_EX_TOPMOST` を外す（WindowStyle ex_style から除去）方法・`zorder`/`sticky-window` のシーム化（emo2 未使用ゆえ実挙動なし）。
10. **R7 窓移動 API の署名**: 何を指定して動かすか（entity・scope・balloon 識別）。UI スレッド専有前提で `spawn_ui`/`UiSender` が後から呼べる純関数形へ。
11. **バルーン推奨 DPI（`descript_balloon` の `dpi,推奨DPI`）**: M1 は 96 前提素通しで可か（表示スケール本体は emo 側・本ユニットは窓サイズへの影響のみ）。

## 設計フェーズへ持ち越す調査項目（Research Needed）

- ukadoc `descript_ghost`/`descript_shell` の placement 系キー正典（値域・所在・優先度・有効条件）を `get_doc`/`search_docs` で総ざらい（mcp__ukadoc）。
- SSP de-facto: scope 相対配置・z-order 実挙動・`defaultx`⇔`defaultleft`・X 原点・複数モニタ時の既定モニタ。**【要件討議#3 で確定】** 初期（既定）表示はプライマリモニタ（`is_primary`）の work area 基準（R2.12）。**ドラッグ移動は全モニタ（仮想デスクトップ全体）が対象**で、`DragConstraint` を単一モニタへ閉じ込めない・全モニタ和に対し物理 px 一貫で算出しモニタ境界跨ぎで画面外消失させない（R4.5/R4.6）。どのモニタに居たかの復元は position-persist（M-life）。設計課題は「全モニタ和の bounding rect 算出（`enumerate_monitors()`）と DragConstraint への供給」。
- `WindowPos` 生成時座標算出（`to_window_coords_for_creation`）が物理 px をどう扱うか（CW_USEDEFAULT スキップ含む）を design で 1 度精読。

---

# 設計フェーズ調査ログ（2026-07-10・kiro-spec-design）

> 上記ギャップ分析（2026-07-09）を入力に、ukadoc 正典の総ざらい＋実シンボル精読＋設計統合を実施。
> 成果物は design.md（座標単位契約・正典表・DD1〜DD14）。本節はその調査根拠と正典/デファクトの差異記録。

## ukadoc 正典調査（mcp__ukadoc・brief 必読指示の実施）

### カスケード優先度（確定・requirements R2.3 と一致）

`descript_ghost`／`descript_shell` の全 `alignmenttodesktop` 系エントリが同一の優先度注記を持つ:
**ゴースト側全体 ＜ ゴースト側スコープ個別 ＜ シェル側全体 ＜ シェル側スコープ個別**（shell スコープ別が最強）。

### スコープ別キーの命名（ドリフト所見 #1 の解消）

ghost/shell descript とも **`sakura.`（本体）／`kero.`（相方）／`char*.`（2 人目以降）** が正典プレフィックス。
brief の「shell スコープ別＝`char*.seriko.*`」は不正確で、shell 側にも `sakura.seriko.alignmenttodesktop`／
`kero.seriko.alignmenttodesktop` が正典として存在する（emo2 の `sakura.defaultx`／`kero.defaultx` は正典命名に合致）。
`char0`/`char1` を scope0/1 の別名として受けるのは de-facto 寛容実装（design DD6）。

### `alignmenttodesktop` の値域（brief 未網羅項目①の解消）

descript エントリ自体は値域を列挙しないが、`\![set,alignmenttodesktop,方向]`（sakurascript）が
**`top`／`bottom`／`left`／`right`／`free`**（＋タグ限定の `default`）を列挙。既定は ghost 全体キーの既定値 `bottom`。
別綴 `alignmentondesktop` も有効（タグ注記）。M1 実挙動は `bottom`／`free` のみ・他はシーム（design DD9）。

### `defaulttop` の free 限定（brief 重要修正①の裏取り）

ghost/shell の全 `(sakura|kero|char*).defaulttop` エントリに
「**seriko.alignmenttodesktop が free である場合のみ有効**」と明記。bottom 整列時の Y は下端固定＝defaulttop 無視（R2.4 と一致）。

### `defaultx` は ukadoc に実在する（brief 重要修正②の訂正・正典/デファクト差異）

brief は「`defaultx` は ukadoc で確認できず」としたが、**`(sakura|kero|char*).defaultx` は descript_ghost／descript_shell
双方に正典エントリが実在**する。ただし正典の意味は「**画像ベース X 座標**」（既定＝画像中央 X）＝サーフェス**基準点のアンカー指定**であり、
`defaultleft`（「ディスプレイ上でのデフォルト X 座標」＝窓位置）とは**別キー**。propertysystem `currentghost.scope(ID).x` も
「サーフェスの基準点（通常サーフェス中央下）のスクリーン X。GHOST/SHELL descript の sakura(kero、char*).defaultx で設定可能」と記載。

**採用判断（design DD2）**: 要件討議（2026-07-10・開発者確定・R2.10）は `alignmenttodesktop,bottom` 下の `defaultx` を
「work area 右端（基準位置）からの左方向オフセット・0＝密着」の de-facto 解釈で確定済みのため、design はこれを実装する
（要件は再オープンしない）。正典のアンカー意味論は**転記のみの将来シーム**として本記録に残す（position-persist／balloon 正式配置の
段で基準点モデルが必要になったら再訪）。`defaultx`⇔`defaultleft` は R2.7 に従い同一 X スロットへ寛容統合
（同層内は defaultx 優先＝emo2 実使用側・層間は shell＞ghost）。

**scope≥1 の defaultx 基準（design DD3）**: R2.9（相方＝本体の surface 幅ぶん左）と R2.10（defaultx=0＝右端密着）を同時に満たすには、
defaultx を「**自スコープの基準位置からの左方向オフセット**」（scope0 基準＝右端、scope n 基準＝前スコープの左隣）と定式化するのが
唯一の自己整合解（kero.defaultx,0 を「右端密着」と読むと sakura と重なり R2.9 と矛盾）。

### z-order／sticky-window／推奨 DPI（シーム裏取り）

- `seriko.zorder,スコープID,...`／`seriko.sticky-window,スコープID,...`（descript_shell・SSP 2.4+）＝`\![set,zorder]`/`\![set,sticky-window]` の descript 版。emo2 未使用→転記シームのみ（R5.2）。
- `dpi,推奨DPI`（descript_balloon・SSP 2.7.21+・省略時 96）に加え **`seriko.dpi,推奨DPI`（descript_shell・同版・省略時 96）も存在**（brief 追記の balloon 側のみならず shell 側にもある）。M1 は両方 96 素通しの転記シーム（design DD11）——窓寸は surface/balloon surface 原寸（物理 px）で決まり、表示スケールは emo 側の将来領分。
- マルチモニタ座標系: propertysystem に「プライマリモニタ左上を原点とする**仮想デスクトップ全体**での位置」と明記——R4.5/R4.6（ドラッグは仮想デスクトップ全域・物理 px 一貫）の正典裏付け。

### バルーンのスコープ対応（R1.2 の正典根拠）

`sakura.balloon.alignment`（本体側の吹き出し）／`kero.balloon.alignment`（相方側の吹き出し）が descript_shell/descript_ghost に
スコープ別で存在＝バルーンはスコープごとに 1 枚。emo2 実測は sakura=left／kero=right（暫定 offset の向きにのみ使用）。

## 実シンボル精読（設計フェーズ追加分）

- `WindowPos::to_window_coords_for_creation`（`wintf/src/ecs/window/window_pos/mod.rs:362`）: `position=None`→`CW_USEDEFAULT` 素通し。位置指定時は**実 DPI で `AdjustWindowRectExForDpi`** によりクライアント領域→窓矩形変換。**WS_POPUP 枠なし窓では枠加算ゼロ＝与えた物理 px がそのまま窓矩形**（配置側の追加変換不要を確認）。
- `DragConfig::default()`（threshold=5 物理 px・move_window=true・左ボタン）／`DragConstraint.apply` はクランプのみでスケール変換なし——U4（drag 非介入）の裏付け。
- `areka_parsers` 公開面: `charset::decode(&[u8], DefaultEncoding) -> String`／`kv::parse_kv(&str) -> BTreeMap`／`package::resolve(&Path, DefaultEncoding) -> Result<MountModel, MountError>`——placement::source の実装部品が全て公開済み（areka-parsers 不改変で成立）。
- `crates/areka/Cargo.toml`: areka-emo-atlas／-compose／-present は現状 **dev-dependencies（example 専用）**。main.rs シームでの採寸には atlas/compose の通常依存昇格が必要（design DD5）。
- `GhostRuntime.mount` は private＋`#[allow(dead_code)]`（読み出し口なし）を再確認→ placement は `package::resolve` を自前で呼ぶ（design DD4・areka-ghost 不改変）。

## 設計統合（synthesis）の記録

- **一般化**: 「scope0/scope1 の 2 窓」を「検出スコープ集合×（キャラ窓＋バルーン窓）」へ一般化（インターフェイスは N スコープ・実装スコープは emo2 の 2）。resolver の基準位置を連鎖形（`base_x(n) = char_x(n−1) − w(n−1)`）にすることで char*（n≥2）へ構造のまま伸びる。
- **build vs adopt**: ドラッグは wintf `DragConfig{move_window:true}` を採用（自前ドラッグ座標処理を**書かない**ことが 07-05 再発防止の本体）。descript 読込は areka-parsers 3 部品を採用。採寸は emo-atlas/compose を採用（PNG ヘッダ直読の自前実装は合成外形と乖離し得るため棄却）。窓生成は entity-spawn→`create_windows` 既存経路を採用（`EcsWindowFactory` は `pub(crate)`＝直接呼ばない）。
- **単純化**: (1) M1 は `DragConstraint` を**付与しない**（無制約＝仮想デスクトップ全域）——制約算出ロジックそのものを消すことで 07-05 の欠陥面を除去し、R4.6 は純粋ヘルパ `virtual_desktop_union`＋規則の文書化で担保（design DD8）。(2) バルーン追従 offset は配置時確定の静的値（動的再計算なし・R4.4 の暫定規則）。(3) resolver は wintf 型非依存の自前値型（`RectPx` 等）で閉じ、newtype による物理/論理の型分離は**モジュール境界の単一通貨規約（U1〜U5）で代替**（M1 パイプラインに論理値が存在しないため型追加は過剰と判断）。
- **受容トレードオフ**: measure が採寸のために emo アセットを合成→破棄し、emo2-boot が装着のため再ロードする**二重ロード**を受容（アセット所有＝emo2-boot の境界を崩さない）。emo2-boot 側で「placement が採寸済みアセットを引き渡す」最適化を検討する場合は `GhostWindows` 契約の Revalidation Trigger。
- **リスク低減**: 07-05 の再発防止は ①design 冒頭の座標単位契約（U1〜U5・レビューでエラー扱い）②DPI パラメタ化 resolver テスト（96/120/144/192・T-R 群）③実 DPI（≠96）実行証跡の受け入れ必達（R3.5）の 3 層。

## 残課題（design で確定済み・実装時の注意のみ）

- `free` の座標原点はプライマリ work area 左上（design DD10）——SSP 実挙動での裏取りは emo2 非使用のため未実施（決定論テストで自団の意味論を固定・将来 SSP 差異が観測されたら DD10 のみ差し替え）。
- scope n≥2 の初期 surface id（正典既定なし）: 暫定 10＋warn（design measure 節）。
- main.rs シーム窓は emo2-boot 装着まで不可視（WUC 合成・内容なし）＝対話 close 不能が正しい状態。終了は smoke ゲート／Ctrl+C（rustdoc 明記・emo2-boot で解消）。
