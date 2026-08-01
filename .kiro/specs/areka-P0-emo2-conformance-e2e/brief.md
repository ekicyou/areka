# Brief: areka-P0-emo2-conformance-e2e

> **種別**: 本坑（main）・⓪ ghost 帰属の**全エンジン統合点＝M1 完成宣言ユニット**（アプリ組み上げ三段の第三段・M-e2e）。roadmap「M-e2e ＝ 全エンジン統合・boot→talk→touch→menu→close 一周適合・M1 ゴール充足」の brief 化。
> **調査日**: 2026-07-16（再入精査⑧・体裁フェーズ棚卸）。
> **🧭 ゴール裁定を収録**: 本 brief が **M-dual（dual-surface／dual-window）の吸収・退役の正本**（2026-07-16 裁定）——M-dual は 2026-07-16 実査で「大半 M-boot 充足済み・残作業は検証と gap-fill に縮退」と確定済み（roadmap 統合点行）。独立 spec を起こさず、**本ユニットの適合検証項目として消化**する。
> **⛔ 着手ゲート**: M1 残ユニットの全完了後（最終ユニット）。ただし**適合表の維持は今から**（下記チェックリストが M1 の「迷いの発生しないゴール」の単一定義）。

> **📌 2026-08-01 追記(58)棚卸更新（棚卸⑤・本ブロックが(52)㊹より優先）**:
> - **W5 は 3/4 着地**（choice-select-events✅・kero-balloon✅・dpi-window-vanish✅）・残＝collision-dpi-hittest は **W6 へ編入**。残ウェーブ改訂＝**W6（col ∥ vis ∥ bind ∥ zorder ∥ scg の5本）→ W6.5（exact ∥ wpl）→ W6.75（budget ∥ atom+bod〔縮退時統合〕）→ W6.9（cage）→ W7=本 spec**（正本は roadmap 追記(58)）。
> - **上流列へ追補 5 本**: `ghost-window-zorder`（バルーン埋もれ＝一周走行の可視性前提）・`scope-chain-gap`（P2 幅差隙間・SSP 実測正典）・`windowposition-limit`（バルーン画面外はみ出し）・`recompose-budget`（アイドル CPU 税＝e2e 実機走行の観測品質前提）・`dpi-transition-atomicity`（適合 #1 DPI 検証時の跳ね解消・+36px 追随）。
> - アンカー: spawn.rs `GhostWindows` :115 → **:164**・`ScopeWindows` :101 → **:150**（van の despawn hook 挿入）。target_map.rs `shell_target` :19 不変。
>
> **📌 2026-07-31 追記(52)棚卸更新（W4 完走・本ブロックが㊹以下より優先）**:
> - **completed 追補（㊹以降）**: wintf-gpu-test-crash（割込）✅・position-persist✅・choice-interact✅・emo-dpi-scaling✅＝**W4 完走**。**残ウェーブ改訂**: W5（dpi-window-vanish ∥ collision-dpi-hittest ∥ choice-select-events ∥ kero-balloon）→ **W6（balloon-visibility ∥ bindoption-exclusivity の2本・追記(52)裁定）**→ **W6.5（scale-exact-rational ∥ test-cage-determinism・追記(51)起票）**→ W7=本 spec。
> - **上流列へ追補3本**: `bindoption-exclusivity`（表情固着バグ＝**適合 #3「着せ替え表情」の前提充足**・bindoption 3値正典準拠）・`scale-exact-rational`（**適合 #1 の DPI 検証を絶対値で書ける前提**＝画素演算の有理数化）・`test-cage-determinism`（M1 宣言を支える檻の決定性）。
> - **着手時義務**: 本 brief の全面再監査（追記㊹時点で唯一補正無しだった経緯・調査日 2026-07-16 の実測は全面陳腐化前提で読む）・適合表へバルーン表示ライフサイクル項目追補・㉘(E)「OnFirstBoot 限定 `\![move]` の2回目起動蒸発は許容仕様」の実機判断・#7（冒頭空行＝pasta 上流未解決）は M1 完成を妨げない扱いの確認。
> - アンカー: spawn.rs `GhostWindows` :109-130 → **:115**（`ScopeWindows` :101）・target_map.rs `shell_target` :19 不変。

