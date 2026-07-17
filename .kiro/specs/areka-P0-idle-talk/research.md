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

- **#3（2026-07-17・ポートフォリオ合流セッション）— §9.5 の実質欠陥4件の要件本文適用**: 並走4spec 要件マージ後の合流セッション（開発者指示「衝突を正しく解決しロードマップ・要件を書き直せ」）で決裁・適用済み。(1) Req2.6 へ fail-open ただし書き（pasta 完全一致比較・実測行は **:98/:123**〔§9.5 の :96,121 は僅少ドリフト〕・台帳/choice-select-events へ申し送り）。(2) Req6.1 を「自発トーク（OnTalk 由来）」へ訂正——敵対的検証の指摘により 15-30 秒は**要件値でなく fixture 既定値の注記**へ降格（メニューで実行時可変・emo2 は聖典でない）。(3) Req6.2 を削除でなく**ログ/wire 観測**（NOTIFY・Ref3="0"・`Status: talking`）へ書換（Req5.3 観測資産で無償・非事象問題を回避しつつ実機相関証跡を残す）。(4) Req3.1 へ全構築点被覆を明文化＋推奨実装＝チョークポイント（`handle_call`／`run_shiori_loop`）検証・force_quit stale TODO 解消と OnClose の Status 添付は design 送り。あわせて Req6.3 の失効参照を completed へ更新。実装順裁定: **idle-talk 先行→input-events**（`ShioriCall`/`ShioriBackend` 共有型の shaper 先行＝衝突最小・ハードゲートではない＝逆順は機械的追随）——正本は roadmap 追記㉘。

---

## 9. セッション引き継ぎ（2026-07-17・設計フェーズ再開用）

> 要件フェーズ完了時点のセッション情報。**別セッションで `/kiro-design areka-P0-idle-talk` を再開する際の必読事項**。全事実は code/git で一次確認済み（ブロッカー監査ワークフロー・多角スイープ＋敵対的検証3レンズ）。

### 9.1 現在地

- `phase=requirements-generated`／要件ディスカッション完了（#1・#2 決裁済み＝§8）。design／tasks／implementation は未実施。
- 次コマンド: `/kiro-design areka-P0-idle-talk`。

### 9.2 【必須】設計着手前の機械的前提条件 — `origin/main` をマージすること

- 本 spec の作業ブランチ base は `fec6c693`＝**PR #60（`areka-P0-cue-playback-duration`・main `9b8317cb`・2026-07-17 19:25 マージ）の直前**。
- #60 は idle-talk が編集する `ShioriBackend` 実装5箇所のうち**3箇所を書き換え済み**（`crates/areka-ghost/tests/ghost/spine_e2e_test.rs` +289行・`crates/areka-ghost/src/runtime.rs`・`crates/areka/src/emo2_boot/spine.rs`）。加えて `areka_sakura::sink::{SurfaceSink,TextSink}` → `areka_sakura::contract::CueSink` へ、`GhostRuntime::boot` の型境界も変化。
- **→ 設計は必ず #60 以降の settled コードに対して行うこと**（pre-#60 の実測シンボルで設計すると存在しないコードを参照する）。roadmap:195 の「先行 spec は cue-playback マージ後に `/kiro-validate-design` で再突合」義務を、本 spec も**merits で継承すべき**（roadmap 上は「ゲート下でない」ため形式的には免除されているが、その免除根拠は §9.6 のとおり偽）。
- 朗報: `git diff fec6c693 9b8317cb -- crates/areka-kanade crates/shiori-host32-host` は**空**＝ギャップ分析の中核証拠（§3＝`ShioriRequest` に status 無し・`build_request` は固定ヘッダ集合）は今日の main でも**有効**。

### 9.3 ブロッカー監査の結論（2026-07-17）

- **ブロッカー = ゼロ**（確信度 高）。順序ゲート（M-boot／pilot host-32）は消化済み。時限ゲートは**前提消滅**（#60 マージ済＝`mayuna-compose`・`seriko-loop`・M-dialogue 3本も全て解禁）。生存ワークツリー（`collision-geometry`／`input-events`／`position-persist`）は**コード編集面ゼロ**（spec 文書のみ・いずれも requirements-generated）。
- **方向は逆＝idle-talk は上流**: `status-execution-states`（brief「Depends: idle-talk＝Status 語彙構造の正本」）・`choice-select-events`（brief「Status ヘッダの口＝idle-talk が設計」）・`input-events` が Status 契約を消費。**遅らせると M-dialogue を止める側**＝先に着地させるのが衝突最小順。
- Req1（Ref0〜3 正典充足）は**既に実装済み**（`events.rs:86-101` ＋既存 unit test）＝本 spec の実作業は **Status がほぼ全て**＋檻。
- Req6 は**既に実機観測済み**: #60 の Task 10 サインオフ記録（`completed/areka-P0-cue-playback-duration/tasks.md:218`・開発者承認）に実 emo2＋実 pasta.dll(i386)＋実 DPI で「**6 talk 実行（起動1＋ランダム5）**、各 steady talk が `steady_talk`→4〜5秒後に `steady_talk_done`」＝pasta の `talk_interval` 15〜30秒による自発 OnTalk が5回発火。今 Status を送っていない（`req.status=nil`＝抑制ゲートが開いている）ことが、それが出る理由でもある＝**着手前ベースライン観測が可能**。

### 9.4 【設計冒頭の決裁事項】

1. **Status シームの形**（§7.1.1・Option A/B/C）＋**爆風の選択**: `ShioriBackend::get/notify`（`crates/areka-kanade/src/shiori/real.rs:47-56`）の**署名変更**（実装5箇所・**4クレート**＝areka-kanade〔real.rs:58,290〕／shiori-host32-host／areka-ghost〔runtime.rs:464・spine_e2e_test.rs:138〕／areka〔emo2_boot/spine.rs:157〕）**vs 既定実装メソッド／`ShioriConnection` 保持**（＝爆風を2クレートに封じ込め）。Status シームは**3 spec 横断契約**ゆえ明示決裁必須。
   - **⚠️ §11 の「2クレート4層」は過少計上**——層は4で正しいが**クレートは最大4**（trait の fan-out を見落としていた）。
