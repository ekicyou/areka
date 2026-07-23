# Design Validation Report: areka-P0-seriko-loop

> 実施日: 2026-07-23 ／ 対象: design.md（design-generated・approvals.design.approved=false）
> 手法: kiro-validate-design REVIEW プロセス（Analysis → Critical Issues → Strengths → GO/NO-GO）。
> design が引用する file:line 主張は現行コードベースへの実査（Grep/Read）で裏取り済み。

## Review Summary

設計は既存アーキテクチャ（アクター＋純関数コア・単一発行点・冪等ガード・容量1メモ化）への Extension として一貫しており、設計上の全主要主張（ticker 2 系統固定・DisplayCommand 非 non_exhaustive・ComposeKey 予約記述・pattern0 厳格選択の宿題明記・ComposeMethod registry 既存）がコード実査で裏付けられた。要件ディスカッション裁定2点（面種非仕切り・method 忠実転記）も設計へ正しく織り込まれている。残る懸念は実装時に檻で確定できる水準であり、実装移行可能と判断する。

### 実査で確認した設計主張（すべて一致）

| 設計主張 | 実査結果 |
|---|---|
| `spawn_ticker` は kanade/dispatcher 2 系統固定・`BoundarySchedule` は同クレート内純粋層（pub(crate)） | ✅ `areka-ghost/src/ticker.rs:82, 165-171` — additive な `spawn_loop_ticker` 新設で既存 2 系統無改変は成立。クロージャ配送による orphan rule 回避（`impl From<Tick> for SerikoMsg` は ghost/seriko どちらにも書けない依存関係）も正確 |
| seriko 受信面は `Cue \| Close` のみ・`emit_display` 単一発行点・「時間駆動ループが同じ発行点を再利用できる」doc | ✅ `areka-seriko/src/actor.rs:53-58, 134-136` |
| parser の overlay フィルタ（転記の穴の物理位置） | ✅ `areka-parsers/src/shell/decode.rs:335`（`fields.get(1) == Some("overlay")` のときのみ Pattern 充填）・`model.rs` の `Pattern` に method 欄なし・`Interval` は `#[non_exhaustive]` 3 種 |
| `DisplayCommand` は意図的に非 `#[non_exhaustive]`（追随強制文化）・`PresentCommand` は `#[non_exhaustive]` | ✅ `areka-seriko/src/output.rs:25-42`・`areka-emo-present/src/command.rs:38` — Show/ShowBalloon への pattern 欄追加はコンパイル強制追随として妥当 |
| `ComposeKey` に pattern 追加の予約記述 | ✅ `areka-emo-present/src/cache.rs:43-51`（「将来 seriko がアニメ pattern 状態を…本キーへ追加する」原文確認） |
| pattern0 厳格選択＝本 spec への宿題明記 | ✅ `areka-emo-compose/src/plan.rs:306-318`（「それらのフレームは seriko-loop（M-life）が再生する」） |
| `ComposeMethod` 完全語彙 registry 既存（新造不要・adopt） | ✅ `areka-emo-compose/src/method.rs:29-165`（`from_name` 同義写像・`Unknown(Box<str>)` 吸収・`is_implemented()==Overlay` のみ） |
| `ScopeStates` の `dynamic_binds`／`current_binds` read-only／`commit_bind` 冪等パターン（`commit_pattern` の鏡映元） | ✅ `areka-seriko/src/state.rs:95, 239-244, 320-323` |
| `EmoWorld` からの表構築（「seriko が再利用する」明記） | ✅ `areka-emo-compose/src/normalized.rs:62-64`・`areka/src/emo2_boot/assets.rs`（shell/balloon 双方の EmoWorld が boot 資産に実在） |

### 裁定2点の反映確認

- **裁定 (a) 面種非仕切り**: 評価器・表・PatternState・commit を surface 非依存に保ち、シェル表/バルーン表を「surface ID 名前空間の別」と明記（能力の仕切りでない）。`ShowBalloon` にも pattern を搬送し、両 slot を同一コードパスで評価。emo2 のバルーン表が空なのはデータ事実として扱う。✅ 要件 Boundary の裁定文言と整合。
- **裁定 (b) method 忠実転記**: parser は `DrawMethod(String)` opaque NewType で原文無加工転記（`ElementPath` 先例準拠）・decode.rs のフィルタ撤去で全メソッド転記・意味解釈は下流 `ComposeMethod::from_name` へ・新旧形式の正典位置は doc 転記。既存 registry の adopt により語彙の二重定義を回避。✅ R4.6/R8.4 と整合。

## Critical Issues（≤3・いずれも NO-GO 水準ではない）

🔴 **Critical Issue 1**: compute_extent 不変の前提「まばたきコマはベース外形内」が実測未裏付け
**Concern**: 設計は `compute_extent` を変更せず transient コマは外形へ寄与しない（越えた分はクリップ＝許容劣化）とするが、「まばたきコマ（1410-1412/2106-2110）はベース外形内に収まる」は前提として宣言されるのみで、fixture 実測の裏付けが design に引かれていない。特に kero の animation0 は `interval,random`（非 bind 種）ゆえ、そのコマ surface は静的外形の母集合（全 element＋全 bind pattern0）に一切入らない。
**Impact**: 前提が崩れるとまばたきが欠けて描画され、R9 実機サインオフで初めて露見する（決定論テストが外形前提を檻に入れていないと素通りする）。
**Suggestion**: tasks 生成時に「表構築時 or golden で、採録アニメ全コマの (原寸＋x,y) がベース Extent 内に収まることを検証する檻」を追加するか、spine e2e golden がコマ描画位置を画素で固定することを DoD に明記する。
**Traceability**: R5.3・R9.1/9.2
**Evidence**: design.md「合成合流と method ゲート」compute_extent 段落