> **📌 2026-07-24 追記㊹棚卸更新（本ブロックが以下の本文より優先・調査日 2026-07-16 の残ユニット認識は失効）**:
> - **completed 済み**: cue-playback-duration・mayuna-compose・seriko-loop・sakura-dialogue-tags・choice-render・input-events・idle-talk・collision-geometry・sylphya（本文の「実装中/並走中」記述は全て過去形へ読み替え）。**残ウェーブ**: 割込 `wintf-gpu-test-crash`（DoD ゲート復旧）→ W4（position-persist ∥ choice-interact ∥ emo-dpi-scaling）→ W5（dpi-window-vanish ∥ collision-dpi-hittest ∥ choice-select-events ∥ kero-balloon）→ W6（balloon-visibility）→ **W7=本 spec**。
> - **上流列へ追補4本**: `choice-interact`（choice-render 2分割の対話半分・**`ChoiceSelection` 正本**＝適合 #7 hover の対話面と #8 の供給元）・`balloon-visibility`（M1 編入裁可済＝**本 spec 着手時に適合表へ「バルーン表示ライフサイクル」項目を追補**・roadmap W7 行登記済み）・`kero-balloon`（**#10 kero 一式の前提充足**＝kero が `balloonk*` 正典資産で表示・placement 採寸 scope 別）・`sylphya`✅（%username 実導出）。#1 の DPI 検証は DPI 追従込みへ格上げ（追記㉟裁定）。
> - アンカー微修正: spawn.rs の `GhostWindows` は **:109-130**（`ScopeWindows` :95-100）・target_map.rs:19-38 は不変。

## Problem

M1 ゴール「emo2 が**そのまま** boot→talk→touch→menu→close まで E2E 実走する」を**証明する仕様が無所属**。各ユニットは自分の観測（決定論檻＋個別実機サインオフ）を持つが:

- **一周を貫く適合走行**（起動→自発会話→撫で反応→メニュー一周→位置調整→終了挨拶→clean exit）を単一の pass/fail として持つ檻・手順・記録が無い。
- **M-dual の残作業（kero 側検証）が宙に浮く**: kero 窓・バルーン窓は spawn 済み（`GhostWindows`＝scope 毎 char+balloon の2窓・`spawn.rs:88-123`）・target 採番偶奇（`target_map.rs:19-38`）・kero alias 解決（seriko ✅・`surfaces.txt:458-507` の `通常,[2100]`〜`ジト,[2110,2210]`）・`\p[n]`/`\1` 交替は R9.3 実機動作——だが「**kero 側の一式が揃って正しい**」（バルーン `balloonk0` 表示・kero 撫で・kero まばたき・kero 位置調整）を誰も束ねて観測していない。
- 「M1 完成」の宣言基準（DoD）が分散している（workspace テスト・License Gate・各所実機サインオフ）——**M2 再構築の起点**となる完成宣言はここで一本化する。

## Current State（2026-07-16 棚卸）

- **完了済みの土台**: M-boot 23/23 ✅（起動→OnBoot talk→close 握手・決定論 spine ＋実機 R9.3）・ghost-setup の spine e2e（`ScriptedShioriBackend`＋`RecordingSink`・S1〜S6）が決定論 conformance 走行の**拡張母体**。
- **残ユニット（本 spec の上流・2026-07-16 時点）**: `cue-playback-duration`（実装中）→`mayuna-compose`／`seriko-loop`／`sakura-dialogue-tags`→`choice-render`／`choice-select-events`＋並走中の `position-persist`／`idle-talk`／`collision-geometry`／`input-events`。**全完了で本 spec が解禁**。
- **fixture の実全体像**（2026-07-16 実査）: dic ハンドラ＝OnFirstBoot/OnBoot(Lua)/OnClose×3/OnTalk×9(内部)/時報(内部)/OnMouseMove(撫で16シーン・**Head1/Bust1＝kero 側撫でも実在**)/OnMouseDoubleClick(メニュー)/OnUpdate*4/OnBalloonChange。バルーン fixture＝balloons0/balloonk0＋arrow0/1・marker・online0-3・balloonc1-4・sstp*。

## Desired Outcome

**M1 の完成が単一の適合走行で証明され、開発者が「M1 完了」を宣言できる。**

**✔ 観測（単一 pass/fail・二層）**:
- **(a) 決定論 conformance spine（CI 常設）**: `ScriptedShioriBackend` 拡張＝boot→（Tick 注入で）自発 talk→（注入 MouseMove 列で）撫で GET→（注入 DoubleClick→ChoiceSelection で）メニュー一周→（`\![move]` cue）→OnClose 握手→clean exit、の**全 SHIORI 交信列と全表示指令列が期待一致**（sleep 不使用・実 pasta 不要）。
- **(b) 実機一周適合（人間サインオフ・M1 完成宣言）**: 実 emo2・実 pasta.dll・実 DPI（≠96）・絶対パス起動で、下記**適合検証項目表**を一周で目視確認し、記録（acceptance-record.md）に残す。

