# Design Validation: areka-P0-sakura-dialogue-tags

> **フェーズ**: design 検証（kiro-validate-design・2026-07-18・非対話実行）
> **入力**: spec.json（ja / design-generated）・requirements.md（確定）・design.md・research.md（design 追記込み）・brief.md・steering 一式
> **検証方法**: design の載荷点主張を実コード（origin/main 同一のワークツリー）へ全件突合せ（下記「検証証跡」）

## 設計レビューサマリ

settled cue モデルへの additive 拡張として、5 裁定（案C 配送・`ResolveChoice` の口・スナップショット消費・正典既定 basepos・`\!` 汎用キャリア）を全て忠実に設計へ写像しており、typed per-command variant の再発明・第二ストア・第二 relevance 機構のいずれも混入していない。design が引用する file:line の載荷点主張（barrier 先行判定・Choice 先積み分離・2 スロット構造・seriko catch-all 吸収・物理 px 契約・lexer の空トークン保持・網羅 match の波及先）を実コードで全件再現でき、事実誤認は檻 1 本の「無改変」表現のみだった。R9 の全受入基準（9.3b/9.7/9.8 含む）が具体テスト設計へ写像済みであり、実装可能な状態にある。

## Critical Issues（2 件・いずれも GO を妨げない精度改善）

🔴 **Critical Issue 1**: R8.1 檻「無改変で緑」の過大表現
**Concern**: design は `command.rs:462-507` の既存 8 variant ワイヤ檻が「無改変で緑」（Modified Files・Data Models・D4）と主張するが、同檻自身が `CueCommand::Choice { id, text }` を構造体リテラルで構築しており（`command.rs:473-476` 実測）、`references` フィールド追加後は E0063 でコンパイル不能＝**構築行 1 箇所の機械的追随（`references: vec![]`）が必須**。
**Impact**: 「意図的更新と非退行を対で峻別する」という本 spec 自身のリスク管理（リスク表 3 行目）の帳簿が 1 箇所ずれる。実装時に「檻が赤い＝設計の嘘？」という混乱や、檻の誤った書き換えを誘発しうる。
**Suggestion**: 表現を「**期待 JSON リテラル不変**（ワイヤ形バイト同一・構築行のみ機械的追随）」へ統一する（research.md D4 の「檻リテラルは不変のまま緑」が正確な表現。design.md 側 2 箇所をこれへ揃える）。ワイヤ形バイト同一の実質（serialize/deserialize とも不変）は検証済みで正しい。
**Traceability**: R8.1
**Evidence**: design.md「Modified Files」dola/command.rs 項・「Data Models」互換性列・D1/D4 ↔ 実測 `crates/dola/src/cue/command.rs:473-476`

🔴 **Critical Issue 2**: D6 書き換え時の `pending_choices` 重複積みの明文化
**Concern**: 現行 `CuePlayer::tick` は bag への Choice 積みを**配送ゲート（`remaining` 減少判定）の外**で行う（`runtime.rs:193-216` 実測）。同一時刻の冪等再 tick では schedule の ready buffer が据え置かれるため（`schedule.rs:166-168`）、生 `CuePlayer` 利用では bag へ同一選択肢が重複積みされ得る。drive 層は `last_tick` 冪等ガードで実害を遮断済みだが、D6 はまさにこのアームを書き換える。
**Impact**: 案C で bag は id 照合専用（`any` 照合）ゆえ観測実害は小さいが、新設する「バッグ並存檻」（R8.6/9.7）が tick 列に依存した bag サイズを観測すると檻が脆くなる。
**Suggestion**: D6 の実装時に bag 積みを配送ゲートと同じ条件内へ移す（または重複排除を明記）し、「bag 内容は tick 列に不変」を並存檻の assert に含める。1 行の設計注記で足りる。
**Traceability**: R8.6, R9.7（バッグ並存檻）
**Evidence**: design.md D6／「choice 配送＝案C」 ↔ 実測 `crates/dola/src/cue/runtime.rs:191-216`・`crates/dola/src/cue/schedule.rs:159-190`

## Design Strengths

