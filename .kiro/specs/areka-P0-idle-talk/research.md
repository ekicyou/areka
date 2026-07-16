# Gap Analysis: areka-P0-idle-talk

> 対象: 確定済み requirements.md（6要件）との実装ギャップ分析。
> 調査日: 2026-07-16（実装偵察・ukadoc 裏取り・emo2 fixture grep）。
> 位置づけ: 本坑 kanade 増分。背骨（Steady pump／StartTalk 経路）は既配線ゆえ「正典充足（Reference＋Status）＋回帰檻＋実機サインオフ」への再スコープ。
> 情報提供に徹する（決定でなく選択肢）。

## Analysis Summary（3-5 bullets）

- **Reference0〜3 は既に充足済み**。`crates/areka-kanade/src/schedule/events.rs:86-101` の `on_second_change(now, talk_playable)` が既に Ref0=起動時間hour・Ref1="0"・Ref2="0"・Ref3="1"/"0" を正典どおり組み立てており単体テスト済み（events.rs:166-202）。brief の疑い「Ref3 以外の充足状況が未検証」は**解消**——本 spec の Reference 作業は確認と将来シームの明示に縮小する。
- **本 spec の実体は `Status` 共通ヘッダ**。ここに**唯一の真の欠落**があり、かつ brief の前提「host 側改変なし／build_request は汎用ヘッダ対応済み」は**成り立たない**（後述の file:line 証拠で反証）。`Status` を通すには msg.rs／real.rs／client.rs／shiori3.rs の**2クレート4層**を貫通させる必要がある。
- **GET→StartTalk／NOTIFY 破棄／talk 中非割込み（Req4）は実装済み・統合テスト済み**（steady.rs＋steady_test.rs）。本 spec の Req4 は新規実装でなく回帰檻の明示・強化。
- **OnTalk/OnHour 不送出（Req3）は現状満たすが檻が無い**。events 表に OnTalk/OnHour は存在せず、kanade は構造上送れないが、ホワイトリストを固定する回帰テストが未整備。
- **決定論シームは既存**。時刻源は Tick の `now: MonotonicMs`（本番=GetTickCount64・`crates/areka-ghost/src/ticker.rs:52`）で注入可能（Req5.2）。mock shiori の `RecordedCall` は method/id/references を観測するが `Status` 観測フィールドが無い（Req5.3 のため拡張要）。

---

## 1. Current State Investigation（現状資産）

### 1.1 対象クレートと層構成

自発会話の全経路は以下の層を貫通する（kanade → host32 の2クレート）:

| 層 | ファイル | 役割 | `Status` 対応 |
|---|---|---|---|
| events 表（純粋生成） | `crates/areka-kanade/src/schedule/events.rs` | `on_second_change` が `ShioriCall` を組む単一正本 | **なし** |
| 状態機械（pump ゲート） | `crates/areka-kanade/src/schedule/steady.rs` | Steady{talk} から GET/NOTIFY を発行 | 状態は持つが Status 未導出 |
| 境界メッセージ型 | `crates/areka-kanade/src/msg.rs:80-89` | `ShioriCall::Get/Notify{id, references}` | **フィールド無し** |
| backend 抽象 | `crates/areka-kanade/src/shiori/real.rs:47-56` | `ShioriBackend::get/notify(id, references)` | **引数無し** |
| SHIORI client | `crates/shiori-host32-host/src/client.rs:115-158` | `Shiori3Client::get/notify` が `ShioriRequest` を組む | **フィールド無し** |
| wire codec | `crates/shiori-host32-host/src/shiori3.rs:58-118` | `ShioriRequest` + `build_request` | **フィールド無し・固定ヘッダ集合** |
| 観測ハーネス | `crates/areka-kanade/tests/kanade/common/mod.rs:74-100` | `RecordedCall{method,id,references}` | **観測フィールド無し** |

### 1.2 Reference 充足の実査（events.rs:86-101）

現 `on_second_change` の出力（ukadoc `OnSecondChange:1` 正典と突合）:

| Reference | ukadoc 正典 | 現実装 | 判定 |
|---|---|---|---|
| Ref0 | OS 連続起動時間（hour） | `(now.0 / 3_600_000).to_string()`（ゼロ方向切捨） | **充足**（Req1.2）。本番は GetTickCount64 注入で意味的にも正しい |
| Ref1 | 見切れ時1・他0 | `"0"` 固定 | **充足**（Req1.3・M1固定） |
| Ref2 | 重なり時1・他0 | `"0"` 固定 | **充足**（Req1.3・M1固定） |
| Ref3 | 再生可能1・他0 | `talk_playable` で `"1"`(GET)/`"0"`(NOTIFY) | **充足**（Req1.4/1.5） |