2. **命名衝突**: `ShioriBackend` には既に**無関係**の `fn status(&mut self) -> HelperStatus`（`real.rs:55`・helper 死活）がある。SHIORI/3.0 実行状態ヘッダは `execution_status`／`StatusHeader` 等へ別名化し理由を記録すること。
3. **檻は二層**: Req2.3（アクティブ集合が空→`Status` 行省略）は**wire 特性**＝kanade の mock（`RecordedCall` は ShioriMsg 層・`Option::None` しか見えない）では観測不能。`shiori-host32-host` の `build_request` に**バイト級 assert** が要る（先例: `shiori3.rs:424-436`「Reference ヘッダを出さない」）。
4. `RecordedCall` の status 観測フィールド（§7.1.4）。
5. Reference1/2 将来シームの口（§7.1.5）。

### 9.5 【✅決着済み 2026-07-17】要件の実質的欠陥4件（→ §8 決着ログ #3 で要件本文へ適用済み）

> ブロッカー監査で発見→ **2026-07-17 ポートフォリオ合流セッションで4件全て requirements.md へ適用済み**（適用形・修正差分は §8 #3 が正本。以下は発見時の記録＝履歴保存。なお (1) の行番号は実測 :98/:123）。

1. **[最重要・隠れギャップ級] Req2.2 のカンマ連結が emo2 に対し fail-open する**: 実 pasta の talk 抑制ゲートは `act.req.status == "talking"` の**完全一致比較**（`vendors/pasta/crates/pasta_lua/scripts/pasta/shiori/event/virtual_dispatcher.lua:96,121`）であって**集合メンバシップ判定ではない**（値は `lua_request.rs:110` で生文字列のまま転記）。ゆえに `talking,online` 等「talking と別状態の同時アクティブ」で抑制が fail-open し、**talk 再生中に OnTalk が発火**する（areka 側は NOTIFY 応答破棄で飲むため症状は「トークが黙って捨てられる」＋pasta の `next_talk_time`／チェイン状態だけが進む）。**M1 が無事なのは Req2.5 の非アクティブ縮退で wire が厳密に `talking` になるからに過ぎない**＝**Req2.6 の「送出契約を変えず実値へ差し替えられる」保証は適合対象 emo2 に対して条件付き**。→ Req2.6 のシームへ但し書きを明記し `areka-P0-status-execution-states` 台帳へ申し送るべき。
2. **Req6.1 の文言が事実誤り**: 「自発会話（**時報系トーク等**）」は実 pasta と矛盾。`check_hour` は初回呼出で `next_hour_unix` を次の正時に設定して nil を返すだけ＝**発火は次の正時（最大約1時間後）**、さらに `hour_margin`（既定30秒・emo2 `pasta.toml` 未設定）で正時直前は OnTalk までスキップ。**数分放置で観測できる自発会話は OnTalk のみ**（emo2 `pasta.toml` `[ghost] talk_interval_min=15`／`talk_interval_max=30` 秒）。→ 観測対象を **OnTalk** へ訂正すべき（brief:19 の「hour.pasta が在る＝時報が来る」も同様に誤読を誘う）。
3. **Req6.2 は実機で観測不能（非事象）**: talk 占有は実測4〜5秒 vs 間隔15〜30秒＝**自然な重なりが構造的に起きない**＝人間が「割り込まなかった」を確認できる瞬間が存在しない。Req4.3 の決定論檻が既に完全被覆。→ 実機基準から外すか、ログ観測（talk 中の Tick で NOTIFY＋Ref3=0 が出ること）へ書換。なお **Req6.3 の「タイミングの正しさは cue-playback の領分ゆえ判定に含めない」は同 spec 完了により根拠文が失効**（無害だが陳腐）。
4. **Req3.1 の檻に漏れ**: `force_quit`（`crates/areka-kanade/src/schedule/mod.rs:160-176`）が events.rs の表**外**で `ShioriCall::Notify{id:"OnClose"}` を inline 構築しており（「events.rs 実装後は委ねる」旧 TODO 残置）、ホワイトリストを events.rs だけに錨付けすると**「単一列挙点」が偽**になる。同箇所は「ForceQuit 時の OnClose にどの Status を添えるか」も未回答。→ 設計で名指しすること（in-crate＝idle-talk 自身の作業）。

### 9.6 未訂正の失効記述（既知・本セッションでは意図的に触れず）

- **`brief.md` Approach 手順3**「ヘッダ付与は `Shiori3Client` の既存汎用 request 経路に乗る（host 側改変なし・`build_request` は汎用ヘッダ対応済み）」＝**反証済み**（§3 の file:line 証拠）。brief は discovery 時点の記録として残置＝**この前提を信じないこと**。
- **`.kiro/steering/roadmap.md:192`** の idle-talk 並走根拠「③・kanade steady/events のみ＝script 生産者であって cue モデル編集者でない」＝**偽**（実際は kanade＋shiori-host32-host＋ShioriBackend 実装群）。**判定（並走可）は正しいが根拠が誤り**——ゲートの判定基準は roadmap:195 自身が「時限ゲートの実体は**コード編集面の衝突**」と定めており、正しい基準を当てれば #60 マージ前は idle-talk は cue-playback と**編集面を共有していた**（実装5箇所のうち3箇所を #60 が書換）＝**タイミングに救われただけ**。誤基準の再利用を避けるため要訂正。訂正文案: 「編集面＝kanade steady/events ＋ shiori-host32-host〔Status ヘッダ〕＋ ShioriBackend 実装（areka-ghost/areka）＝**cue モデルとの交差ゼロ**ゆえ並走可」。
- **`roadmap.md:190`／`:167`・`focus.md:67`** が `areka-P0-cue-playback-duration` を「**実装中**（別坑）」と記述＝**失効**（#60 完了・`completed/` アーカイブ済）。時限ゲート節ごと退役し `mayuna-compose`・`seriko-loop`・M-dialogue 3本の解禁を明記すべき。
- **⚠️ 並行編集の衝突注意**: 別ワークツリー `epic-kepler-bdbee8` が `.kiro/steering/roadmap.md` を**未コミット編集中**（ゲート解除・追記㉗）＝**本 spec は roadmap の当該箇所に敢えて触れず衝突を回避した**（本 spec の roadmap 編集は `status-execution-states` の登録＋M2 送り注記＋idle-talk 行の Status 語彙追記のみ）。上記訂正はそちらと調整のこと（[[harness-shell-quirks]] の kiro-complete コンフリクト定石）。
- **既知の隣接欠陥（kanade 領分・#60 Task 10 記録）**: 起動挨拶で非致命 ERROR `unknown_talk_done talk_id=1`（kanade が挨拶の talk_id を追跡せず TalkDone が無照合 slot へ到着）。Req6 には影響しないが **idle-talk が自然な引受先**として記録されている。