1. **裁定の忠実な写像と檻の対置換規律**: 5 裁定が全て構造へ落ちている——(a) R8.6 案C は `runtime_test.rs:156-163` の先積み一択檻を配送列檻＋バッグ並存檻へ**対で置換**（削除でない）、(b) R2.7 は実在の `#[non_exhaustive] SakuraMsg`（`contract.rs:23-34` 実測・3 アーム）への additive アーム＋D7 の即時 settle で R-5（一 tick 遅延）まで解消、(c) R4/R8.7 は `Custom` 再利用＋正準コンストラクタ/抽出子＋`command_target_of` 単一権威（`Option` 戻り値が「1 名前=高々 1 消費者」を構造保証）で typed variant 新設ゼロ、(d) R5.2 は `BaseposResolver` シーム＋追跡 spec、(e) R7 は provider 差替シーム。先送りは全て「完全語彙保持＋縮退＋追跡 spec＋対応表」の 4 点セットに則る。
2. **編集面規律の実測担保と正直な緊張の記録**: 「kanade/talk/parsers/seriko＝0 ファイル」は実測で成立する——seriko は relevance 枝＋catch-all（`actor.rs:200-230, 279-286` 実測）が Cursor（Balloon 分類）と Custom キャリア（None 分類）を良性吸収し、`CueTarget::Window` 追加の網羅 match 波及は `spine_e2e_test.rs:434-440` のみ（grep 全件確認）、parser は必要データ（references・空トークン保持=lexer `scan_bracket_args` 実測・`GenericCommand{name,raw_args}`）を既に転記済み。D8 は R7.3 の「`StartTalk` 経由」文言との緊張を隠さず「構造体でなく talk 起動境界」と明記し、理由（kanade 構築点への波及＝W1 併走規律）と sylphya 着地時の再検討申し送りを残した——実質（per-talk 凍結像・値源非所有・差替シーム）は dispatcher 刻印点で満たされる。

## 検証証跡（実コード突合せの要点）

| design の主張 | 実測結果 |
|---|---|
| barrier 判定が完了判定より先（R2.3 構造充足） | ✅ `runtime.rs:226-248`（barrier match→return が `is_completed` より前） |
| Choice 先積み分離＝意図的更新の対象 | ✅ `runtime.rs:191-203`・檻 `runtime_test.rs:156-163` |
| `resolve_choice` 実装済み＋解決後 settle | ✅ `runtime.rs:279-293, 315-319` |
| `SakuraMsg` 3 種・`#[non_exhaustive]` | ✅ `contract.rs:23-34` |
| `spawn_talk` 2 sink 固定・`TalkDriver` は Vec 保持 | ✅ `drive.rs:65-92, 126-134` |
| `GhostBootOptions` 2 固定スロット（generic S,T） | ✅ `areka-ghost/src/runtime.rs:65-79` |
| `move_window_to` 物理 px 素通し・BalloonFollow 随伴・warn+false・dead_code | ✅ `follow.rs:490-519` |
| `Anchored`（ドラッグ確定系）実在＝R9.5 構造檻の対象 | ✅ `follow.rs:212` |
| seriko は catch-all/relevance 枝が新 cue を吸収（0 ファイル） | ✅ `areka-seriko/src/actor.rs:200-230, 279-286` |
| emo-text 網羅 match（catch-all なし）＝Cursor アーム強制 | ✅ `state.rs:224-244` |
| compile catch-all＋除外檻＋ClearAll 前置 | ✅ `compile.rs:117-134, 511-544` |
| 既存 8 variant ワイヤ檻＝JSON リテラル固定 | ✅ `command.rs:462-507`（※Issue 1 の構築行のみ機械的追随要） |
| fixture move の空トークン保持（6 トークン） | ✅ lexer `scan_bracket_args`（`,` で空 `cur` も push）＋`decode_bang`（skip(1)） |
| `CueTarget::Window` の網羅 match 波及＝spine_e2e のみ | ✅ workspace grep 全件確認（seriko は `Some(other)` catch-all） |
| R9 全基準のテスト写像（9.1〜9.8・9.3b 含む） | ✅ Traceability 表＋Testing Strategy に全件対応 |

## Final Assessment

**Decision: GO**

**Rationale**: settled モデルへの整合を実コードの載荷点全件で確認でき、5 裁定・ワイヤ互換（バイト同一）・W1 編集面規律・座標系（物理 px 一元＋実 DPI サインオフ）・R9 全網羅のいずれにも構造的欠陥がない。指摘 2 件はいずれも表現精度／実装時注記の水準であり、アーキテクチャ変更を要さない。

**Next Steps**:
1. 設計ディスカッションで Issue 1（檻表現の是正）・Issue 2（D6 の bag 積みゲート注記）を design.md へ反映（軽微・数行）。
2. `/kiro-spec-tasks areka-P0-sakura-dialogue-tags` でタスク生成へ進む。
3. 実装時の最重要リスクは引き続き R-6（座標系）——D11 の「`WindowPos` のみを源とする物理 px 一元」を崩さず、R9.6 の実 DPI 実機サインオフを DoD から外さないこと。