→ Req1.1〜1.5 は**実装・単体テスト済み**。残るは Req1.6（Ref1/Ref2 の将来実測差替シーム）のみ。

**Ref1/Ref2 固定0の正当化（emo2 fixture grep）**: `crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/dic/` を grep した結果、見切れ/重なり/Reference1/Reference2 を消費するハンドラは**皆無**。`hour.pasta` は「OnHour は仮想イベント・pasta.dll がランダム選択」と明記＝pasta が OnSecondChange から時報を内部駆動する構図。よって M1 の固定 "0" は fixture 挙動を毀損しない。

### 1.3 pump／調停（steady.rs）— Req4 は実装済み

`steady.rs:55-179` が DD-6 を完全実装:
- `Steady{None}` + Tick → OnSecondChange **GET**（Ref3=1・events.rs:91）
- `Steady{Some}` + Tick → OnSecondChange **NOTIFY**（Ref3=0・events.rs:96）＝再生中は発生源から Value を断つ
- `Steady{None}` + Value → StartTalk＋`Steady{Some}` 遷移（steady.rs:92-103）
- `Steady{None}` + 204 → 無起動維持（steady.rs:104-107）
- `Steady{Some}` + Value → warn!＋破棄（DD-6 防御・steady.rs:115-119）

統合テスト（`tests/kanade/steady_test.rs`）が Req4.1/4.2/4.3/4.4 を被覆済み（GET→StartTalk・204→無起動・NOTIFY 破棄・TalkDone 復帰）。

### 1.4 決定論シーム — 既存

- 時刻源: Tick の `now: MonotonicMs`（本番 GetTickCount64・テストは任意単調値）＝Req5.2 の注入点は既存。
- mock shiori: `spawn_mock_shiori`（common/mod.rs:255）が `RecordedCall` に method/id/references を蓄積。Req5.3 の「Status ヘッダの有無と値」観測には `RecordedCall` 拡張が必要。

---

## 2. Requirements Feasibility & Gap Map（要件↔資産マップ）

| 要件 | 必要能力 | 既存資産 | ギャップ種別 |
|---|---|---|---|
| Req1.1〜1.5（Ref0〜3充足） | Ref0=uptime hour・Ref1/2=0・Ref3=可否 | events.rs 実装済＋単体テスト | **充足済**（確認のみ） |
| Req1.6（Ref1/2 実測差替シーム） | 送出契約を変えず値を差替可能に | 現状は固定リテラル | **Constraint**: 窓 geometry は UI スレッド知識。kanade worker へ運ぶ Tick 付帯（TickInfo）が未存在＝口だけ設計 |
| Req2.1（再生中 `Status: talking`） | Steady{Some}→talking ヘッダ | **経路が層貫通で存在しない** | **Missing（最大）**: 2クレート4層貫通 |
| Req2.2（無再生時 talking 非送出） | Steady{None}→Status 省略 | 同上 | Missing＋**Unknown**（省略 vs 空値の SSP 挙動） |
| Req2.3（将来状態値 choosing 等の同一経路） | 拡張可能な状態表現 | 同上 | Missing（enum 化の口） |
| Req3.1〜3.3（送出 ID ホワイトリスト・OnTalk/OnHour恒久不送出） | 発行 ID 集合の固定と檻 | events 表に該当無し（構造上送れない）だが**檻無し** | **Missing（檻のみ）** |
| Req4.1〜4.4（GET→talk・204無起動・NOTIFY破棄・完了復帰） | pump 調停 | steady.rs 実装済＋統合テスト済 | **充足済**（回帰檻の明示） |
| Req5.1〜5.3（決定論検証） | 注入Tick・mock・Status観測 | Tick注入・mock 既存／Status観測欠 | **Missing（RecordedCall 拡張）** |
| Req6.1〜6.3（実機サインオフ） | 実 emo2 放置→自発会話 | 未実施 | **Missing（人手検証）** |

---

## 3. 焦点: `Status` ヘッダシーム — brief 前提の反証（file:line 証拠）

### 3.1 反証: host32 の `build_request` は汎用ヘッダ対応では**ない**

brief §Approach 3 は「ヘッダ付与は Shiori3Client の既存汎用 request 経路に乗る（host 側改変なしを design で確認・build_request は汎用ヘッダ対応済み）」と仮定する。**この前提は成り立たない**。

**証拠1 — `ShioriRequest` に status フィールドが無い**（`crates/shiori-host32-host/src/shiori3.rs:58-69`）:
```
pub struct ShioriRequest<'a> {
    pub method: Method,
    pub id: &'a str,
    pub references: &'a [String],
    pub sender: &'a str,
    pub charset: Charset,
}
```
→ status も汎用ヘッダマップも無い。