### 9.7 実機サインオフ（Req6）の運用メモ

- 自動ハーネス `AREKA_EMO2_REAL_RUN` は `AREKA_APP_SMOKE_EXIT_MS=1500`（ms）で自動 close するため **idle-talk（15〜30秒待ち）の観測には使えない**＝文書化済みの**手動直接起動**（`target\<profile>\areka.exe <ghost_root> <balloon_root>`）を使う。
- **罠**: `cargo build/test --workspace` は **x64 の `shiori-host32-helper.exe` を `target/debug/` へ落とす**＝32bit `pasta.dll` を LoadLibrary できない。**i686 ビルドを `areka.exe` 隣へ上書きコピー**すること（`crates/areka/tests/emo2_real_run.rs:26-34`）。
- brief の「起動は絶対パス必須（MOD_NOT_FOUND）」と `emo2_real_run.rs:75-95` の文書化手順（相対パス表記）が食い違う＝design で運用手順を一本化すること。

---

## 10. 設計フェーズ Discovery（2026-07-17・`/kiro-design` 実行）

> **Discovery Scope**: Extension（light discovery＝既存2クレートへの層貫通拡張）＋正典/実 wire の一次裏取り。
> **前提条件の充足**: §9.2 の必須マージ条件は**充足済み**——作業ブランチ HEAD＝`dd888f2f`＝`origin/main` の先端（PR #60/#65/#66 を含む）。`git merge-base --is-ancestor origin/main HEAD` が真。ゆえに設計は #60 以降の settled コードに対して行った。
> **Key Findings**: (1) 実 SSP 捕獲ログを発見し「空集合→行省略」を実物で裏取り（§10.2）(2) `Status` の wire 位置と正典順を実物で確定（§10.2）(3) Req3.1 が推奨したチョークポイントは前提が偽と判明し、真の唯一出口を特定（§10.3）(4) pasta は OnSecondChange の Reference を**一切読まない**＝Ref1/Ref2 固定 0 の正当性を消費側コードで再確認（§10.1）。

### 10.1 pasta（消費側）の実査 — `vendors/pasta` @ `048d646c`

| 調査項目 | 実測 | 含意 |
|---|---|---|
| talk 抑制ゲート | `virtual_dispatcher.lua:98`（`check_hour`）・`:123`（`check_talk`）＝`if act.req.status == "talking" then return nil`。**完全一致比較** | §9.5.1 の指摘を確認（行番号は **:98/:123** が実測＝§9.5 の :96,121 は僅少ドリフト）。集合メンバシップ判定でない |
| Status の受渡し | `pasta_shiori/src/lua_request.rs:110` `Rule::key_status => table.set("status", value)` — split/parse なしの**生文字列転記** | 複合値は Lua 側でも分解されない＝fail-open は構造的 |
| OnSecondChange の Reference 消費 | **皆無**。`second_change.lua:14-18` は `dispatcher.dispatch(act)` のみ。dispatcher が読む request フィールドは `req.date.unix`（:84,:119）と `req.status`（:98,:123）の**2つだけ**。`act.lua:161` の `transfer_req_to_var`（`r0..r9` 露出）は**どこからも呼ばれない** | **Req1.3（Ref1/Ref2 固定 "0"）の正当性を消費側コードで再確認**（§1.2 の fixture grep に加えた二重の裏取り）。Ref0/Ref3 すら pasta には inert |
| `check_hour` の初回デッドウィンドウ | `:87-90` — `next_hour_unix == 0` なら次の正時を設定して `return nil`。`calculate_next_hour_unix`（:50-53）＝`unix - (unix % 3600) + 3600` | §9.5.2 を確認＝**数分放置で観測できる自発会話は OnTalk のみ**（Req6.1 の訂正が正しい） |
| ゲート後の状態 | `check_hour` の skip は `next_hour_unix` を**消費しない**（:103 の更新前に return）。`check_talk` の skip は `next_talk_time` の初期化（:128）**より前** | アイドル時に `talking` を誤送出すると自発会話が**恒久停止**する（Req2.7 の罠）＝実機サインオフが検出する |
| `talk_interval` | submodule HEAD 既定＝`:40-41` の `180`/`300`。emo2 fixture が上書き＝`fixtures/emo2/ghost/master/pasta.toml:23-24` の `15`/`30`。`menu.pasta:36-49` が実行時プリセット（30-45／60-90／180-300）を提供 | Req6.1 の「15〜30秒は fixture 既定値であって要件値でない」注記が正しい。なお submodule HEAD の script は `pasta_talk_interval*` を読まず `get_config` が初回キャッシュ＝**同梱 `pasta.dll` は HEAD と別ビルド**（emo2 は聖典でない原則の実例） |