## 適合検証項目表（M1 ゴールの単一定義・迷いの発生しないゴール）

| # | 項目 | 由来ユニット |
|---|---|---|
| 1 | 起動: 実 surface 表示（first `\s` まで非表示→表示）・既定位置（右下・相方は左）・DPI 正 | emo2-boot✅/window-placement✅ |
| 2 | OnBoot 挨拶 talk が typewriter＋正しい wait/改行/表情同期で再生 | cue-playback |
| 3 | 着せ替え表情（`\![bind]`）でむらさきの表情が変わる | mayuna-compose |
| 4 | まばたき2系統（むらさき bind+random／エモ random） | seriko-loop |
| 5 | 放置で自発会話（時報系）・talk 中は割込まない | idle-talk |
| 6 | 撫で: Head/Bust ストローク→touch 反応（**sakura 側＋kero 側の両方**＝Head0/Bust0/Head1/Bust1） | collision-geometry＋input-events |
| 7 | ダブルクリック→メニュー表示（選択肢・字下げ・hover 反転） | input-events＋sakura-dialogue-tags＋choice-render |
| 8 | 選択→シーン遷移→サブメニュー→もどる→閉じる（一周） | choice-select-events |
| 9 | エモの位置調整（`\![move,-353,...]`・boot 時＋メニュー発火時） | sakura-dialogue-tags |
| 10 | **二人立ち総合（旧 M-dual の吸収先）**: kero 窓＋kero バルーン（balloonk0）表示・`\1`/`\p[n]` 交替・kero alias 表情（`\s[ジト]` 等）・両バルーン追従/単独ドラッグ | M-boot 充足＋本 spec 検証 |
| 11 | `%username` が展開されて表示（生文字列露出なし） | sakura-dialogue-tags |
| 12 | 位置永続化: 窓を動かして終了→再起動で復元・OnFirstBoot は初回のみ | position-persist |
| 13 | 終了: メニュー or 退避手段→OnClose 挨拶→`\-`→clean exit（stand-in despawn 不使用） | input-events＋kanade✅ |
| 14 | **省略可項目の縮退確認**（gap 監査）: OnUpdate*/OnBalloonChange＝未送出（M2）・arrow/marker/online/balloonc/sstp＝未描画で破綻なし・OnChoiceTimeout の裁定どおりの挙動 | 本 spec |

## Approach

1. **決定論 spine の拡張**（ghost-setup S1〜S6 の母体を conformance 台本へ）: scripted backend に「自発 talk 応答・撫で応答・メニュー script・選択遷移 script・move 込み script」を追加し、**入力注入（Tick/Mouse/ChoiceSelection）→SHIORI 交信列→表示指令列**の全経路を1本の統合テストに固定（CI 常設・実 pasta 非依存）。
2. **実機適合走行の手順書＋記録**: 上記14項目のチェックリストを acceptance-record.md 様式で固定（window-placement の前例に倣う）。実 DPI 120/192 級・マルチモニタ跨ぎを含む。
3. **gap-fill の規律（escape hatch）**: 適合走行で発見された欠陥は**本 spec で直さない**——症状を仕分けし、小さければ「この場修正」・構造的なら**個別 spec を just-in-time で切って先に完遂**（emo2-boot R9.3 の実機7件仕分けと同じ運用・spec 工場回避のまま）。本 spec は「証明」に徹する。
4. **M1 完成宣言の DoD 一本化**: `cargo test --workspace` exit 0（i686 host-32 成果物ビルド後・[[workspace-test-needs-i686-host32-artifacts]]）＋ License Gate（cargo deny＋cargo about・kiro-complete DoD 統合済み）＋ 実機14項目サインオフ（開発者・人間判断）＝**M1 完成**。完了時に roadmap の M1 節を閉じ、M2 再構築（「実物を見て組み直す」）の起点を宣言する。

## クロスユニット契約（2026-07-16）