**証拠2 — `build_request` は固定ヘッダ集合を発行する**（`shiori3.rs:87-118`）: request line → `Charset` → `Sender` → `ID` → `Reference0..N` → `SecurityLevel: local` → 空行。**任意ヘッダを注入する機構が無い**（汎用ビルダとは References の連番付与のみを指し、ヘッダ集合は固定）。rustdoc（shiori3.rs:83）も「`SenderType`/`SecurityOrigin`/`X-SSTP-PassThru` は M1 最小のため送出しない」と固定集合を明示。

**証拠3 — client も status を運ばない**（`crates/shiori-host32-host/src/client.rs:116-122, 144-150`）: `get`/`notify` は `ShioriRequest{method, id, references, sender, charset}` をハードコード構築。status 引数なし。

**証拠4 — kanade 側 backend/境界型も同様**:
- `ShioriBackend::get/notify(id, references)`（`real.rs:49-51`）— ヘッダ引数無し。
- `ShioriCall::Get/Notify{id, references}`（`msg.rs:80-89`）— status フィールド無し。

### 3.2 結論

`Status` ヘッダ注入は host32 crate（`shiori-host32-host`）の**改変を必須**とする。改変は最小で以下4層＋観測を貫通:
1. `shiori3.rs`: `ShioriRequest` に `status: Option<&str>`（等）を追加し、`build_request` が `Status: <v>` 行を（Some のとき）発行。
2. `client.rs`: `Shiori3Client::get/notify` に status を通す（引数追加 or builder）。
3. `real.rs`: `ShioriBackend::get/notify` シグネチャ拡張＋`ShioriConnection` impl 転送、`handle_call`（real.rs:99-111）が status を forward。
4. `msg.rs`: `ShioriCall::Get/Notify` に `status` フィールド追加（events.rs が導出・steady.rs が Steady{talk} から供給）。
5. 観測: `RecordedCall`（common/mod.rs:74-100）に status を記録し Req5.3 を満たす。

### 3.3 `Status` の正典典拠 — ukadoc に単独ページ無し（確認済）

ukadoc MCP で「Status/talking/choosing」を2度検索したがいずれも not_found。正典典拠は `doc/emo2-conformance-scope.md` §1（line 18）:「リクエストで emo2 が読むヘッダ: ID / Reference0..n / **Status（talking/choosing/online 等9種で OnSecondChange の発火制御）** / Sender / Charset」。これが実需正本＝brief のフォールバック計画どおり。SSP が talk 無し時に Status を送るかは不明ゆえ、M1 は保守的に「active talk 中のみ talking」で開始し記録する（Req2.2）。

---

## 4. Implementation Approach Options（`Status` シーム）

`Status` を層貫通させる形をどう設計するかが本 spec の中心的判断。3案を提示する。

### Option A: `ShioriCall`／`ShioriRequest` に `status` フィールドを追加（層全体を素直に拡張）

各層の型に `status: Option<HeaderValue>`（enum）を足し、events.rs が導出→steady が供給→build_request が発行。

- **どこを触るか**: msg.rs／real.rs（trait＋impl）／client.rs／shiori3.rs／common/mod.rs（観測）／events.rs（導出）。
- **✅ 長所**: 型で status を明示・全 SHIORI 呼出に一貫適用可能（将来 choosing/online も同経路・Req2.3 に自然）。observation も型で漏れなく取れる。
- **❌ 短所**: 全 `ShioriCall::Get/Notify` 構築点・match 点が影響（boot.rs/close.rs/events.rs/mod.rs/real.rs/tests）。`ShioriCall` は derive 無し（msg.rs:80）ゆえ機械的だが箇所は多い。host32 API 破壊的変更。

### Option B: `Status` を OnSecondChange 専用に閉じた別経路で通す（最小侵襲）

`ShioriCall` を汎用に拡張せず、OnSecondChange の GET/NOTIFY にのみ status を載せる専用構築（例: events.rs で status を含む専用 variant／専用フィールド、または client に `get_with_status` を1本追加）。

- **✅ 長所**: 影響範囲を OnSecondChange 経路に局限。boot/close の既存構築点・テストを不変に保てる。
- **❌ 短所**: 「共通ヘッダ」を特定イベント専用にする＝設計思想（共通ヘッダは全リクエスト属性）と乖離。Req2.3 の「同一送出経路で choosing 等を伝える」を後で汎用化する二度手間の芽。build_request が特定イベントを知らない汎用ビルダ原則（shiori3.rs:16）と整合させる工夫が要る。