### 10.2 実 SSP wire の一次証拠 — **`ayame.log` は存在しない**

§8 #2 が参照した `ayame.log` は**リポジトリに実在しない**（worktree 全域を grep：言及は `research.md:211` の散文のみ）。実 wire の正本は以下である:

**`vendors/pasta/crates/pasta_shiori/doc/shiori-sample.log`**（実 SSP 2.3.86 捕獲・8,517 行・903 SHIORI メッセージ・387 `Status:` 行・39 OnSecondChange ブロック＝GET 19／NOTIFY 20）

| 観測 | 証拠 | 設計への含意 |
|---|---|---|
| **アイドル時は `Status` 行が無い** | `:2291-2300` — `GET`／`Charset`／`Sender`／`SecurityLevel`／`ID: OnSecondChange`／`Reference0..4`。空値 `Status:` **ではなく行そのものが不在**。Ref3=1・Ref4=1 | **Req2.3 を実物が裏付ける**（DD-IT-5）。ukadoc には省略に関する記述が無いため、これが唯一の実挙動証拠 |
| talk 中は `Status: talking,balloon(0=0)`＋Ref3=0＋NOTIFY | `:1307-1318` | ukadoc の NOTIFY 規則と一致。**かつ実 SSP は複合値を送る**＝pasta は SSP に対し既に fail-open |
| ヘッダ順は一貫 | `Charset → Sender → SecurityLevel → Status → ID → Reference*` | **`Status` は `ID` より前**（DD-IT-6） |
| 観測された `Status` 値は6種のみ | `balloon(0=0)` 97回／`talking,balloon(0=0)` 208回／`talking` 1回（`ID: rateofusegraph`・:1253）／`choosing,balloon(0=2,1=0)` 43回／`talking,choosing,balloon(0=2)` 5回／`talking,choosing,balloon(0=2,1=0)` 33回 | **全て正典の語彙定義順**（talking→choosing→balloon）＝「正典順で連結」を実物が裏付ける |
| **正典と実 SSP の乖離** | 正典＝`balloon(0=2/1=0)`（`/` 区切り）／実 SSP＝`balloon(0=2,1=0)`（**`,` 区切り**・`:3727` 他 76 箇所） | 実 SSP 形はトップレベル `,` と衝突し `split(',')` を壊す＝曖昧。正典の `/` は自己無矛盾 → **DD-IT-9 で正典を採用** |

### 10.3 Req3.1 の推奨チョークポイントは前提が偽 — 真の出口を特定

要件 3.1 は「本番/mock が共通で通る単一チョークポイント（`handle_call`／`run_shiori_loop`・real.rs:99/113）」を推奨するが、**実測でこの前提は成り立たない**:

- 統合ハーネスの mock（`crates/areka-kanade/tests/kanade/common/mod.rs:255` `spawn_mock_shiori`）は **`ShioriMsg` チャネル層で shiori アクター丸ごとを差し替える**（同 mod.rs:7-9 が「trait 不要＝型レベル差し替え」と明記）。`ShioriBackend` を**一切使わない**＝`handle_call` を通らない。
- `handle_call`（`real.rs:99-111`）を通るのは本番 `ShioriConnection` と in-source `FakeBackend`（`real.rs:290`）のみ。

**真の唯一出口＝`actor.rs::round_trip_request`（`actor.rs:146-149`）**。`Action::ShioriRequest` の実行点は `drive` の `actor.rs:111-113` ただ1箇所で、そこから `round_trip_request` へ入り `ShioriMsg::Request` を送る。本番・統合 mock の**双方が必ず通る**。→ Req3.1 の**規範節**（「全 `ShioriCall` 構築点を被覆」）を真に満たすのはこちら。推奨実装は根拠が誤りゆえ設計で変更した（DD-IT-7・要件本文の変更は不要＝規範節は満たす）。

### 10.4 その他の実測（設計に反映／申し送り）

- **`ShioriCall` 構築点の全数**（prod）: `events.rs` 7 箇所（:36,48,58,68,91,96,107）＋ **`schedule/mod.rs:170` の `force_quit` inline のみ**。§9.5.4 の指摘を確認。`boot.rs`／`close.rs` は自前構築ゼロ（events 経由）。
- **委譲不能の理由が判明**: `events::on_close` は **GET**（`:107`）だが `force_quit` は **NOTIFY** を要する（`mod.rs:170`）。ゆえに単純委譲は不可 → **`on_close_notify` の増設**が解（DD-IT-8）。`mod.rs:161-164` の注記は「TODO」の語を含まないが実質 stale。
- **`ShioriBackend` 実装は 5 箇所・4 クレート**（§9.4.1 の指摘どおり）: `real.rs:58`（本番 `ShioriConnection`）／`real.rs:290`（`FakeBackend`・test）／`areka-ghost/src/runtime.rs:464`（`FakeShioriBackend`・test）／`areka-ghost/tests/ghost/spine_e2e_test.rs:138`／`areka/src/emo2_boot/spine.rs:157`。**本番実装は 1 つのみ**。
- **命名衝突は 2 件**（§9.4.2 の指摘＋1件）: `ShioriBackend::status() -> HelperStatus`（`real.rs:55`・helper 死活）と、`areka/src/emo2_boot/spine.rs` の `RecordedCall::Status`（同じく helper 死活）。→ `ExecutionStatus`／`ExecutionState`／`ExecutionSnapshot` で別名化（DD-IT-2）。
- **`..` による黙殺リスク**: `steady_test.rs:287-290, 398-401, 543-546, 566-569` は既に `Get{references, ..}` で分解＝フィールド追加後も**コンパイルは通るが Status を検査しない**。→ 期待値構築を `expected_call(events::..)` へ寄せる（design Testing Strategy 15）。
- **`build_request` の既存 wire 檻は `contains` ベース**（`shiori3.rs:361-485`）＝Status 行の追加は非破壊。ただし `build_empty_references_still_terminated`（`:424`）の `!contains("Reference")` は将来の Status 値と衝突し得る（本設計の正典語彙とは非衝突）。
- **【申し送り／本 spec 外】`SecurityLevel` の位置逸脱**: ukadoc `spec_shiori3:SecurityLevel:1` は「できるだけ最初のほうの行で通知される。少なくとも **ID ヘッダより前に現れる**」と定め、実 SSP も `Charset→Sender→SecurityLevel→Status→ID` の順で送る。**現 `build_request` は `SecurityLevel` を末尾に置く**（`shiori3.rs:113`）＝既存逸脱。本 spec は `Status` の位置のみを扱い**修正しない**（Out of Boundary）。引受先候補＝`areka-P0-emo2-conformance-e2e`。
- **【申し送り／文書】`doc/emo2-conformance-scope.md:18`** の「`Status`（talking/choosing/online 等**9種**）」は正典の **10 種**に対し過少。同行の「`Reference0..n`」も OnSecondChange については偽（§10.1）。scope doc は実需正本として有効だが、語彙の正本は ukadoc。

