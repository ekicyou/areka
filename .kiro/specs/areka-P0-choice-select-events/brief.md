# Brief: areka-P0-choice-select-events

> **種別**: 本坑（main）増分。③ kanade 帰属（M-dialogue の完成ユニット＝選択確定→SHIORI→次シーン）。`areka-P0-input-events` brief が明示分離した「OnChoiceSelectEx は choice-render 完了後の増分」の**宛先**（2026-07-16 名称確定・input-events の「マウス入力→kanade→GET→StartTalk」背骨を再利用）。
> **調査日**: 2026-07-16（再入精査⑧・fixture 実物調査＋コード実態偵察＋ukadoc 裏取り）。
> **順序（フェーズ別・2026-07-16 精密化）→ ✅ ゲート解除（2026-07-17・cue-playback 完了＝追記㉗）**: choice-render（ChoiceSelection 契約の正本）と契約先決ペア——契約が brief 間で先決済みのため**並走可**（撫でクラスタと同型）。~~cue-playback（CuePlayer/resolve_choice）完了が tasks 生成・実装フェーズの実質前提~~ **→充足済み**（`CuePlayer`/`pending_choices`/`resolve_choice` は settled シームとして main 着地済み）。着手時は settled コードを直接参照すればよい（設計を先行させた場合の `/kiro-validate-design` 再突合義務は 2026-07-17 現在未発生＝要件未着手）。**実装順注記（2026-07-17 合流裁定・roadmap 追記㉘）**: 本 spec は input-events の背骨＋idle-talk の `Status` 口＋dialogue-tags の choice cue 形＋choice-render の ChoiceSelection を消費する**実質最後尾**ユニット。

## Problem

選択肢を**選んだ後**の経路が存在しない。emo2 のメニューは選択→`＊On〜` シーン遷移→もどる/閉じる、の循環で成立するが:

- 選択確定を SHIORI へ届けるコードが**ゼロ**（workspace grep「OnChoiceSelect」＝production Rust 0 件・実測 2026-07-16）。
- kanade の `KanadeMsg`/`Input`（`msg.rs:46-61`／`schedule/mod.rs:36-45`）に選択系 variant が無い。
- **emo2 は OnChoiceSelect/Ex ハンドラを一切持たない**（fixture grep 0 件）——`\q[おしゃべり頻度,Onおしゃべり頻度メニュー]`（menu.pasta:15 実物）のように **ID に `On〜` イベント名を書き、任意名イベントの直接発火**に依存している。これは de-facto でなく**正典形**: ukadoc の `\q` は計6形あり、**`\q[タイトル,OnID,r0,r1...]`＝On 始まり ID は任意名イベント直接発火・r\* は Ref0 起点**が定義済み（2026-07-16 検証）。つまり**カスケード（OnChoiceSelectEx→OnChoiceSelect→OnID 直接）の正確な実装が emo2 適合の生死を分ける**。
- 選択待ち中の `Status: choosing` ヘッダ（emo2 が OnSecondChange 発火制御に使う・scope doc §1）と、選択肢タイムアウト（ukadoc: 既定でタイムアウトし OnChoiceTimeout 発火・**カウントは「トーク表示が全て終わってから」**＝duration 権威と直結）も無所属。

## Current State（2026-07-16 実装偵察）