### Option C: ハイブリッド — 共通ヘッダ束を導入し、当面 status のみ実体化（口は汎用・値は最小）

`ShioriRequest`／`ShioriCall` に「共通ヘッダ束」（`status: Option<StatusValue>` を第一フィールドとする小さな構造 or enum）を1つ足し、build_request は束を順に発行。M1 は status のみ実装、将来ヘッダ（Sender 種別等）も同束へ。status 値は拡張 enum（`Talking` のみ実装・`Choosing` 等はシーム）。

- **✅ 長所**: Req2.3（拡張の口）と Req1.6（送出契約を変えず差替）を構造で満たす。host32 改変を1回で恒久設計に。全イベント一貫（Option A の一貫性）＋将来拡張の器（Option B の懸念解消）。
- **❌ 短所**: 設計初期コストが最も高い（束/enum の境界設計）。M1 で値1つに束は過剰との批判もあり得る（[[analyze-ideal-form-not-minimal]] の観点では理想形寄り）。

**推奨の方向性（決定は design/discussion）**: 一貫性と拡張の口（Req2.3/1.6）を要件が明示的に求めるため **Option A または C** が要件適合的。Option C は「共通ヘッダ」という要件語彙を型に写す点で最も忠実。Option B は host32 改変を避けられない以上、最小侵襲の利得が限定的（どのみち shiori3.rs は触る）。

---

## 5. 付随ギャップの実装方針

### 5.1 OnTalk/OnHour ホワイトリスト檻（Req3）
現状 kanade は OnTalk/OnHour を構造上送れない（events 表に無い）。純データ表明として、events.rs が生成し得る全 `ShioriCall` の id 集合を列挙し「OnTalk/OnHour を含まない・許可集合に一致」を assert する回帰檻を追加（[[test-only-decision-branches-not-proven-wiring]] の判断分岐ではなく存在檻）。統合層では mock shiori の `recorded()` を走査し全 run で OnTalk/OnHour 不在を確認。

### 5.2 RecordedCall の Status 観測拡張（Req5.3）
`RecordedCall`（common/mod.rs:74）に `status: Option<String>`（等）を足し `from_call` が `ShioriCall` から写す。既存アサーション（steady_test 等）は references 中心ゆえ status 追加は additive。

### 5.3 Ref1/Ref2 将来シーム（Req1.6）
窓 geometry は UI スレッドの知識で、kanade worker へは Tick 付帯（TickInfo 拡張）が要る。M1 は「固定0＋差替可能な導出点」に留め、TickInfo 拡張の実装は増分。design で「口」の形（例: `on_second_change` に見切れ/重なりを引数化 or TickInfo から供給）を決める。

### 5.4 実機サインオフ（Req6）
実 emo2・実 pasta.dll を絶対パス起動（MOD_NOT_FOUND 注意）→数分放置→自発会話（hour.pasta 系）発火を目視、talk 中の非割込みを確認。判定は「発火の有無」に限定し再生タイミングは cue-playback へ帰属（[[areka-placement-real-ghost-first]]）。

---

## 6. Effort & Risk

| 項目 | Effort | Risk | 一言 |
|---|---|---|---|
| Status シーム層貫通（host32含む2クレート4層） | **M**（3-7日） | **Medium** | 破壊的 API 変更だが機械的・純粋生成＝全網羅可。host32 改変の確定が前提 |
| Reference 充足の確認＋Req1.6 口 | **S** | Low | 実装済み・確認と将来シームのみ |
| OnTalk/OnHour ホワイトリスト檻 | **S** | Low | 純データ表明 |
| RecordedCall Status 観測 | **S** | Low | additive |
| 実機サインオフ | **S** | Medium | 実 pasta 依存・起動運用（絶対パス）に注意 |

総じて **M / Medium**。Risk の芯は「host32 改変の是非と Status シーム設計（Option A/B/C 選択）」の1点に集約。

---

## 7. Recommendations for Design Phase

### 7.1 優先設計判断
1. **`Status` シームの形**（Option A/B/C）を design 冒頭で決裁。**是非は決着済み**——要件ディスカッション #1（2026-07-17）で開発者が **(a) M1 に維持**を選択（Requirement 2 据え置き）。brief 前提「host 側改変なし」は**反証済み**ゆえ host32（`shiori-host32-host`）の `ShioriRequest`/`build_request` 破壊的改変は**受容済みコスト**（§3 の file:line 証拠）。設計は「やるか否か」ではなく「どの形（A/B/C）で層貫通させるか」のみを扱う。
2. **`Status` 値の表現**（拡張 enum: `Talking` 実装・`Choosing`/`Online` シーム）と、talk 無し時の**省略 vs 空値**の扱い（Req2.2・SSP 実挙動 Unknown → 保守的に省略で開始し記録）。
3. **status を全 `ShioriCall` の属性にするか OnSecondChange 専用にするか**（Req2.3 の一貫性要求との整合）。
4. **RecordedCall の status 観測フィールド**追加（Req5.3 の観測可能性）。