---

## 11. Architecture Pattern Evaluation（`Status` シームの形）

| Option | Description | Strengths | Risks / Limitations | 判定 |
|--------|-------------|-----------|---------------------|---|
| **A: 層貫通の第一級フィールド** | 各層の型へ `status` を追加。kanade が語彙を所有し host32 へは不透明文字列で渡す | 型で明示・全リクエストへ一貫適用・観測が漏れない・下流3 spec の消費契約として素直 | 4 クレート横断の破壊的変更（実装5箇所・構築点23箇所） | **採用**（DD-IT-1） |
| B: OnSecondChange 専用の別経路 | `get_with_status` 等を1本足す | 影響範囲を局限 | 「共通ヘッダ」要件語彙と乖離／**どのみち host32 改変は必須**ゆえ利得が限定的／Req2.6 の汎用化で二度手間 | 棄却 |
| C: 共通ヘッダ束を導入 | `status` を第一メンバとする束を1つ足す | 将来ヘッダの器 | **M1 は単一メンバ**＝束は過剰（synthesis 簡素化レンズ）。要件が約束するのは*状態値*の差替（Req2.6）であって*ヘッダ集合*の拡張ではない | 棄却 |

---

## 12. Design Decisions（design.md「設計判断」表の trade-off 補遺）

### Decision: 単一スナップショット（`ExecutionSnapshot`）から Ref3 と Status を導出（DD-IT-3）
- **Context**: 今日 `on_second_change(now, talk_playable)` は Ref3 と GET/NOTIFY を `talk_playable` から決める。ここへ Status を足すと**同じ源を2度読む**形になる（events.rs:86-101 は `talk_playable` の3つ目の消費者を招く）。
- **Alternatives Considered**: 1. `on_second_change(now, talk_playable, status)` — 呼び手が両者を独立に渡す／2. `on_second_change(now, &ExecutionStatus)` — Ref3 を `status.contains(Talking)` から導く。
- **Selected Approach**: `on_second_change(now, &ExecutionSnapshot)`。スナップショットが Ref3・GET/NOTIFY・Status の**唯一の源**。
- **Rationale**: 案1は「Ref3=`"1"` かつ `Status: talking`」という**不整合の組み合わせを表現可能**にする（実装バグの余地）。案2は Reference3（トーク再生可否）の意味を Status 語彙へ**意味的に結合**してしまう（別概念）。スナップショット案は不整合を型で表現不能にしつつ両概念を分離したまま保つ。
- **Trade-offs**: 全構築関数の署名が変わる（+7 箇所）。見返りに**構築点が Status を忘れられない**（共通ヘッダの構造的強制）。
- **Follow-up**: 将来「トーク再生可否」が talk 有無以外の条件を得たら、`ExecutionSnapshot` に別フィールドを設けて分岐させる（Ref3 と `talking` を同一 bool に癒着させない）。

### Decision: ForceQuit 時 OnClose の Status（Req3.1 の design 送り事項・DD-IT-4）
- **Context**: 要件が「ForceQuit 時 OnClose に添える `Status` の扱いは design で確定する」と明示。
- **Alternatives Considered**: 1. 常に非アクティブを固定（強制終了は talk を放棄するため）／2. 遷移**前**の phase から導出（talk 中なら `talking`）／3. 遷移**後**の phase から導出。
- **Selected Approach**: **3**。`force_quit`（`mod.rs:166-175`）は `state.phase = Phase::Unloading{Forced}` を**代入した後**に NOTIFY を構築する既存順序を持つ。`snapshot_of(&state.phase)` をその位置で呼べば自動的に非アクティブ＝ヘッダ行省略。
- **Rationale**: 案1は「例外規則」を1つ作る（規則が2つに割れる）。案2は「放棄した talk を再生中と主張する」ことになり、かつ ForceQuit 経路にだけ特別な読み出し順序を要求する。案3は**単一規則「Status ＝送出時点の phase のスナップショット」を例外なく全構築点へ適用**しつつ、結果的に案1と同じ安全側の値を出す。
- **Trade-offs**: 「talk 中に強制終了した」という情報は wire に出ない。ただし当該 NOTIFY は best-effort で応答は構造的に破棄され、直後に unload するため消費側に影響しない。
- **Follow-up**: `close_test.rs:560-565` の期待値を `events::on_close_notify` 由来へ寄せる（inline 二重定義の解消）。