- **kanade の再利用棚**: 応答 `Value`→`TalkId` 採番→`Action::StartTalk` の調停（`steady.rs:92-103`）・active talk 単一 slot（dispatcher）・`events.rs` の Reference 組立様式——**新しい調停を発明しない**（input-events と同じ規律）。
- **CuePlayer 側の解除口は settled（→✅ 2026-07-17 main 着地・現行実測へ更新）**: dola `CuePlayer` に `WaitingForChoice` 停止（`runtime.rs:71`・遷移 `:231-237`）と **`resolve_choice(choice_id) -> Option<String>`**（`:279-293`）が main 実装済み＝**選択確定で barrier を解いて talk を続行/終了する機構は供給済み**。本 spec はその呼び手＋SHIORI 配送。
  - **⚠️ 訂正（2026-07-18・`areka-P0-sakura-dialogue-tags` R2.7 申し送り）**: `CuePlayer::resolve_choice` は **talk アクター内に閉じており外部から直接呼べない**。解決は sakura-dialogue-tags が定義する **talk アクター境界の型付き入力（`SakuraMsg` の additive アーム）経由**でのみ到達する。ゆえに本 spec は「`resolve_choice` を直接呼ぶ」のでなく、**その `SakuraMsg` 解決アームへ選択 id を投入**する（口の形は sakura-dialogue-tags が正本・本 spec は消費）。下記 Current State/Approach の「`resolve_choice` 呼出」は全てこの口経由と読み替える。
- **入力の供給元**: choice-render の `ChoiceSelection{scope, id, label, extras}`（同 brief 正本・mock で先行観測可能）。
- **正典 layout（ukadoc 裏取り済み 2026-07-16）**: **OnChoiceSelectEx**＝Ref0=ラベル・Ref1=ID・Ref2+=拡張引数（`\q` の r2 以降）・OnChoiceSelect より**先に**発生／**OnChoiceSelect**＝Ref0=ID／**OnChoiceTimeout**＝Ref0=タイムアウトしたスクリプト。

## Desired Outcome

選択クリックが SHIORI へ**SSP 準拠のカスケード**で届き、実機で emo2 のメニューが**一周**する（ダブルクリック→メニュー→「おしゃべり頻度」→サブメニュー→「もどる」→「閉じる」）。選択待ち中は `Status: choosing` が載り、タイムアウト規律が確定する。

**✔ 観測（単一 pass/fail）**: 決定論（mock shiori・注入 ChoiceSelection・注入 Tick・sleep 不使用）＝(a) **emo2 形**: ID=`On〜` の選択→カスケード結果として任意名イベント GET が発行され Value→StartTalk（既存棚）＋`resolve_choice` 呼出 (b) **正典形**: OnChoiceSelectEx（Ref0=ラベル/Ref1=ID/Ref2+=extras）が先行し、204 時のフォールバック順序が design 確定則どおり (c) 選択待ち中の OnSecondChange に `Status: choosing`（解除で消える） (d) タイムアウト→OnChoiceTimeout（Ref0=script）→応答規律（204 で選択解除） (e) 選択後の二重発火なし（1選択=1カスケード）。＋実機＝実 emo2 でメニュー一周（頻度設定の変化は pasta 内部変数＝目視は遷移 talk で判定）。

## Approach

