# Brief: areka-P0-seriko-engine

> **種別**: 本坑（main）。⑤ seriko トラックの M-boot ユニット（**静的＋指令適用**のみ・SERIKO ループ/blink は M-life `seriko-loop`）。
> **調査日**: 2026-07-05（sakura✅・emo-compose✅ 完了後の実シンボル偵察＋ukadoc 正典確認）。
> **解禁根拠**: 上流契約が両方とも正本確定済み——**sakura の再生出力契約（`TalkCue`/`SurfaceSink`）✅**＋**emo-compose の合成入力契約（surface id＋`BindSet`）✅**。観測の独立化（fixture 指令列直入力→mock sink 観測）で単体完結。

## Problem

sakura が `SurfaceSink` へ流す surface 指令（`\s[ID]`）を受けて「**今どのスコープにどの surface を出すか**」を保持し emo を駆動する層が存在しない。emo-compose は純粋関数（状態なし）・emo-present は表示口（指令の適用先）——**surface 状態の所有者が不在**のままでは M-boot 統合（emo2-boot）で script が surface を切り替えられない。

## Current State

- **sakura ✅（出力契約の正本・実シンボル）**: `SurfaceSink { fn emit(&mut self, cue: TalkCue) }`／`TalkCue { at: f64, actor: ActorKey, command: CueCommand }`。Shell 系 variant は **`CueCommand::Emote { key: String }`**（alias/name 文字列の不透明転写）と **`CueCommand::EntityRef(u32)`**（数値 id）——`cue_target_of` が Shell/Balloon 振り分けの正本（`areka_sakura::contract`）。**SurfaceArg の alias/id 解釈は seriko 側の責務**（sakura brief で確定済み・sakura は不透明転写）。
- **emo-compose ✅（合成入力契約の正本・実シンボル）**: `EmoWorld::build(&Shell)`＋`AliasMap(BTreeMap<String, Vec<u32>>)`（kero.surface.alias 系の正規化）／`Composer::compose_into(&mut ComposedSurface, &EmoWorld, &AtlasTable, surface_id: u32, active_binds: &BindSet)`／`BindSet::from_ids`（animation ID 整列済み集合・Send）。
- **shell-parse ✅**: `Shell` モデル（alias 定義・animation/bind 定義の転記）。
- **areka-actor ✅**: アクター規約（spawn_actor/run_inbox/Close 停止・全 Sender drop 正常終了）。
- **emo-present（並走中）**: 表示指令 API（surface id＋bind 集合を運べる形・メッセージ enum 転写可能形）を同 brief が所有——seriko はその**呼び手（消費者）**。

## Desired Outcome

per-scope の surface 状態（`\s` 指令適用・`\s[-1]` 非表示・alias/name→id 解決）＋静的 bind 集合（shell descript の bindgroup default）を所有し、**emo への表示指令（scope, surface_id, BindSet）を発行する** actor（`SurfaceSink` 実装）。

**✔ 観測（単一 pass/fail）**: fixture の `TalkCue` 列（`EntityRef`・`Emote`（alias 文字列）・`-1` 相当を含む）を直入力し、**mock emo sink への発行列（scope, surface_id, binds, 非表示遷移）が期待一致**（決定論・表示不要・sleep 不使用）。emo2 fixture の alias 実データで解決を追験。

## Approach

1. **`SurfaceSink` 実装 actor**: areka-actor 規約で独立スレッド・inbox は `TalkCue`（＋Close）。sakura の sink trait をそのまま実装（契約再定義しない）。
2. **alias/name→id 解決**: `Emote{key}` の文字列を surface id へ解決。**正本は emo-compose の `AliasMap`**（`EmoWorld` 経由 or 構築時に解決表を受領——二重定義しない・所有形は design 判断）。ukadoc: `surfaces.txt` の `surface.alias` ブレスと surface `name` 定義は**同様に扱う**（両方引く）。解決不能キーは error! ＋ 指令 skip（ログ規律）。
3. **per-scope 状態**: `ActorKey`（"0"/"1"…）→ 現 surface id の写像。`\s[-1]`（`EntityRef` の -1 表現は sakura の転写形を design で確認）＝**非表示**状態。
4. **静的 BindSet**: shell descript の `bindgroupN.default` から起動時一度だけ解決（`BindSet::from_ids`）。**動的切替は M-mayuna（`mayuna-compose`）の領分**——本ユニットは口（bind 状態の置き場）だけ持つ。
5. **emo への出力**: emo-present の表示指令 API 形（id＋binds・`Send` 所有データ）に合わせて発行。**単体観測は mock sink trait**（本ユニット定義のテスト用 trait）で切る＝emo-present 完了を待たない。

## クロスユニット契約（後続を詰ませない事前考慮）