### Decision: パラメータ付き状態の下位書式は正典（`/`）を採用（DD-IT-9）
- **Context**: 正典 `balloon(0=2/1=0)` vs 実 SSP 2.3.86 `balloon(0=2,1=0)`（§10.2）。
- **Alternatives Considered**: 1. 実 SSP に合わせ `,`／2. 正典どおり `/`／3. 切替可能にする。
- **Selected Approach**: **2**（正典）。
- **Rationale**: (a) steering「正典は ukadoc・実装は聖典でない」。(b) **技術的にも正典が優る**——内部が `,` だとトップレベルの `,` 連結と衝突し `split(',')` が `balloon(0=2` と `1=0)` に割れる。`/` は非衝突＝自己無矛盾。(c) **M1 は `opening`／`balloon` とも非アクティブ＝送出せず実害ゼロ**——今決めても消費者が居らず、決めないと `render` が半端になる。案3は憶測の可変性（`spec 工場の禁止`）。
- **Trade-offs**: 将来 `balloon` を実導出したとき、`,` を期待する消費側と食い違う可能性。
- **Follow-up**: 実導出の所有者（`areka-P0-status-execution-states`）が**実 SSP 互換の再検証時に決着**させる（同 brief Approach 2b の受け入れ条件に相乗り）。

### Decision: 既定実装メソッドで爆風を封じ込めない（§9.4.1 の「爆風の選択」への回答）
- **Context**: `ShioriBackend::get/notify` の署名変更は 4 クレート・5 実装へ波及。既定実装メソッド／`ShioriConnection` 保持で 2 クレートに封じ込める案があった。
- **Selected Approach**: **署名変更を受容**（Option A）。
- **Rationale**: 既定実装は「未追随の実装が Status を黙って落とす」＝**fail-open** を生む。本 spec は 3 spec 横断の**契約正本**であり、契約を黙って落とす経路を残せない。爆風は機械的・コンパイラ捕捉で、実装 5 箇所のうち**本番は 1 つだけ**（残 4 はテストダブル）。roadmap W1 の「共有型の shaper が先行」順序（追記㉘/㉙）がこの爆風を最小化する編成そのもの。
- **Trade-offs**: `areka-ghost`／`areka` が同時に落ちる期間が生じる（同一 PR 内で解消）。

---

## 13. Synthesis Outcomes（3レンズ適用）

### 13.1 一般化（Generalization）
- **Req1.6（Ref1/Ref2 の差替シーム）と Req2.5/2.6（Status 残状態の差替シーム）は同じ問題の変種**——要件自身が「Reference1/Reference2 と同型」と述べている。→ **`ExecutionSnapshot` という単一の口へ一般化**した。見切れ／重なりの実測供給も、実行状態の源着地も、**同じ構造体へフィールドを1本足す**という同一の操作になる。インターフェイスのみ一般化し、実装は現要件の範囲（`talk_active` 1本）に留める。
- **`Status` は OnSecondChange 専用でなく全リクエストの共通属性**へ一般化（要件の語彙「共通ヘッダ」に忠実）。ただし M1 で非空になるのは OnSecondChange のみ（他は phase から自動的に空）＝実装スコープは広げない。

### 13.2 Build vs. Adopt
- **wire 整形**: 既存 `build_request`（自前 SHIORI/3.0 codec）を**採用・拡張**。任意ヘッダマップ機構は導入しない（M1 の追加ヘッダは `Status` 1本＝汎用機構は過剰）。
- **観測ハーネス**: 既存 `spawn_mock_shiori`／`RecordedCall`／`log_capture` を**採用・additive 拡張**。新規ハーネスを建てない。
- **`Status` 語彙**: 既製の解決策なし（伺か固有）＝**自前**。ただし**正典（ukadoc）＋実 SSP 捕獲ログという既存の一次資料へ全面的に従属**させ、発明を排した。
- **決定論檻**: 既存の events.rs 単体テスト・steady_test.rs 統合テストを**保存**（`test-only-decision-branches-not-proven-wiring`／`obsolete-vs-broken-test-policy`）。

### 13.3 簡素化（Simplification）
- **共通ヘッダ束（Option C）を棄却**——単一メンバの束は間接層の水増し。要件が約束するのは*状態値*の差替であって*ヘッダ集合*の拡張ではない。
- **`StatusInputs` と `ExecutionStatus` を別々に持つ案を却下**し、源＝`ExecutionSnapshot` 1本に統合（Reference と Status の共通の源＝重複した「源の型」を作らない）。
- **TickInfo 拡張を今回作らない**——見切れ／重なりの実測は Out of scope。口は `ExecutionSnapshot` のコメント付きシームで足り、**存在しない供給者のための構造体を先に建てない**（`spec 工場の禁止`）。
- **9状態のためのダミー源を作らない**——非アクティブ縮退は導出表の「行＋シーム注記」で表す（Ref1/Ref2 の固定 `"0"`＋注記と同型）。M1 の入力空間は bool 1本＝**全網羅テスト可能**という副次利得。
- **`unknown_talk_done` 欠陥を吸収しない**（要件外・Out of Boundary へ明記）。

---

## 14. Risks & Mitigations（設計フェーズ時点）

- **消費側 fail-open（最重要・Req2.6 ただし書き）** — M1 は縮退により wire が厳密に `talking` 単独ゆえ安全。実値差替の解禁時に複合値 wire の消費側互換検証を受け入れ条件へ（台帳 spec へ登記済み・design Revalidation Triggers）。
- **正典 `/` と実 SSP `,` の乖離** — M1 は送出せず実害ゼロ。実導出の所有者が決着（DD-IT-9 Follow-up）。
- **4 クレート横断の破壊的変更** — 機械的・コンパイラ捕捉。実装5箇所は特定済み。W1 先鋒順序が衝突を最小化。
- **`Status` 追加が pasta の talk スケジュールを変える** — 抑制ゲートが初めて閉じるため talk 中は `next_talk_time` が進まない（`virtual_dispatcher.lua:120-128`）＝発火間隔の体感が変わり得る。仕様どおり（SSP と同じ）。Req6 の判定はタイミングを含めない（Req6.3）。
- **既存 `..` 分解による Status の黙殺** — 期待値構築を `expected_call(events::..)` へ寄せる。
- **実機サインオフの運用ミス** — x64 helper が `target/debug/` に居ると 32bit `pasta.dll` を load できない／自動ハーネスは 1500ms で close する。design の運用手順へ一本化（絶対パス・i686 上書きコピー・自動 close なし・`RUST_LOG=info,kanade=trace`）。