1. **カスケード則の確定（design 冒頭・最重要）**: ukadoc は **OnID 形（`\q[タイトル,OnID,r0,r1...]`＝On 始まり ID→任意名イベント直接発火・r\* は Ref0 起点）を正典定義済み**（2026-07-16 検証）＝emo2 の2引数 On〜 はこの形（references 空→Ref 無しで OnID を GET）。design で確定すべき残りは「**OnID 形でも OnChoiceSelectEx/無印が先行発火するか**」「非 On ID の 204 フォールバック順序」——`\*` doc の「選択時は通常通り OnChoiceSelectEx（トークがなければ OnChoiceSelect）」との整合を SSP 実挙動込みで裁定し、**判定を純関数化**して全網羅檻に。emo2 実物（OnID 直接）と正典 Ex/無印形の両方を檻で固定。
2. **kanade 増分（additive・input-events 背骨再利用）**: `Input::ChoiceSelected{selection}` variant＋`events.rs` に on_choice_select_ex/on_choice_select/任意名 GET の組立→Steady で GET 発行→応答 Value は既存 StartTalk 棚（`steady.rs:92-103`）へ。**選択で旧 talk が終わる/続く**の裁定（`resolve_choice` の戻り＝barrier 後続の有無）と StartTalk slot 調停（新 talk 差替）を design で確定。
3. **CuePlayer 連携**: ChoiceSelection→`resolve_choice(id)` 呼出（barrier 解除・後続 cue 続行）と SHIORI カスケードの**順序・排他**（選択は1回だけ有効・解除後の遅延クリックは棄却）を確定。
4. **`Status: choosing`**: idle-talk が設計するヘッダ注入 enum の口へ `choosing` を追加（kanade は選択待ち状態を choice-render の表示状態でなく**自分の配送状態**として保持＝単一真実源）。**⚠️ 消費側互換（2026-07-17 合流裁定の登記＝choosing は M1 最初の非 talking `Status` 値）**: 実 pasta の自発トーク抑制は `status == "talking"` **完全一致**（`virtual_dispatcher.lua:98,123`）ゆえ、選択待ち中に `Status: choosing`（talking 非含有）で OnSecondChange GET が届くと pasta 側抑制は掛からず OnTalk が漏れ得る。選択待ち中の自発トーク抑止は **kanade 側の調停**として design で確定すること——cue-playback の `WaitingForChoice` は talk 未完了（占有 horizon 未到達）＝kanade の active talk slot が占有継続→NOTIFY・Ref3="0" が自然形（この場合 `Status` は `talking,choosing` か `choosing` 単独かも含めて wire 形を裁定・idle-talk Req2.6 ただし書き／`status-execution-states` 台帳 2b と整合させる）。
5. **タイムアウト**: 「トーク表示が全て終わってから」のカウント開始＝**cue-playback の talk 絶対終了時刻**（duration 権威・記憶 [[areka-dola-absolute-time-sync-broadcast]] の占有 horizon）を起点に、既定値（SSP de-facto・ukadoc で確認）で OnChoiceTimeout GET→204 なら選択解除。**emo2 は未使用（ハンドラ無し・`\*` 無し・choicetimeout 設定無し）**＝M1 は「実装するが emo2 では 204 経路のみ通る」——実装 or 型シーム縮退の最終裁定は design（正典準拠を既定・工数過大なら明示縮退を記録）。

## クロスユニット契約（並走を詰ませない事前考慮・2026-07-16）

- **ChoiceSelection＝choice-render が正本**（本 spec は消費のみ・mock 注入で決定論観測が完結＝choice-render 未完でも着手可能な形）。
- **choice cue 形・barrier 並び＝sakura-dialogue-tags が正本**（`resolve_choice` の id は同契約の id と同一通貨）。
- **`Status` ヘッダの口＝idle-talk が設計**（enum 拡張のみ・再設計しない——idle-talk brief「input-events/choice-render 側が足せる形に」の宛先は本 spec に更新）。
- **input-events 背骨の再利用**: 「入力→kanade→GET→StartTalk 棚」の型（イベント種 enum 拡張余地）は input-events が先に確立——本 spec は choice アームを**足すだけ**。input-events 完了が望ましい先行（未完なら背骨ごと本 spec が持つ羽目になる＝順序推奨を roadmap に明記）。
- **タイムアウト起点＝cue-playback の talk 終了時刻**: 独自の時間計算を発明しない（duration 権威に従属・[[areka-cue-runtime-consolidated-in-dola]]）。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-16 裏取り）