- **M-dual の吸収・退役（本 brief が正本）**: roadmap 増分の `areka-P0-dual-surface`（⑤）・`areka-P0-dual-window`（⑥）は**ユニット名を退役**——実体（kero 窓 spawn・target 偶奇・alias 解決・`\p` 交替）は M-boot で充足済みであり、残る「kero 側の束ねた検証」は項目 #10 が所有。検証で構造的 gap が出た場合のみ、その症状に対する個別 spec を just-in-time で切る（旧名を復活させない）。
- **各ユニットの実機サインオフとの関係**: 各上流ユニットは自分の症状の実機確認を済ませて完了する（判定は各 spec 帰属）——本 spec は**相互作用と一周**（例: 撫で talk 中にメニュー・選択待ち中の自発 talk 抑止=Status: choosing）を観測対象とする。**判定を混ぜない**規律は idle-talk brief と同型。
- **決定論 spine の資産系譜**: ghost-setup `ScriptedShioriBackend`／emo2-boot spine／kanade 統合テストの拡張であり**新しいテスト機構を発明しない**。
- **画家則の適合範囲（2026-07-17 合流裁定で登記・collision-geometry research §10.6 の申し送り受領）**: collision 重なり優先は emo 合成規約＝**画家のアルゴリズム**（後定義が手前）で SSP `collision-sort`（既定 none＝先書き手前）とは**逆向き**（collision-geometry 議題1裁定）。emo2 fixture には重なり collision も `collision-sort` 宣言も無く、本 spec の適合走行はこの逸脱を**検出しない**——本 spec が証明するのは「**emo2 適合**」であって「SSP 完全適合」ではない。
- **`\![move]` の2回目起動挙動（2026-07-17 合流裁定 E の申し送り・項目 #9/#12 の相互作用）**: `\![move]` は永続値を書かない（position-persist R1.9 二層分離・sakura-dialogue-tags brief が正本）。emo2 の `\![move]` は OnFirstBoot 限定ゆえ、初回ゲート（#12）導入後は**未ドラッグの2回目起動で初回位置調整が既定配置へ戻る**＝許容仕様として裁定済みだが、**適合走行時に開発者の実機判断で最終確定**すること（違和感があれば just-in-time の個別 spec で扱う）。

## ukadoc 必読（design 着手時に ukadoc MCP で正典参照）

- `list_shiori_event` の boot/close/入力/選択の各イベント（各上流 brief で裏取り済み・本 spec は**交信列の順序**を総覧で再確認）。
- **`Status` ヘッダ 9種**（scope doc §1）——一周中の遷移（talking/choosing）が交信列に正しく現れることを spine の檻に。
- OnUpdate*/OnBalloonChange が**任意（M2）**である根拠の最終確認（項目 #14）。

## Scope

- **In**: 決定論 conformance spine（1本・CI 常設）／実機14項目の適合走行手順＋記録様式／gap 仕分け運用／M1 完成 DoD の一本化と宣言／M-dual 検証吸収（#10）。
- **Out**: 発見欠陥の修正そのもの（just-in-time 個別 spec へ）／新機能・新 UI（全て上流ユニット）／里々/YAYA・Shift_JIS 等の生態系拡張（M1 後・scope doc §7）／NAR インストーラ・選択 UI・SSTP/FMO/Plugin/更新（M2 予約）。

## Boundary Candidates

- spine 台本（scripted backend 拡張＝決定論）／実機手順・記録（人間判断の様式化）／DoD ゲート（機械検証＋宣言）。

## Out of Boundary

- 各エンジンの内部品質（各 spec の檻が正本）——本 spec は結合と一周のみ。

## Upstream / Downstream

- **Upstream（全部）**: `cue-playback-duration`→`mayuna-compose`・`seriko-loop`・`sakura-dialogue-tags`→`choice-render`・`choice-select-events`／`position-persist`・`idle-talk`・`collision-geometry`・`input-events`／完了済み全ユニット（M-boot 23）。
- **Downstream**: **M2 ロードマップ再構築**（M1 完成宣言が起点・「実物を見て組み直す」）／生態系拡張（里々/YAYA・Shift_JIS・SAORI）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-ghost-setup`（spine e2e 母体）・`completed/areka-P0-emo2-boot`（spine＋実機サインオフ様式）。
- **Supersedes（吸収・退役）**: roadmap 増分の `areka-P0-dual-surface`・`areka-P0-dual-window`（M-dual→検証項目 #10 へ縮退・2026-07-16 裁定）。
- **Adjacent**: `doc/emo2-conformance-scope.md`（M1 実物定義の正本——本 spec 完了時に「充足済み」注記で閉じる。**§1 の OnChoiceSelectEx Ref0 記述の訂正**は choice-select-events design が実施）。

## Constraints

- Rust 2024・新規依存なし・tokio 不使用。
- **決定論 spine は実 pasta 非依存**（scripted backend・注入入力のみ・[[deterministic-test-coverage-mandate]]）／**実機走行は実 pasta・実 DPI・絶対パス起動**（[[areka-placement-real-ghost-first]]・MOD_NOT_FOUND 運用注意）。
- DoD: `cargo test --workspace` exit 0（[[workspace-test-needs-i686-host32-artifacts]]）＋License Gate＋実機14項目（人間サインオフ・AI 単独で完成宣言しない）。
- 正典は ukadoc・emo2 は最小適合 fixture（[[ukadoc-mcp-preferred-source]]）。
