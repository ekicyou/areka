# 設計バリデーションレポート: areka-P0-kero-balloon

> 実施: 2026-07-31 / worktree `claude/kiro-start-areka-p0-kero-1f9ab7`（HEAD `969a9b3`）
> 対象: `design.md`（2026-07-31 生成）／入力: `requirements.md`（確定）・`research.md`・`.kiro/steering/`
> 方式: kiro-validate-design REVIEW プロセス（Analysis → Critical Issues → Strengths → GO/NO-GO）・非対話実行
> 検証手段: 設計が引用する `path:line` アンカーを本 worktree の実コードに対して Read/Grep で実測突合（下記 4 章）

---

## 1. レビューサマリ

設計は「最下流の純関数権威（`areka-emo-present::balloon`）＋上位は引数の通し」という 1 本の背骨で、系列解決・定義マージ・採寸・文字層・アニメ表の全消費点を単一規則へ束ねており、research.md が最大の実機限定リスクと特定した「採寸 (B) と装着 (A) の独立 2 実装の規則ずれ」を構造的に解消する。要件 48 AC（R1:10・R2:6・R3:8・R4:7・R5:6・R6:4・R7:7）は Requirements Traceability に全件が実現要素付きで存在し、ディスカッション裁定（D1 windowposition In-scope・D9 per-scope AnimationTable＋looper.rs 境界拡張・D11 正規名 p{n}def 連鎖）も設計本文へ正しく反映されている。実コードに対するアンカー実測（約 25 点）で齟齬ゼロ——実装着手可能な品質に達している。

## 2. Critical Issues

**なし（0 件）**。ブロッキングに該当する構造欠陥・要件ギャップ・正典乖離・ウェーブ干渉違反は検出されなかった。

以下は**非ブロッキングの助言事項**（実装時・タスク生成時に拾えば足りる粒度。設計ディスカッションでの確認候補）:

- 🟡 **助言 1: `balloon_offset` 合流欄の単位空間の混在を doc で明示すること**
  **Concern**: `windowposition` 由来の調整量は k 適用済み物理 px で `ScopeConfig.balloon_offset` へ合流する（R3.6）が、同欄の既存供給元 `balloon.offsetx/offsety`（descript）は非スケールの生値のまま加算される（`resolver.rs:186` は `unwrap_or((0,0))` の生加算・現行規約の維持は Out of scope として正当）。emo2 は descript offset が `None` ゆえテストでは混在が顕在化しない。
  **Suggestion**: 新設 `placement/windowposition.rs` の doc コメントに「本欄は物理 px 加算欄であり、descript offset の非スケールは既存規約の温存（W5 対象外）」と 1 行明記し、将来の取り違えを封じる。
  **Traceability**: R3.6／**Evidence**: design.md「areka / placement」Service Interface・`resolver.rs:186`（実測）。
- 🟡 **助言 2: R6.1 の info ログも placement／boot の 2 呼出点から重複出力される**
  **Concern**: 設計は R6.3（windowposition/validrect 実値ログ）の scope あたり 2 行出力を「権威一元化の生き証人」と明記済みだが、`resolve_balloon_faces` 完了時の R6.1 info（および R6.2 warn）も同様に 2 回出る。サインオフの grep 突合手順が行数を数える形だと誤カウントし得る。
  **Suggestion**: Monitoring 節の R6.1/R6.2 にも「2 呼出点×各 1 回・値一致を確認」の注記を揃える（実装時の 1 行加筆で足りる）。
  **Traceability**: R6.1/R6.2／**Evidence**: design.md「Monitoring」1〜2 項・Flow 1。
- 🟡 **助言 3: `actor.rs:2817` の檻内コメントが `:2665` のテスト名を名指ししている**
  **Concern**: R7.2 の更新対象として設計は `:2665`（`refresh_actor_scale_with_same_k_is_noop_returning_false`）の名称・意図更新を挙げるが、`:2812-2817` のキル排他性コメントが同テスト名を参照しており（実測）、リネーム時に追随しないと陳腐化コメントが残る（R7.2 が禁じる状態）。
  **Suggestion**: R7.2 の更新一覧に `:2817` コメントの追随を含める。
  **Traceability**: R7.2／**Evidence**: `crates/areka-emo-text/src/actor.rs:2812-2817`（実測）。

