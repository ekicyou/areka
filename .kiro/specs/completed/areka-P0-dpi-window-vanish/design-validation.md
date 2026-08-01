# 設計バリデーションレポート: areka-P0-dpi-window-vanish

> 実施日: 2026-07-31 / 対象: `design.md`（確定版・本書は設計を変更しない）
> 方式: kiro-validate-design REVIEW プロセス（Analysis → Critical Issues → Strengths → GO/NO-GO）
> 検証方法: 現行ツリー実コードとの突合（Grep/Read）＋ requirements.md 全 38 受入基準のトレース確認

---

## Review Summary

設計は「単一ライター強化＋判断の純関数化」という既存 placement 規律の延長線上にあり、新規依存ゼロ・既存偽装境界の全面再利用で S1〜S3 の是正・恒久観測・決定論檻・レジストリ掃除を一貫した構造で解決している。設計が依拠するコードレベルの主張（file:line アンカー）を現物ソースで抜き取り検証した結果、**全件が実ツリーと一致**しており、誤読アンカーに基づく設計リスクは検出されなかった。全 38 受入基準のトレーサビリティ・フェーズ順序（実機採取は是正前ビルド）・W5 同居境界も充足している。以下の 3 点は実装可否を覆すものではなく、設計ディスカッションで詰めるべき改善点である。

---

## アンカー検証結果（設計の前提となる静的構造証跡の実測突合）

| 設計の主張 | 実測結果 | 判定 |
| --- | --- | --- |
| 書き手A: OS 提案矩形 left/top を `SWP_NOSIZE` で直書き・実施ログは `trace!` | `crates/wintf/src/ecs/window_proc/window_pos.rs:359-369`（`guarded_set_window_pos(suggested.left/top, SWP_NOSIZE\|SWP_NOZORDER\|SWP_NOACTIVATE)`）・`trace!` は `:352` | **一致** |
| S1: Bottom 射影は X 素通し・`raw`=`WindowPos.position` | `follow.rs:105-109`（`BottomSnapPolicy::resolve` が `x: raw.x` を返す）・`follow.rs:810-826`（`raw` を `WindowPos.position` から読む） | **一致** |
| S2: 位置再射影が `refresh_scale` の `Some` に条件付け | `frame.rs:835`（`if let Some(new_size) = source.refresh_scale_report(..)` の内側でのみ `reconcile_window_size`）・`presenter.rs:772-818` の `None` 経路（k 不変 `:772`／不可視 `:776`／未表示 `:783`／丸め後同寸 `:818`。加えて再表示不成立 `:813` も `None`＝D7 の一律処理で吸収される） | **一致** |
| S3: 最近傍フォールバックの無観測 | `follow.rs:1146-1159`（`min_by_key` で最近傍を返し warn なし・帰属判定は half-open） | **一致** |
| `GhostWindows` 掃除口なし | `spawn.rs:114-135`（`remove` 系メソッド皆無。doc に「窓 despawn 後の Entity 無効化は M1 では追跡しない」と明記＝欠陥の自己申告） | **一致** |
| `MonitorSnapshot` 構築点（Req 1.1 正典） | `main.rs:645-647`（`enumerate_monitors()` 忠実転写・無ログ） | **一致** |
| kero-balloon 編集面（同一ファイル異ハンク） | `frame.rs:928`（`run_text_scale_phase`）・`frame.rs:205/545`（`balloon_models`）は本設計の編集ハンク（`:782-839`／`:985-1026`／`:1188-1228`）と重ならない | **一致** |
| `DpiChangeContext` は thread_local set/take | `components.rs:33-83`・`window_pos.rs:53`（echo 消費）＝「`None` なら set しない」裁定の実装先が実在 | **一致** |

**要件カバレッジ**: 全 38 受入基準（1.1-1.9／2.1-2.9／3.1-3.4／4.1-4.6／5.1-5.6／6.1-6.4）が Requirements Traceability 表に行単位で存在し、各行にコンポーネント・フローの対応がある。**欠落なし**。