### 7.2 Research Needed（design に持ち越す不確実性）
- **[Unknown] SSP が talk 無し時に `Status` を送るか**（省略 or `Status:` 空 or `online` 等）。ukadoc 単独ページ無し＝`doc/emo2-conformance-scope.md` §1 を実需正本とし、可能なら実 SSP の wire 観測で裏取り。M1 は保守的省略で開始。
- **[Unknown] `Status` の複数値表現**（talking/choosing 併存の区切り）。9種のうち M1 は talking のみゆえ単値で足りるが、Req2.3 の拡張形を design で先取りするか。
- **[Constraint] Ref0 の意味的正しさは ticker 結線に依存**。本番 Tick.now は GetTickCount64（ghost-setup ✅・`ticker.rs:52`）＝OS 連続起動時間。kanade 側では注入値をそのまま hour 換算しており正しいが、これは ghost-setup 責務（本 spec 外）——実機サインオフで Ref0 が妥当かを併せて観測。
- **[Constraint] Ref1/Ref2 実測は窓 geometry（UI スレッド）依存**。TickInfo 拡張が未存在ゆえ M1 は口のみ。

### 7.3 既存資産の保全
- events.rs の Reference 表・単体テスト、steady.rs の DD-6 実装、steady_test.rs の統合テストは**既存決定論資産＝不変に保つ**（additive 拡張のみ）。[[test-only-decision-branches-not-proven-wiring]]／[[deterministic-test-coverage-mandate]] に整合。
- `ShioriCall` は derive 無し（msg.rs:80）ゆえ status 追加時のテスト比較は既存の手動 destructure 方式（steady.rs:234 `assert_shiori`）を踏襲。

### 7.4 Next Step
本ギャップ分析を踏まえ `/kiro-design areka-P0-idle-talk` で技術設計へ。design 冒頭で §7.1 の4判断（特に Status シーム host32 改変）を明示的に決裁すること。

---

## 8. 要件ディスカッション決着ログ

- **#1（2026-07-17）— `Status` ヘッダの M1 スコープ帰属**: 開発者決裁 **(a) M1 に維持**。Requirement 2（2.1/2.2/2.3）据え置き。host32（`shiori-host32-host`）の `ShioriRequest`/`build_request` 破壊的 API 変更を**受容済みコスト**として確定（§3 反証を受けて）。設計判断 B1 は「是非」から「形（Option A/B/C）」へ縮退（§7.1.1 参照）。根拠: Req6 実機サインオフ（放置→自発会話）の成立条件が Status での発火制御に依存し得るため、Status を欠くとサインオフ自体が危うい。

- **#2（2026-07-17）— `Status` の状態語彙と放置時表現**: 開発者指摘「Status を最小実装へ丸めるな・ukadoc に準じよ」を受け、`Status` を ukadoc `Status [SSP拡張]`（`ukadoc:spec_shiori3:Status_20_5bSSP_62e1_5f35_5d:1`）の実行状態語彙全体（talking／choosing／minimizing／induction／passive／timecritical／nouserbreak／online／opening(種類)／balloon(ID群)・カンマ連結）へ拡充。**決裁 (甲)**: M1 は源が既配線の `talking` のみ実導出・残9状態は語彙保持＋非アクティブ縮退＋実測差替シーム（Reference1/2 同型）。放置時は**アクティブ集合が空→ヘッダ行省略**（正典＋実 SSP 捕獲ログ `ayame.log` 一致・`Status: talking` は再生中のみ）。逆側の罠＝アイドル時 `Status: talking` 送出は pasta 自発会話を恒久抑制（実 pasta `virtual_dispatcher.lua:98/123` の負ガード `req.status == "talking"` → skip）＝Req6 前提。**残状態の実導出**は本 spec 外＝choosing は `areka-P0-choice-select-events`・他は新設追跡 spec `areka-P0-status-execution-states`（2026-07-17 立ち上げ）＋ロードマップ追記。Requirement 2 を7基準へ全面改稿（旧「talking のみ・将来値」フレーミングを撤回）。裏取り根拠は本会話の研究ワークフロー（ukadoc 正典・emo2 fixture・実 SSP wire・敵対的検証3レンズ全会一致）。