## 3. Design Strengths

1. **単一権威の置き場所と粒度が実測に裏づけられている**——列挙 2 実装（`balloon.rs:50 enumerate_frames`／`measure.rs:390-409` の固定名最小再実装）の実在を確認したうえで、列挙・ID 単位選択・上書き名導出・model 読込までを `areka-emo-present::balloon` に同居させ、3 消費者（assets/measure/placement）を消費者に落とす。「採寸した窓寸 ≠ 合成された枠」という実機でしか見えない欠陥クラスを型と構造で不可能にしており、`TextSlotBinding`/`ResolvedBalloonText` の `PartialEq` 既存導出（actor.rs:46/:133 実測）・looper の `(ActorKey, Slot)` キー走査（looper.rs:177-188 実測）など、既存コードの形を最大限流用して差分を最小化している。
2. **ウェーブ干渉制約の遵守が設計の一級市民になっている**——`resolver.rs` P1〜P5 無改変（P5 の `balloon_offset` 加算口が既存・実測確認）でエスケープ条項を不発動に保ち、W6 2 本（balloon-visibility／bindoption-exclusivity）と W6.5 への申し送りを Revalidation Triggers として明文登記。`frame.rs:531-540` の無条件 ShowSurface（W6 領分）に触れない線引きも実コードの関数域と一致する。

## 4. アンカー実測突合（設計の主張 vs 実コード）

| 設計の主張 | 実測結果 |
|---|---|
| `balloon.rs:39` `FRAME_PREFIX="balloons"`・`:50 enumerate_frames`・`:88 frame_id`（private）・`:120 build_balloon_target`（scope 引数なし） | ✅ 全一致 |
| `balloon.rs:264` テスト `frame_id("balloonk0.png") == None` | ✅ 一致（`:258-271`） |
| `assets.rs:79` `BALLOON_FACE0_TXT="balloons0s.txt"`・`:122 balloon_model`（doc「全 scope 共有」）・`:281-284` scope ループ（毎回同一引数）・`:287-300`「先頭 World から 1 度だけ」注記・`:322 build_balloon_model`・`:330 read_decoded_lenient`（一律 warn） | ✅ 全一致 |
| `assets.rs:439-449` テストが単一 model の sakura 値（validrect 46/-56/36/-44・wp 266/-129）を assert | ✅ 一致 |
| `measure.rs:127-128/:227-231` W5 席保全コメント・`:179-180` ループ外 1 回採寸・`:390-409` 固定名 `"balloons0.png"` 最小再実装・`:383` scope:0 帰属 | ✅ 全一致 |
| `frame.rs:205` `balloon_models: HashMap<u32, BalloonModel>`（W5 の席の明示 doc）・`:531-540` 無条件 ShowSurface(surface_id=0)・`:549-556` `connect_balloon_text`＋同一写像警告・`:928-970 run_text_scale_phase`（構造無改変で per-scope 化が効く形） | ✅ 全一致 |
| `actor.rs:348-375` `refresh_actor_binding` が k のみ比較（W4 申し送りの穴）・`:46/:133` `PartialEq` 導出済み・`:220 layout_input: HashMap<ActorKey, ResolvedBalloonText>`・`:2665` 既存檻 | ✅ 全一致（D3=(ii) の判定キー拡張は追加 derive 不要で実装可能なことを確認） |
| `looper.rs:43-49` `SerikoLoopConfig.balloon_table` 単数・`:180/:236` `Slot::Balloon` 表引き・走査キーが `(scope: ActorKey, slot, sid)` | ✅ 全一致（`balloon_tables.get(scope)` への変更が既存走査構造にそのまま載ることを確認） |
| `resolver.rs:77-81` `balloon_offset` 恒等式・`:109-113` DD7「正式規則は後続へ委ねる」・`:186` `balloon_offset.unwrap_or((0,0))` 加算 | ✅ 全一致（P5 無改変で供給合流が成立） |
| `config.rs:47-50` `ScopeConfig.balloon_offset`（emo2 は None＝未使用） | ✅ 一致 |
| `placement/mod.rs:254-290 prepare_stages`・`:278-283` 採寸 info ログ | ✅ 一致（windowposition 供給の挿入点として妥当） |
| `spine.rs:1249-1253` S4 doc「emo2 fixture は balloons0.png のみ」（陳腐化予定の事実記述） | ✅ 一致（assert 本体と doc の分離も設計どおり） |