**フェーズ順序**: Testing Strategy「順序制約」と実装フェーズ順序（Phase A 観測＋掃除 → Phase B 実機採取＝**S1/S2/S3 是正未投入の Phase A ビルド** → Phase C 是正 → Phase D 檻）が「是正投入後は消失の実機再現が消え Q1〜Q4 の確定材料が失われる」を明記しており、証跡保全の要請を構造で満たす。Phase A の内容（水準是正・純関数骨組み・掃除）はいずれも挙動不変または良性ノイズ除去であり、採取証跡を汚染しない。

**W5 同居契約**: `placement/measure.rs` 不接触（kero-balloon 所有）・`frame.rs` 同一ファイル異ハンクの先着後 rebase 申し送り・`spawn.rs` 編集の W6 `balloon-visibility` への申し送りが Boundary／File Structure／Revalidation Triggers に三重に明記されている。

**プロジェクト規律**: 判断分岐の純関数化＋決定論檻（実証済み配線は再テストしない）・恒久観測は専用 target `areka::placement::diag` 既定 OFF・log-first の水準割当表・下端中央原点の保全・比／不変条件判定（Req 5.6）——いずれも steering・開発記憶と整合。

---

## Critical Issues（最大 3・設計ディスカッションへ送る）

🔴 **Critical Issue 1**: Req 3.4 のバルーン側可視性に構造的保証がない
**Concern**: `guard_visibility` はキャラ窓の `resize_window_to` 経路にのみ配線され、バルーン窓の矩形は誰も検査しない。設計は 3.4 を「ガード＋D7 再射影＋follow_balloon 恒等式」で充足と主張するが、恒等式はバルーンを**キャラの近くに置く**ことしか保証せず、offset が大きい・モニタ端でキャラが clamp された等の合成でバルーン矩形が全 work area 非交差になる余地が残る（確率的充足であって構造的充足でない）。
**Impact**: 3.4 は shall 条項であり、バルーンだけ消える残余欠陥が本 spec の完了後も理論上残る。W6 `balloon-visibility` との境界が曖昧なままだと「どちらも所有しない」空白になる。
**Suggestion**: 設計ディスカッションで二択を裁定する——(a) `follow_balloon` 経路にもバルーン矩形の交差検査（warn のみ・clamp しない観測点）を足す、または (b) 「バルーン単独の可視性保証は W6 所有」と Boundary に明記し 3.4 の充足範囲を「キャラ随伴に起因する不可視化の防止」へ限定解釈することを診断レポートに登記する。
**Traceability**: Requirement 3.4（Requirement 3.1 の適用対象がキャラ窓に限定されている点との対比）
**Evidence**: design.md「PlacementRoute 配管＋guard_visibility」節（適用点が `resize_window_to` のみ）・Traceability 表 3.4 行

🔴 **Critical Issue 2**: S1 の赤→緑（Req 5.4）が「是正前のコード」でなく新設純関数上の模擬で示される構成
**Concern**: Unit Tests 1 は S1 の赤を「是正前挙動（無条件 `Some(suggested)`）」として**新設する** `dpi_suggested_position_decision` 上で表現する。しかし実際の是正前欠陥は wndproc の無条件 `guarded_set_window_pos` 呼出（`window_pos.rs:359`）に在り、純関数は是正時に初めて生まれる——Req 5.4 の文言「**是正前のコードに対して**…失敗することを示す」との対応が間接的になる。
**Impact**: 檻が「欠陥のモデル」を検査して「実配線」を検査しない形に落ちると、記憶〈檻に入れるのは判断分岐のみ・実証済み配線は再テストしない〉の前提である「配線が一度は実行で実証される」が S1 について欠け、赤の証明力が弱まる。
**Suggestion**: Integration Tests 5（wintf dispatch 檻: `WM_DPICHANGED` dispatch 後に `DpiChangeContext` が set されない）を S1 の赤→緑の**正証跡**に指定し、Phase D の「是正コミット直前後で実行記録」の対象へ明記する（純関数檻は分岐網羅の補助と位置づける）。tasks 生成時に赤の採取コミット位置を固定すること。
**Traceability**: Requirement 5.4（S1）
**Evidence**: design.md「Testing Strategy > Unit Tests 1」「Integration Tests 5」「実装フェーズ順序 Phase D」