- **裏取り済み**: `ukadoc:list_shiori_event:OnChoiceSelectEx:1`（Ref0=ラベル・Ref1=ID・Ref2+=拡張・「OnChoiceSelect よりも先に開始」）／`OnChoiceSelect:1`（Ref0=ID）／`OnChoiceTimeout:1`（Ref0=script）／`ukadoc:list_sakura_script:_5c_2a:1`（`\*`＝タイムアウト無効・「選択時は通常通り OnChoiceSelectEx（トークがなければ OnChoiceSelect）」）／`\![set,choicetimeout,時間]`（カウントは表示完了から・0/-1=無効）。
- **design で確定**: 任意名（OnID 形）直接発火の正確な条件——**`\q[タイトル,OnID,r0,r1...]` 形の list_sakura_script doc を必読**（On 始まり判定・r\*→Ref0 起点）＋`ukadoc:list_plugin_event:OnChoiceSelect(Ex)/OnAnchorSelect(Ex)/\q等に指定された任意名イベント` 全文＋SSP 挙動・タイムアウト既定値・204 カスケードの正確な順序（OnID 形での Ex/無印 先行有無）。**scope doc §1 の「OnChoiceSelectEx（\q[title,id] の id を Reference0）」記述は正典（Ref0=ラベル/Ref1=ID）と不一致＝design で scope doc を訂正すること**（2026-07-16 検出）。

## Scope

- **In**: カスケード判定の純関数＋全網羅檻／kanade `Input::ChoiceSelected`＋Reference 組立（Ex/無印/任意名）／既存 StartTalk 棚への合流＋slot 調停裁定／`resolve_choice` 連携（1選択=1カスケード・遅延クリック棄却）／`Status: choosing`／タイムアウト（正典準拠 or 明示縮退の裁定込み）／実機サインオフ（メニュー一周）。
- **Out**: 選択肢の表示・hover・クリック解決（choice-render）／`\q`→cue コンパイル（sakura-dialogue-tags）／`\_a` アンカー系イベント（emo2 未使用・M1 外）／OnChoiceEnter/選択肢ホイール系（M1 外）／owner-draw メニュー（M2）。

## Boundary Candidates

- カスケード判定（純関数・正典/emo2 両形の全網羅）／kanade 増分（additive アーム）／CuePlayer 連携（結線・排他規律）／タイムアウト（duration 権威消費）。

## Out of Boundary

- 選択肢 UI（契約 ChoiceSelection で分離済み）／talk 再生そのもの（cue-playback 領分）。

## Upstream / Downstream

- **Upstream**: **`areka-P0-choice-render`（ChoiceSelection 正本・契約先決で並走可）**／**`completed/areka-P0-sakura-dialogue-tags`（choice cue/id 通貨）**／**`areka-P0-cue-playback-duration`（resolve_choice・talk 終了時刻）**／`areka-P0-input-events`（背骨・順序推奨で先行）／`completed/areka-P0-kanade`（StartTalk 棚）／`areka-P0-idle-talk`（Status ヘッダの口）。
- **Downstream**: `areka-P0-emo2-conformance-e2e`（メニュー一周の適合項目＝M-dialogue 完成の証明はここ）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-kanade`（Input/events/steady の additive 増分・決定論資産不変）。
- **Adjacent**: `areka-P0-input-events`（同じ kanade events/steady を触る——マウス2イベント vs choice 1アーム＝**別アーム additive**・同時着手時は近接注意）／`areka-P0-idle-talk`（Status enum 共有・別値 additive）。
- **Consumes**: `.kiro/specs/completed/areka-P0-input-events/brief.md` の M-dialogue 申し送り（「choice 確定イベントがそのまま再利用できる形」）＝本 spec が受領。

## Constraints

- Rust 2024・新規 crates.io 依存なし・tokio 不使用。
- **決定論**: mock shiori＋注入 ChoiceSelection＋注入 Tick で全経路網羅（[[deterministic-test-coverage-mandate]]）。カスケード判定・タイムアウト起点は純関数（[[test-only-decision-branches-not-proven-wiring]]）。
- ログ規律: 未解決 id・遅延クリック棄却は warn+継続（[[areka-log-first-no-silent-failure]]）。
- **実機受け入れ**: 実 emo2・実 pasta.dll・実 DPI でメニュー一周の人間サインオフ（[[areka-placement-real-ghost-first]]）。起動は絶対パス必須（MOD_NOT_FOUND 運用注意）。
- 正典は ukadoc・emo2 は最小適合 fixture（[[ukadoc-mcp-preferred-source]]）。