---

## 15. References

- ukadoc `Status [SSP拡張]` — `ukadoc:spec_shiori3:Status_20_5bSSP_62e1_5f35_5d:1`（実行状態語彙10種・カンマ連結・`opening(種類)` と `balloon(ID群)` の `/` 区切り下位書式）
- ukadoc `OnSecondChange` — `ukadoc:list_shiori_event:OnSecondChange:1`（Ref0〜4・再生不能時 Ref3=0＋NOTIFY＋返却スクリプト無視）
- ukadoc `SecurityLevel` — `ukadoc:spec_shiori3:SecurityLevel:1`（「少なくとも ID ヘッダより前に現れる」＝§10.4 の既存逸脱の根拠）
- **実 SSP 捕獲ログ** — `vendors/pasta/crates/pasta_shiori/doc/shiori-sample.log`（SSP 2.3.86・903 メッセージ・39 OnSecondChange。**`ayame.log` は実在しない**＝§8 #2 の参照先を本ログへ訂正）
- pasta 消費側 — `vendors/pasta/crates/pasta_lua/scripts/pasta/shiori/event/virtual_dispatcher.lua:98,123`（完全一致ゲート）・`:87-90`（check_hour 初回デッドウィンドウ）・`pasta_shiori/src/lua_request.rs:110`（生文字列転記）
- emo2 fixture — `crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/pasta.toml:23-24`（`talk_interval` 15/30）・`dic/menu.pasta:36-49`（実行時プリセット）
- 実需正本 — `doc/emo2-conformance-scope.md:18,22,27`（emo2 が読むヘッダ・OnSecondChange＝心臓部・`OnTalk`/`OnHour` 送出禁止）
- 先例 — `.kiro/specs/completed/areka-P0-cue-playback-duration/tasks.md:218`（実機サインオフ運用＝絶対パス起動・i686 helper 上書きコピー）
- 下流契約 — `.kiro/specs/areka-P0-status-execution-states/brief.md`（残状態の台帳・Approach 2b＝消費側互換の檻）

## 16. 設計ディスカッション決着ログ（2026-07-17）

### #1 檻違反の失敗語彙 → `ShioriFailure::Internal(String)` 追加（DD-IT-11）

**開発者裁定**: 「あるべき正しい姿にすべき。必要ならスコープを増やす。無意味に型を増やしすぎるのもよくない。シンプルに、誠実に。」＝ variant 1 本の追加を採択（新 enum の発明は不採用）。

**裁定を支えた実測**（50 subagent・47 主張の敵対的検証＝34 confirmed / 12 imprecise / 1 refuted）:

1. **爆風＝ワークスペース全体で 1 箇所**。`ShioriFailure` は `msg.rs:105-120`（derive は Debug+thiserror のみ・`#[non_exhaustive]` 無し・Clone 無しは `common/mod.rs:297` が意図明記）。variant 追加でコンパイルが壊れるのは cfg(test) 内 `describe`（`real.rs:256-267`・ワイルドカード無し nested match）**のみ**＝`cargo build` 緑・`cargo test` が捕捉。本番消費は 2 箇所（`schedule/mod.rs:235-240,254-256`）とも `ref failure`＋`%failure`（Display）で variant を見ない＝**無改変で正しく流れる**。
2. **コンパイラが捕捉しない追随が 1 系統**: テスト記述子 `FailKind`（`common/mod.rs:301,314`・`failure_test.rs:75-80`）は `ShioriFailure` でなく `FailKind` を match するため、5 本目を足しても両檻は黙って通り「4 語彙を静的に網羅」（`failure_test.rs:66`）が嘘になる。→ File Structure Plan へ意図的追随として計上。
3. **「内部規律違反 variant の先例なし」は反証された**: `SessionError::RequestInFlight`（`areka/src/shiori_session.rs:48-61`・「利用規律」違反と外部 `Shiori(#[from])` の同居）／`UiSpawnError`（`areka-actor/src/ui.rs:165-177`・「検出可能な前提違反を error! 記録のうえ返す経路として予約された型・panic はしない」）。`Internal` は発明でなく既存規律への合流。
4. **失敗方針の正本は steering に無い**: `.kiro/steering/` 全 9 ファイル grep で不在。正本＝`completed/areka-P0-actor-foundation/design.md:523`（検出可能な前提違反は `error!`＋`Err`・panic は致命限定）＋`completed/areka-P0-kanade/requirements.md:88-97`（Req6.1 区別語彙・6.3 ログ無し失敗禁止・6.4 致命ログ後のみ panic 許容）。panic 案の棄却根拠は「panic 禁止」でなく「檻違反＝検出可能な前提違反」条項。
5. **fault 終端の実挙動＝「沈黙のゾンビ」**（旧設計文「明示的に落ちる」は事実に反した）: `Failed` は `round_trip_request` を無記録で素通し（`actor.rs:173-174`・ログは transport 失敗 3 種のみ）→ `Input::ShioriReply` 再注入（`:129-135`）→ `awaits_reply` 6/10 phase のみ `ERROR shiori_failed` 1 行で `to_unloading_fault`（`mod.rs:234-240`）→ `Unloading{Fault}`→`Stopped`→`StopSelf`＝スレッド終了。**プロセスは死なない**: `app.run()`（`main.rs:318`）が窓ループでブロック・`kanade_handle` の join は `run()` 復帰後の `shutdown` のみ（`runtime.rs:195`）・走行中の監視者ゼロ。運用者ログは 3 行（ERROR 1＋INFO 2）＋ticker の dead-inbox INFO（`ticker.rs:228-236`）で以後沈黙。→ 全 `Failed` 共通の kanade 既存欠陥として **Open Item 4 に登記**（本 spec は吸収しない）・設計文は事実へ訂正済み。
6. **檻チョークポイントの保証水準**（検証で判明・議題 #3 へ）: `round_trip_request` は kanade drive ループ内の唯一の egress だが、`ShioriMsg` は pub（`msg.rs:65`）・`spawn_shiori_actor` は再エクスポート（`lib.rs:39`）＝`Sender<ShioriMsg>` 保持者は檻を迂回して直接 post できる（in-tree 使用例 `real.rs:762`・`common/mod.rs:1110`）。被覆は**型保証でなく所有規律**（sender は kanade actor のみが保持・`areka-ghost/src/runtime.rs:117`）。
7. **下流の檻負担の実測**（議題 #4 の布石）: input-events（W2）＝追加 2 ID のみ・自要件で檻を 2 回名指し済み（`requirements.md:28,103`）。position-persist（W3）＝追加ゼロ。**choice-select-events（W5）＝構造的非互換**——ukadoc 正典 `\q[タイトル,OnID,...]` は実行時スクリプト由来の**任意名イベント**を発火し（emo2 は fixture grep 0 件でこの形のみに依存・`brief.md:13`）、`&'static str` の id（`msg.rs:82,86`）にも固定 const 表にも載らない。