🔴 **Critical Issue 3**: Req 1.9 の機械計数語に scope・方向の結合規則が未規定
**Concern**: 充足判定の grep 語 `[WM_DPICHANGED] DPI component directly updated`（`window_pos.rs:331`・debug!）は entity と新旧 DPI を持つが **scope と窓種別を持たない**（wintf は scope を知らない＝依存方向上正しい）。1.9 は「キャラ窓の各 scope×各方向×3 回」を**機械的に計数できる語**で要求しており、entity→scope の突合規則がなければ手作業判読に退行する。
**Impact**: 受理回数下限は「再現しない」結論（Req 2.6）の成立条件そのものであり、計数が機械化できないと診断フェーズの完了判定が曖昧になる（2026-07-18 の偽陰性と同型の手順欠陥）。
**Suggestion**: diagnosis-procedure.md の設計内容（成果物節④）に「entity→scope の結合規則」を明記する——spawn 時の diag レコード（scope・種別・entity を含む）を突合キーとし、方向は同行の old/new DPI 比較で機械判定する、という 2 段 grep の手順を判定語とともに固定する。実装追加は不要（既存ログ＋Phase A の diag レコードで結合可能）。
**Traceability**: Requirement 1.9（2.6 の成立条件）
**Evidence**: design.md「成果物 > diagnosis-procedure.md ④充足条件」・`window_pos.rs:325-332`（entity・old/new DPI のみ）

---

## Design Strengths

1. **アンカーの実測精度が高く、欠陥の一般化が正しい**: 抜き取り検証した全 file:line（書き手A・X 素通し・`Some` ゲート・`None` 経路・最近傍フォールバック・掃除口なし）が現物と一致。S1/S2/S3 を「位置権威が暗黙」という単一欠陥の 3 症状へ一般化し、源断ち（明示 component `DpiSuggestedRectPolicy`）により D4「X＝直前の areka 確定接地点」が**新規 component なしで成立**する縮退を見出した設計判断は秀逸。書き手Aの遮断で echo 連鎖の実行順非決定性（research §6.1 の未解決）ごと消える点も構造的。
2. **フェーズ構成が Req 2.7（憶測修正の禁止）を構造で強制する**: Phase A（挙動不変）→ Phase B（是正前ビルドで採取＝Q1〜Q4 証跡保全）→ Phase C（静的確定分のみ是正）→ Phase D（赤→緑の実行記録）の順序が、証跡破壊と先走り改造の両方を工程レベルで排除している。W5/W6 の所有境界（measure.rs 不接触・frame.rs 異ハンク rebase・spawn.rs 申し送り）も三重に明文化され、並走事故への防波堤が厚い。

---

## Final Assessment

**Decision: GO**

**Rationale**: 設計の根拠となる静的構造証跡は全数が実ツリーと一致し、全 38 受入基準のトレース・フェーズ順序・W5 同居契約・プロジェクト規律のいずれにも致命的欠落がない。Critical Issues 3 件はいずれも設計の骨格を変えずにディスカッション裁定＋tasks 反映で解消できる改善点であり、実装着手を妨げない。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で Critical Issues 1〜3 を裁定する（特に Issue 1 の (a)/(b) 二択と Issue 2 の赤証跡の正の指定）。
2. 裁定結果を design.md へ反映（必要なら軽微改稿）。
3. `/kiro-spec-tasks areka-P0-dpi-window-vanish` で実装タスクを生成する（Phase A〜D の順序制約と赤→緑コミット位置の固定を tasks に転写すること）。