🔴 **Critical Issue 2**: 未モデル化 interval 語彙の fallback-Bind 縮退と R8.2「完全形保持」・Error 表の観測性の齟齬
**Concern**: `decode_animations` は 3 種以外の interval キーワード（sometimes/periodic 等）を passthrough し、pattern を持つ ID は既定 `Interval::Bind` へ倒す（decode.rs:286-291・現行挙動）。設計の Error 表は「採録外 interval（Bind・将来 variant）→ debug! 非採録」とするが、表構築時点では元語彙が既に失われており、fallback-Bind と真正 Bind を区別できない＝「sometimes と書いたのに黙って bind 扱い」の診断が不可能。R8.2 の「定義に現れる語彙の完全形保持」は method には拡大されたが interval には及ばない（gap 分析 [Constraint] として裁定済みのスコープではある）。
**Impact**: emo2 は非影響だが、将来ゴーストでの無音の挙動齟齬（[areka-log-first-no-silent-failure] の穴）と、[defer-canon-with-full-vocabulary-and-tracking-spec]（完全語彙＋追跡）の残債が設計文書上で不可視になる。
**Suggestion**: (1) decode の未認識 interval キーワードに debug!/warn! を 1 行足して観測性を回復する（method フィルタ撤去と同ファイル・同 spec 内の最小差分）、または (2) design/tasks に「interval 完全語彙は将来 spec 送り・fallback-Bind は診断不能」と明記して追跡 spec への申し送りとする。いずれかを design discussion で確定すること。
**Traceability**: R8.2・R7.5
**Evidence**: design.md「Error Categories and Responses」表構築行／Requirements Traceability 8.2 行

🔴 **Critical Issue 3**: IdleResidual からの再抽選発火直後（Pending 中）の残留コマ扱いが未明文
**Concern**: `-1` 無し末尾残留（IdleResidual）のアニメが再抽選で発火し、先頭コマの wait > 0 の場合、`frame_at` は `Pending`（コマ搬送なし）を返す。on_tick の PatternState 組立規則では Pending のアニメはエントリを持たないため、残留していた最終コマが一瞬クリアされてベースが露出する（発火→Pending→Active の遷移でフラッシュ）。この挙動が意図か（残留維持か即クリアか）が設計に明文化されていない。
**Impact**: emo2 実測列は先頭 wait=0（0/150/22・0/40/80）ゆえ fixture では非観測だが、決定論テストの期待値（7.2 の PatternState 列）を書く際に実装者の解釈が割れ、檻の期待値がブレる。
**Suggestion**: timeline/looper の doc とテストで「発火後 Pending 中は残留コマを維持する／クリアする」のどちらかを 1 行で確定する（ukadoc 無規定ゆえどちらでも可・決定論であることが要点。9.4 と同じく実機齟齬時 SSP 裏取りの但し書きで足りる）。
**Traceability**: R4.4・R7.2・R9.4
**Evidence**: design.md「1 アニメの再生状態機械」／LoopRuntime on_tick (3) の組立規則

## Design Strengths

1. **全結合点が実コードの証拠に基づく**: 単一発行点 doc・キャッシュキー予約記述・pattern0 厳格選択の宿題明記・ComposeMethod registry・commit_bind 鏡映など、既存コードが「本 spec が来ること」を予約している箇所を正確に特定して接いでおり、独自機構の新造がゼロ（新規依存辺ゼロ・新規 crates.io 依存ゼロ・rand 不使用）。orphan rule まで踏まえたクロージャ配送の選択は additivity 主張（1.4）を構造的に保証している。
2. **決定論設計の徹底**: 経過時刻の関数としての `frame_at`（tick 落ち・catch-up 安全）・1000ms 絶対グリッド抽選・固定抽選順序（D-7）・注入 tick＋注入乱数のみのテスト戦略が [deterministic-test-coverage-mandate] を全経路で満たす形になっており、デファクト推定 2 点（9.4）も檻の期待値として明文化済み。

## Final Assessment

**Decision: GO**

**Rationale**: 既存アーキテクチャとの整合・要件トレーサビリティ（R1–R9 全 AC が Components/Interfaces/Flows へ写像済み）・決定論検証可能性のいずれも実装移行水準にあり、設計主張はコード実査で全件裏付けられた。Critical Issues 3 件はいずれも「実装前に 1 行の確定 or 檻の追加」で解消できる残留リスクであり、アーキテクチャ再設計を要さない。

**Next Steps**:
1. design discussion で Issue 1–3 の扱いを確定（Issue 2 は (1) decode への observability 1 行追加か (2) 追跡 spec 申し送りかの二択）。
2. 確定内容を design.md（または tasks の DoD）へ反映。
3. `/kiro-spec-tasks areka-P0-seriko-loop` で実装タスク生成へ。
