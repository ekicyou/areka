# 設計バリデーションレポート: areka-P0-window-placement

> 実施: 2026-07-10（kiro-validate-design・非対話モード）
> 入力: spec.json（language=ja・phase=design-generated）／requirements.md（確定・7 要件）／design.md／research.md／brief.md／.kiro/steering/
> プロセス: design-review.md 準拠（Analysis → Critical Issues → Strengths → GO/NO-GO）

## Review Summary

design.md は 2026-07-05 リジェクトの根本原因（物理 px／論理 DIP の単位混在・ドラッグ二重スケール）への対策を「座標単位契約（U1〜U5）＝設計正本」として冒頭に据え、純粋 resolver（wintf 非依存・DPI パラメタ化テスト 96/120/144/192）＋実 DPI（≠96）手動受け入れ必達の 3 層防御で要件 R3 を構造的に満たす。要件討議の確定事項（スコープごとバルーン窓・scope0 右下＋scope1 は surface 幅ぶん左・初期＝プライマリ work area／ドラッグ＝全モニタ）はすべて設計に忠実に反映され、全 33 acceptance criteria が Traceability 表で個別に結線されている。**設計が参照する実コードシンボルを網羅的に突合した結果、不一致はゼロ**であり、実装可能性は高い。

## Analysis

### 1. 実シンボル突合（本レビューで独立検証・全件一致）

| design.md の記述 | 実コード | 判定 |
|---|---|---|
| `open_startup_window(app: &WinApp)` シーム | `crates/areka/src/main.rs:411`（`spawn_dummy_window`:307・`DummyWindowMarker`:191） | 一致 |
| `ConfigInputs { ghost_root, balloon_root }` | `main.rs:79`（`resolve_config_inputs` 実在） | 一致 |
| `DPI` Component（`scale_x/y`・`to_logical_*`・`to_physical_*`） | `wintf/src/ecs/window/dpi.rs:61–132`（全メソッド実在） | 一致 |
| `Monitor.bounds/work_area/dpi/is_primary`・`enumerate_monitors()` | `wintf/src/ecs/window/monitor.rs:70–73/173` | 一致 |
| `DragConfig.move_window`・`DraggingState`・`DragConstraint.apply`（物理 px） | `wintf/src/ecs/drag/mod.rs:35/45/72/84/97` | 一致 |
| `WindowPos::to_window_coords_for_creation` | `wintf/src/ecs/window/window_pos/mod.rs:362` | 一致 |
| `SetWindowPosCommand`／`enqueue` | `wintf/src/ecs/window/command.rs:114/152` | 一致 |
| `HitTest::none()`／`alpha_mask()`・`OnDrag`・`OnPointerPressed`・`ClickThroughRegistryHandle`・`Phase<T>`／`EventHandler<T>` | hit_test/mod.rs:110/124・drag/dispatch.rs:61・pointer/dispatch/mod.rs:83/21/68・clickthrough/controller.rs:373 | 一致 |
| `EmoPresenter::attach_target` | `areka-emo-present/src/presenter.rs:100` | 一致 |
| `areka_parsers`: `kv::parse_kv`・`charset::decode`・`package::resolve` | kv/parse.rs:20・charset/decode.rs:24・package/resolve.rs:40（すべて公開） | 一致 |
| emo2 shell descript 実測値（`alignmenttodesktop,bottom`・`sakura.defaultx,0`・`kero.defaultx,0`・`balloon.alignment` left/right） | fixtures/emo2/shell/master/descript.txt:10–16（完全一致） | 一致 |
| emo2 ghost descript `kero.name`（DD6 スコープ検出シグナル） | fixtures/emo2/ghost/master/descript.txt（`kero.name,エモ` 実在） | 一致 |
| emo-atlas/compose は現状 dev-dependencies（DD5 昇格が必要） | `crates/areka/Cargo.toml:33–40` で確認 | 一致 |
| wndproc ドラッグ中の `WindowPos` echo-bypass 更新（on_char_drag の前提） | `wintf/src/runtime/wndproc_bridge.rs:54` doc 明記＋mock-shell.rs:375 donor 実証 | 一致 |

### 2. リジェクト教訓（07-05）への対策検証

- **座標単位契約が設計の正本**（Approach 0・U1〜U5）: 配置パイプライン全体を物理 px 単一通貨に固定し、`BoxStyle` 使用禁止（U2）・drag 非介入（U4）で混在演算と二重スケールの発生面そのものを除去 — R3.2/3.3 を規約＋テストで担保。
- **resolver は純粋・DPI パラメタ化テスト可能**: wintf 非依存の自前値型（`RectPx` 等）で閉じ、T-R 群を dpi ∈ {96,120,144,192} でパラメタ化（隠れた `/96` 変換があれば 96 以外で崩れる檻）— R3.4 を直接満たす。
- **実 DPI（≠96）手動受け入れ必達**: `examples/window-placement.rs` の観測プロトコル①〜⑤で「dpi=96 のみの緑＝不合格」を明文化 — R3.5 を受け入れゲートとして固定。
- **DD8（DragConstraint 非付与）**: 07-05 の単一モニタ誤釘付けの欠陥面を消しつつ、R4.6（条件付き要件）を純粋ヘルパ `virtual_desktop_union`＋テスト T-R7 で担保。要件の条件構造の正確な読解。