- **emo-present との surface 指令契約（対向・並走中）**: API 形の正本は emo-present brief（「指令 API」節）。着手時に同節を読み整合すること。**調整点: `\s[-1]` 非表示の表現**——表示 API に「非表示」の意味論があるか両 design で突合（seriko 側は非表示遷移を発行できる必要がある。ukadoc `\s[ID番号]` に「\s[-1]で非表示サーフェス」と明記）。
- **M-life `seriko-loop` へのシーム**: SERIKO interval エンジン（blink 等）は本ユニット皆無。ただし「アニメ再生が surface 状態と合成指令に割り込む」将来形を想定し、**状態→合成指令の発行点を単一関数に集約**（ループが後から同じ発行点を叩ける形）。
- **M-mayuna `mayuna-compose` へのシーム**: bind 状態の置き場（BindSet 保持）を per-scope 状態と同居させ、動的切替 unit が置き場だけ差し替えられる形。
- **ghost-setup（並走中）との結線**: 本ユニットの actor は sakura dispatcher の sink 差し込み口（`SurfaceSink` trait 結線）に挿さる——trait 実装であること自体が結線契約（追加の口は不要）。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc` で正典参照・2026-07-05 確認済み）

- **`ukadoc:list_sakura_script` の `\s[ID番号]`**（確認済み要点: 現スコープ側の surface 変更・**`\s[-1]` で非表示**・**surfaces.txt の `surface.alias` または `name` で定義された文字列を ID の代わりに使用可**）。`\p[ID]` スコープはsakura が `ActorKey` へ転写済み（seriko は受けるだけ）。
- **`descript_shell_surfaces` の `surface.alias` ブレスと `name,定義名`**（name は「surface.alias ブレスと同様に扱われる」と明記——**両方を同一解決表で引く**）。
- **`descript_shell` の bindgroup 系（MAYUNA）**: `bindgroupN.default` の解決規則（design 冒頭で get_doc・emo2 実測値を検証行に）。
- **brief 未網羅→design で埋める**: ① alias が複数 id を持つ場合（`AliasMap` の `Vec<u32>`）の選択規則（ランダム？先頭？——SSP de-facto を確認）② `\s` の即時性（wait 中の到着順は sakura の `at` 秒が正本＝seriko は到着順適用で可か）。

## Scope

- **In**: `SurfaceSink` 実装 actor／alias・name→id 解決（AliasMap 正本消費）／per-scope surface 状態（非表示含む）／静的 BindSet 解決（bindgroup default）／emo への表示指令発行（mock 観測）／発行点の単一関数集約（loop シーム）。
- **Out**: SERIKO ループ・interval・blink（**seriko-loop**・M-life）／bind 動的切替（**mayuna-compose**・M-mayuna）／surface 合成（**emo-compose**✅）／表示・AlphaMask（**emo-present**）／collision（**collision-geometry**）／`\i[ID]` アニメ再生指令（M-life）。

## Boundary Candidates

- 解決層（alias/name→id・純粋・単体テスト可）／状態層（per-scope・BindSet）／発行層（emo 指令・単一関数）の三片。

## Out of Boundary

- 表示の実体（emo-present）・合成結果の正しさ（emo-compose の golden が担保）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-sakura-engine` ✅（TalkCue/SurfaceSink 正本）／`completed/areka-P0-emo-compose` ✅（AliasMap/BindSet/compose 正本）／`completed/areka-P0-shell-parse` ✅（Shell モデル）／`completed/areka-P0-actor-foundation` ✅。
- **Downstream**: `areka-P0-ghost-setup`（dispatcher の sink 結線）／`seriko-loop`・`mayuna-compose`（増分）／M-boot 統合（emo2-boot）。

## Existing Spec Touchpoints

- **Extends**: なし（新設クレート）。
- **Adjacent**: `areka-P0-emo-present`（**並走**・表示指令 API の対向＝契約は emo-present 正本・`\s[-1]` 突合）／`areka-P0-ghost-setup`（**並走**・sink trait 結線のみ＝非衝突）。

## Constraints

- Rust 2024・tokio 禁止・新設 `crates/areka-seriko`（非衝突）。依存: areka-sakura（contract）・areka-emo-compose・areka-parsers・areka-actor・tracing（上向き import 禁止・wintf 非依存）。
- **決定論的テスト網羅**（開発者指示・記憶 deterministic-test-coverage-mandate）: 指令適用・解決失敗ログ・非表示遷移・Close 停止まで実行テストで回帰檻化。sleep 不使用。
- ログ規律: 解決不能 alias・未知 variant は error!/warn!＋skip（silent failure 禁止）。
- 正典は ukadoc・emo2 fixture は最小適合サンプル。