**齟齬 0 件**。stale アンカー・実コード構造との矛盾は検出されなかった。

## 5. 観点別チェック

- **要件カバレッジ**: 48 AC 全件が Traceability 表に存在し、実現要素（コンポーネント／ファイル）と対応。境界（Out of scope 14 項目）も Non-Goals / Out of Boundary へ漏れなく転写。✅
- **ディスカッション裁定との整合**: D1（windowposition＝初期既定値の正典化・永続値優先・`balloon_offset` 合流）／D9（per-scope AnimationTable・`areka-seriko/src/looper.rs` の境界拡張明示・bind/state/actor 無改変）／D11（正規名 `p{n}def` 先頭・`s`/`k` 旧名・n≧2 連鎖の `balloonk`＝名指し系列で `p1def` 不含・デフォルト地位は scope 0 のみ）——いずれも `prefix_chain` 仕様・Boundary Commitments・COMPAT 記録一覧 (a)(b)(c) に字義どおり反映。✅
- **W5/W6 干渉制約**: `resolver.rs` P5 無改変＝エスケープ条項不発動（実測裏づけ）。新設 `windowposition.rs`＋`mod.rs` 小ハンクは Revalidation Triggers へ登記済み（dpi-window-vanish の編集集合確定時に再確認）。assets.rs 異ハンク（本仕様 `:278-300` ⇄ bind `:196-210`）・W6 vis への `BalloonScopeAssets` 実形申し送りも research.md 6 章の干渉台帳と一致。✅
- **判定分岐の檻化可能性**: 檻 1〜10 は全て純関数（`prefix_chain`／`select_faces`（fs 非依存に分離）／`override_file_name`／`to_screen_adjust`／`refresh_actor_binding` 内側シーム（in-crate 檻用の分離が実在・actor.rs:345-348 実測）／looper 表引き）。合成 fixture の donor（`TempDir`＋`MemoryDecoder`・balloon.rs:184-215）実在確認。実 fixture 実測値（400×224／288×203・kero wp −190,−75）が期待値として明記され、実機サインオフは有界 auto-exit＋ログ grep の決定論判定。✅
- **ログ規約・失敗経路**: log-first 全面踏襲・D8 の層別レベル（NotFound→debug）は「相方側で毎起動 warn」の事故を先回りで防ぐ。R1.7 は権威の単一施行点で error＋Err→既存 2 縮退経路。✅

## 6. Final Assessment

**判定: GO**

**根拠**: 要件 48 AC の完全トレース・全ディスカッション裁定の忠実な反映・約 25 点のコードアンカー実測で齟齬ゼロ、かつ W5/W6 干渉制約（resolver 無改変・先行着地・Revalidation Triggers 登記)を満たしており、ブロッキング欠陥がない。助言 3 件はいずれも実装時の doc/テスト衛生で吸収できる粒度であり、設計変更を要さない。

**Next Steps**:
1. 設計ディスカッション（`kiro-design-discussion`）で助言 1〜3 の扱いを確認（採否いずれでも設計本文の改稿は不要）。
2. `/kiro-spec-tasks areka-P0-kero-balloon` でタスク生成へ進む。
3. 実装着手時に `git log -- crates/areka-emo-present/src/balloon.rs crates/areka/src/emo2_boot/assets.rs crates/areka/src/placement/measure.rs` で並走着地の陳腐化を再確認（research.md 10 章の定跡）。