### 3. 討議確定事項・並走保護の検証

- スコープごとバルーン窓 1 枚（R1.2・ukadoc 正典根拠つき）／scope0 右下＋scope1 は scope0 surface 幅ぶん左（P2 連鎖基準・DD3 は R2.9 と R2.10 の唯一の自己整合解）／初期＝プライマリ work area（R2.12）・ドラッグ＝全モニタ（R4.5）— すべて反映済み。
- 並走保護: 改変は `crates/areka`（main.rs＋placement/ 新設＋Cargo.toml）のみ。`areka-emo-present`／`areka-parsers` は消費のみ（example の dev-dependency 使用は新規ファイル＝brief 許容範囲）。
- ukadoc 正典表: brief の必読指示（キー×所在×優先度×有効条件の 1 枚表・emo2 実測検証行）を完全実施。brief のドリフト 2 件（スコープキー命名・`defaultx` 正典実在）を research.md で訂正記録済み。

## Critical Issues

ブロッキング（NO-GO 相当）の問題は検出されなかった。以下 2 件は非ブロッキングの改善指摘（設計ディスカッションの種）。

🟡 **Issue 1**: scope1 バルーンの初期位置が scope0 キャラ窓と重畳する（暫定規則の観測上の見え方）
**Concern**: P5 幾何＋emo2 実測（`kero.balloon.alignment,right`・両スコープ defaultx=0・同幅なら `balloon_x(1) = char_x(0)`）では、scope1 のバルーンが scope0 キャラ窓上に重なって出現する。バルーン surface0 は内側不透明（A=255 白）のため、重畳域では αマスクにより scope0 のドラッグではなくバルーンが掴まれる。
**Impact**: 手動受け入れ（プロトコル②③）で観測者が「配置破綻」と誤読し、正しい暫定挙動が不合格扱いになるリスク。実装欠陥ではなく要件が受容した単純規則（2.9「重なり回避ロジックなし」・4.4 暫定 offset）の帰結。
**Suggestion**: `examples/window-placement.rs` の rustdoc 観測プロトコルに「scope1 バルーンの重畳は暫定規則の正常挙動（正式配置は balloon 表示系の後続）」と期待図を 1 行明記し、pass/fail 判定から明示的に除外する。
**Traceability**: R4.4・R3.1（受け入れ判定の主対象定義）／**Evidence**: design.md「配置規則 P5」・DD7・「examples/window-placement.rs」節

🟡 **Issue 2**: `GhostTitles` が署名参照のみで構造未定義
**Concern**: `spawn_ghost_windows(world, placements, titles: &GhostTitles)`／`PreparedPlacement.titles` に登場する `GhostTitles` の形（スコープ別 name の由来キー・欠落時既定値の正本）が design.md 内で定義されていない。
**Impact**: 軽微。実装時に自明に埋まる粒度だが、窓タイトルは Win32 識別・デバッグ観測に使われるため、tasks 生成時に置き場所（config か source か）が揺れうる。
**Suggestion**: tasks フェーズで「`GhostTitles` = ghost descript `name`/`sakura.name`/`kero.name` 由来・欠落時 `"areka"` 既定」の 1 行定義を config または source の責務へ割り当てる。
**Traceability**: R1.1（窓生成の構成入力）／**Evidence**: design.md「placement::spawn」「main.rs seam」節の署名

## Design Strengths

1. **リジェクト根本原因への 3 層防御が構造化されている**: 座標単位契約（U1〜U5・レビューでエラー扱いの正本）→ DPI パラメタ化純粋 resolver テスト（T-R 群・07-05 欠陥を檻に入れる回帰設計）→ 実 DPI 手動受け入れ必達（3.5）。「テスト緑が欠陥を隠す」経路そのものを潰しており、deterministic-test-coverage-mandate とも整合。
2. **実コード接地の密度が高い**: 設計が参照する全シンボル（wintf 座標型・drag 機構・parsers 公開面・emo2 fixture・donor 2 本）が行番号つきで実在確認済みであり、本レビューの独立突合でも不一致ゼロ。DD1〜DD14 が各判断の根拠と要件 ID を明記し、Traceability 表が全 33 AC を結線。実装フェーズでの発明余地（＝逸脱リスク）が最小化されている。

## Final Assessment

**Decision: GO**

**Rationale**: 07-05 リジェクトの再発防止（座標単位契約・純粋 resolver・実 DPI 必達）が設計の正本として構造化され、確定要件 33 AC すべてに実装コンポーネントが結線済み。参照シンボルの実在・並走保護（emo-present／parsers 不改変）・討議確定事項の反映を独立検証し、不一致ゼロ。検出した 2 件はいずれも非ブロッキング（受け入れプロトコルへの注記 1 行と型定義 1 行で解消可能）であり、実装リスクは受容範囲。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で Issue 1（バルーン重畳の観測注記）・Issue 2（`GhostTitles` 定義）を確認・反映
2. `/kiro-spec-tasks areka-P0-window-placement` でタスク生成へ進む
3. 実装時は U1〜U5 違反をレビューエラー扱いとする規約（design 正本）を reviewer 指示に含めること