**適用差分**: design.md＝DD-IT-11 新設・Error Handling 表 1 行目（Internal＋ゾンビ事実訂正）・actor.rs Service Interface doc・File Structure Plan（msg.rs／real.rs／common/mod.rs／failure_test.rs 行）・機械的追随注記 4 型化・Risks 爆風 3 軸化・Testing Strategy #7・Open Items #4 新設。

### #2〜#4 再検討（2026-07-17・開発者指示「考えればわかるものを議題にするな・危険な分岐だけを議事に」）

**メタ裁定の適用**: 残り3議題を #1 裁定原理（あるべき姿・シンプルに誠実に・必要ならスコープ増・型の乱造禁止）へ照らして再検討した結果、**3件とも導出可能＝開発者判断が必須の危険分岐は 0 件**。以下は導出記録（#2 のみ completed spec の確定判断の再開封を含むため事後拒否権を明示）。

#### #2 起動挨拶中の wire 正直性 → **O2（boot talk 正規追跡）を導出適用＝DD-IT-12**

- **導出**: Req1.5「アクティブなトークが再生中は Ref3=0」に boot 例外条項は無い。挨拶（実測 4〜5 秒）は実再生される talk であり、fire-and-forget（`boot.rs:102-107`）による非追跡は「源の欠落」であって要件免除ではない。「誠実に」＝wire は事実を申告する。よって O1（縮退受容＋注記）は「要件との不一致の容認」となり裁定原理に反する。
- **敵対的自己検証で確認した安全性**: ① 挨拶中 close は `CloseTalkWait`（中断 ACK 待ち）＝**通常 talk の close と同型化**＝より正規形（`canonical-not-minimal-lifecycle`）② TalkDone(挨拶) の配送は既知 ERROR `unknown_talk_done` の存在自体が証明 ③ pasta 側は挨拶中ゲートが閉じ初回ランダムトークが挨拶終了後起算（〜5 秒後ろ倒し・SSP 同型・Req6.3 でタイミングは判定外）④ 檻期待値の更新は Value 経路のみ（`obsolete-vs-broken` 方針の「更新」に該当）。
- **副産物**: `unknown_talk_done` 根治（旧 Open Item 3 解消・「ついで吸収」でなく Req1.5 充足の帰結）・Req6.2 実機証跡が起動直後の挨拶窓で確定観測可能。
- **事後拒否権**: completed kanade の「boot は close-gate しない」DD の再開封を含む。開発者が fire-and-forget 維持（O1）を選ぶ場合は本コミット 1 個の revert で戻る（設計のみ・実装未着手）。

#### #3 チョークポイントの保証水準 → **保証境界の明文化のみ（分岐なし）**

- **導出**: 檻の主語は Req3.2 の文言どおり kanade。`Sender<ShioriMsg>` 直接 post による迂回（`msg.rs:65` pub・`lib.rs:39` 再エクスポート・in-tree 使用例 `real.rs:762`／`common/mod.rs:1110`）は「kanade の送出」ではなく、同階級のコードは `Shiori3Client` 直叩きも可能＝kanade 内の檻で防御不能な階級であり、どの設計を選んでも消えない。型封鎖は mock 統合ハーネスの検証様式（`ShioriMsg` の受理・構築）と非両立。→ 選択肢が実在しない＝議題でなく注記。actor.rs Responsibilities へ「保証境界」bullet を追加。
- 檻の実効範囲＝「kanade 状態機械の全構築点（現在・将来）」は DD-IT-7 のとおり成立（本番・mock 双方が `round_trip_request` を通過）。

#### #4 `ALLOWED_EVENT_IDS` と W5 任意名イベント → **シーム注記＋W5 申し送りのみ（今決めるのは憶測先行）**

- **事実**: choice-select-events（W5）は `\q[タイトル,OnID]`＝実行時スクリプト由来の任意名イベント発火に依存（emo2 唯一の依存形・`menu.pasta:15` 実物・fixture に OnChoiceSelect 系ハンドラ 0 件）。任意名は `&'static str` の id（`msg.rs:82,86`）にも固定 const 表にも載らない。
- **導出**: W5 の拡張は受理規則への**カテゴリ追加（additive）**であり檻の解体ではない＝チョークポイント・固定表・`OnTalk`/`OnHour` 恒久禁止は不変のまま将来拡張可能。今 String 化や拡張受理規則を先取りするのは「憶測先行設計しない」（status-execution-states 台帳の規律・YAGNI）に反し、`\q[x,OnTalk]` と Req3.2 恒久禁止の交差は W5 の要件フェーズの議題（`portfolio-convergence`＝単一 spec の椅子から決めない）。→ events.rs へ SEAM(W5) doc・Revalidation Triggers 行を精密化・本節を W5 への申し送りの正本とする。
