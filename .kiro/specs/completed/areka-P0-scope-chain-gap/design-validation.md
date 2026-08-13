# Design Validation: areka-P0-scope-chain-gap

> 実施: 2026-08-11（kiro-validate-design・非対話モード）
> 対象: `design.md`（finalized）／照合: `requirements.md`・`research.md`・`ssp-oracle-notes.md`・steering・現行コードベース（file:line 実測）

## Review Summary

設計は「1 分岐の式是正＋`prev` 型縮小＋檻の真実性是正＋正典記録＋実機受け入れ」という最小是正（Option A）に収束しており、要件 6 本すべてが Traceability 表で実現要素（C0〜C5）へ写像されている。設計が主張する file:line・期待値・不変量を現行 tree に対して全数照合した結果、実質的な誤りは検出されなかった（詳細は下記「検証結果」）。実装への手戻りリスクを持つ残件は、従判定ツールの再利用に小改修が要る 1 点のみである。

## 検証結果（doc 主張の file:line 裏取り・記憶則適用）

| 設計の主張 | 実測結果 |
|---|---|
| P2 欠陥式 = `resolver.rs:155-158`（`Some((prev_x, prev_w)) => prev_x.saturating_sub(prev_w)`） | **一致** ✓（:157 が欠陥式そのもの） |
| `prev: Option<(i32, i32)>` = :131-132・更新 = :178 | **一致** ✓ |
| モジュール doc 式引用 :98-102・インライン :151-152 | **一致** ✓（`base_x(n≥1) = char_x(n−1) − w(n−1)（2.9）` を実記載） |
| P5 ハンク :180-188 非接触（wpl 領分） | **一致** ✓（roadmap.md:95 の干渉台帳と同一範囲表記。P5 コード実体は :180-190 だが台帳規約に整合） |
| `t_r2_scope_chain_defaultx_zero_stays_adjacent` :130・不等幅 400/320/200・現行期待 `x0−w0`／`(x0−w0)−w1` | **一致** ✓（是正後期待 `x0−w1`／`x0−w1−w2` の算術も正） |
| `t_r2_chain_defaultx_offsets_leftward_from_base` :175・`t_r4_free_position_feeds_scope_chain` :524・`t_r6_chain_uses_clamped_previous_position` :363（assert 不変・:377 コメントのみ追随） | **一致** ✓（t_r6 は是正後も `x0−w1` が左外→クランプで assert 不変が成立） |
| DD3 否定 assert（:156-160）が是正後も有効 | **成立** ✓（`x0−w1 ≠ wa.right−w1`、w0≠0 ゆえ） |
| DPIS = [96,120,144,192]（`resolver_test_support.rs:4`） | **一致** ✓（要件 2.4/3.5 の行列と同一） |
| `prepare_emo2_returns_two_scope_placements` :57・`s1.char_pos (1052,640)` :80・`s1.balloon_pos (1198,565)` :84・`s1.balloon_offset (146,−75)` :95・導出コメント :38-51 | **一致** ✓。是正後値の算術検証: `1486−336=1150` ✓・右置き基準 `1150+336=1486`＋wp `−190`＝`1296` ✓・offset `1296−1150=146` 不変 ✓ |
| `prepare_emo2_matches_ssp_balloon_offsets_at_dpi_120` :87 が char 絶対位置を assert しない | **一致** ✓（assert 対象は char_size・balloon_size・balloon_offset・相対恒等式のみ。R4.2 の無改変合格檻として妥当） |
| `persist.rs:397-409` の `merge_scope restore` ログ・`default_char_x` = resolver 出力（:402） | **一致** ✓（`default_char_x = placement.char_pos.x`。move 演出・保存位置復元のいずれの汚染も受けない判定チャネルの選定は正しい。ログは無条件発火＝プロファイル削除時も出る） |
| COMPAT §8 の体裁先例（kero-balloon R3.8 行・列構成 項目｜裁量｜根拠｜出典 spec） | **一致** ✓（§8 表実在・R3.8 行が「否定した先行 AC の名指し・アーカイブ非改変・オラクルと測定値」の同型を提供） |
| 上書き対象 R2.9 の実在（`completed/areka-P0-window-placement/requirements.md`） | **一致** ✓（「scope1（相方）を scope0 のサーフェス画像幅ぶん左へずらした位置へ置く（SSP de-facto…）」を実記載） |
| DPI 120 檻の寸法錨 543×859／420×500 | **算術一致** ✓（434×5/4=542.5→543・687×5/4=858.75→859・round half away from zero・336×5/4=420・400×5/4=500） |
| `tools/measure-ssp-rects.ps1` 実在・証跡ログ 8 本実在 | **一致** ✓（spec ディレクトリで確認） |
| scg⇄wpl 直列必達（scg 先）・W6.5 rebase | **一致** ✓（`.kiro/steering/roadmap.md:68/:70/:95`） |

## Critical Issues（最大 3）

🔴 **Critical Issue 1**: 従判定ツールの再利用は無改修では不成立（プロセス名ハードコード）
**Concern**: 設計 C5 は「`tools/measure-ssp-rects.ps1` を areka の窓へ向けて」従判定（外部矩形実測）を行うとするが、同ツールの `Get-SspWindows` は `Get-Process -Name ssp`（:42）をハードコードしており、areka プロセスの窓は列挙されない（無改修だと "(no visible ssp windows)" を返し証跡が空になる）。
**Impact**: 従判定は補強証跡に限定され合否は主判定（ログ grep）で確定するため GO を覆さないが、R6.4 の字義「是正後の二体の**窓矩形を実測**し」を満たす唯一のチャネルが黙って空振りする恐れがある。
**Suggestion**: 実装フェーズ（tasks）で `-ProcessName` パラメタ化（既定 `ssp`）の小改修を C5 の前提タスクに含める。読み取り専用の性質・既存証跡ログの再現性は変えない。
**Traceability**: R6.4（窓矩形の実測）
**Evidence**: design.md「C5: 実機受け入れ」従判定の節／`tools/measure-ssp-rects.ps1:42`

（他に GO を左右する問題は検出されなかった。上記 1 件のみ。）

## Design Strengths

1. **判定チャネルの汚染回避が構造的**: 実機主判定に `char_x`（復元後値）でなく `default_char_x`（`persist.rs:402`・resolver 出力そのもの）を採る決定は、ゴースト演出 `\![move]` と保存位置復元という 2 つの汚染源を同時に遮断し、追加実装ゼロで R6.2 の決定論判定を成立させる。是正前実機ログで同式が gap=123 を返す事実（before/after の対）まで確認済みで、判定式の妥当性が実データで裏付いている。
2. **退行防止が型と檻の二重**: `prev` の `Option<(i32,i32)>`→`Option<i32>` 縮小で欠陥式（前スコープ幅参照）の再導入をコンパイルエラー化し、さらに新設檻 `t_r2_unequal_widths_leave_no_gap` に欠陥式の否定 assert（不等幅入力でのみ判別可能）を置く。「名前が嘘をつくテスト」の根本原因（等幅入力では観測不能）への対処が要件 3.2 と正確に噛み合っている。

## Final Assessment

**Decision: GO**

**Rationale**: 是正の本体は SSP 実測（H1 確定・`ssp-oracle-notes.md` 正本）に基づく 1 分岐の式変更で、波及面（テスト 4 本＋フィクスチャ 1 本＋doc 引用）は設計時点の tree に対する file:line 照合で全数一致を確認した。要件 6 本の Traceability に欠落はなく、wpl との同一関数直列関係（P5 非接触・檻分離）も roadmap 台帳と整合する。Issue 1 は従判定（補強証跡）に限定された小改修であり、tasks で吸収可能なため GO を妨げない。

**Next Steps**:
1. `/kiro-spec-tasks areka-P0-scope-chain-gap` でタスク生成。その際 Issue 1（measure-ssp-rects.ps1 の `-ProcessName` パラメタ化）を C5 系タスクの前提に組み込む。
2. 実装は C1（式是正＋型縮小）→ C2/C3（檻）→ C4（§8）→ C5（実機）の依存順（design の Component 依存と一致）。

---

# Round 2: 要件 7／C6 の追補に対する設計検証

> 実施: 2026-08-13（kiro-validate-design・開発者指示による再実行）
> 対象: `design.md` の**要件 7／C6 追補ぶん**（Round 1 は要件 6 本・C0〜C5 の版に対する判定であり、追補は未検証だった）
> 照合: `requirements.md` 要件 7／実装コード（file:line）／最終ゲートの実機ログ 3 脚（§5.6）

## Review Summary

C6 は「初期配置は実表示寸が確定するまで暫定」という一貫した見立てのもと、判定を純関数へ、結線を薄いアダプタへ分けており、依存方向・単一位置ライター・丸め権威の非導入といった既存の構造規律をすべて守っている。実機 3 脚で意図どおりの挙動（移動 1 件／除外 2 種）が観測されており、実装リスクは低い。一方で、**設計文書が約束している範囲が実装の保証範囲より広い**箇所が 1 件あり、これは文書の限定で解消できる。

## 検証結果（doc 主張の file:line 裏取り）

| 設計の主張 | 実測結果 |
|---|---|
| 再解決は resolver の P2 式そのもの（`new_x(n)=x(n−1)−w(n)`） | **一致** ✓（`chain_finalize.rs:106`・`resolver.rs:161` と同型） |
| 先頭スコープは動かさない／Y は動かさない | **一致** ✓（`chain_finalize.rs:96`・`moved_default_pos` が Y 据置） |
| 反映は `move_window_to` のみ（唯一の位置ライター） | **一致** ✓（`drain_resnap.rs:352`。他の書き込み口なし） |
| 確定は一度きり（`ChainFinalized` で二度目以降 no-op） | **一致** ✓（`drain_resnap.rs:286`・実機で 3 脚とも 1 回のみ） |
| 明示的再配置の除外＝既定位置との一致判定 | **一致** ✓（`chain_finalize.rs:98`。実機脚ⓐで move 済み scope を除外・脚ⓒで復元 scope を除外） |
| 依存方向（placement ← emo2_boot の一方向） | **一致** ✓（`placement` 配下に `use crate::emo2_boot` は 0 件） |
| resnap の landing を待つ（部分適用しない） | **一致** ✓（`drain_resnap.rs:321` の寸一致ガード） |

## Critical Issues（最大 3）

🔴 **Critical Issue 1**: 「実表示寸の確定」の代理が「全スコープの**初回**表示＋resnap landing」であり、起動直後に面を差し替えるゴーストでは差替**前**に確定してしまう
**Concern**: C6 は確定点を「実表示サーフェス寸が判明した時点」と述べるが、実装の駆動条件は「全スコープが 1 度表示され、`WindowPos.size` が実表示寸と一致した最初のフレーム」である。ゴーストが起動直後に面 A→B（寸法違い）と差し替える場合、A の時点で条件が揃えば確定してしまい、B への差替で隙間が戻る。
**Impact**: 実機で隙間 0 が出たのは、emo2 のキャラ窓が**最初から定常面（`surface_id=1000`）を表示した**ためで（ゲート脚ⓑのログで確認・配置時の 868 は採寸値であって表示された面ではない）、**構造的な保証ではなく当該ゴーストの台本に依存した結果**である。要件 7 の目的文「起動が落ち着いた画面で二体が隣り合っている」を全ゴーストに対しては保証しない。
**Suggestion**: 挙動を変えるなら「起動後の安定を待つ」判定が要るが、それは 7.4 の「一度きり」と正面から衝突する。**最小対応は設計への限定明記**——確定点は初回表示の landing であり、その後の差替は（起動直後であっても）7.4 により是正しないこと、および emo2 で隙間 0 が成立する条件を明示する。
**Traceability**: 7.1（実表示寸での再解決）／7.4（確定後は再解決しない）
**Evidence**: design.md「C6 設計判断」第 2 項／`drain_resnap.rs:270-283`（駆動条件）

🔴 **Critical Issue 2**: 確定が**永久に見送られても無言**（waiting と stuck を区別する手段がない）
**Concern**: `finalize_chain_once_with` の見送り経路（`GhostWindows` 不在・窓不在・実表示寸未確定・寸不一致・`WindowPos` 欠損・非正寸）はすべて素の `return` で、ログを出さない。成功時のみ `debug!` が 1 行出る。
**Impact**: 起動中の見送りは正常な待ち状態ゆえ毎フレーム出すのは誤りだが、**確定が一度も起きなかった場合に痕跡が残らない**。隙間が開いたままの現地報告を受けても、確定が走らなかったのか走って移動 0 だったのかをログから判別できない。steering の「ログ無し失敗経路の禁止」が嫌う形。
**Suggestion**: 一定フレーム／時間を過ぎても未確定なら**一度だけ**理由つきで `info!`（または `warn!`）を出す。既存の一発ガードと同じ形（フラグ 1 個）で実装でき、正常系のログ量は増えない。
**Traceability**: 7.1／7.4
**Evidence**: `drain_resnap.rs:286-336`（全経路が bare return）

🟡 **Critical Issue 3**: 確定経路が **P4 クランプを経由しない**——「キャラ窓は必ず work area 内」の不変量が確定後の配置で失われる
**Concern**: 初期配置は `resolver.rs:179` が全 alignment で X を `[wa.left, wa.right−w]` へクランプするが、`finalize_chain` はクランプを持たず `move_window_to` で直接書く。先頭スコープが**広い面へ差し替わる**と後続は左へ寄るため、原理上 work area 左端を割り得る。
**Impact**: 完了済み `dpi-window-vanish` が扱った「窓が見えなくなる」症状と同族の経路が新設されている。emo2＋2880px 幅では発生しないが、狭い作業領域・多スコープ連鎖では起こり得る。design.md は自認して `windowposition-limit` へ申し送り済みだが、**その spec の着手までは出荷挙動に残る**。
**Suggestion**: 受容するなら発生条件を design に明記する。安価な緩和は「`new_x < wa.left` になる移動は見送る」1 行（work area を判定へ渡す必要があるが、クランプ規則そのものは wpl の領分のまま）。
**Traceability**: 7.1
**Evidence**: design.md「Revalidation Triggers」要件 7 の第 1 項（自認）／`chain_finalize.rs:106`（クランプなし）

## その他（GO を左右しない軽微な不整合）

- **Non-Goals の取り残し**: design.md:27 は「位置の追従・保存・**復元**の実装変更」を Non-Goal とし、:28 の改訂は `spawn.rs` のみを境界内へ移した。しかし 7.3⑶ の実装は `main.rs` の復元合成シーム（`restore_merged_placements`）を変更しており、同日に是正した「This Spec Owns」と文面が食い違う。**Non-Goals 側にも同じ限定を入れるべき**（`persist.rs` 本体が無改変であることは事実で維持できる）。

## Design Strengths

1. **追従の領分へ踏み込まずに定常隣接を得た**: 「一度きりの確定」という切り方により、`follow` の実装・`windowposition-limit`・DPI 遷移系と同じフレーム経路で競合することを避けている。判定を `(scope, 現在位置, 現在寸, 既定位置) → 移動指示` の純関数へ切り出したことで、GPU も World も要さない決定論檻 14 本で全分岐を固定できており、結線側 6 本＋復元シーム 2 本と役割が重複していない。
2. **除外判定にフックを増やさなかった**: 「現在位置が既定位置と一致するか」だけで、台本の移動指令・利用者のドラッグ・保存位置の復元という**由来の異なる 3 つの再配置**をまとめて除外している。move／drag 側へ通知を足す設計であれば、経路を 1 つ足すたびに漏れる形になっていた。実機 3 脚で 3 種すべての除外・非除外が観測されている。

## Final Assessment

**Decision: GO（条件つき）**

**Rationale**: 依存方向・単一位置ライター・丸め権威・純関数と結線の分離といった構造規律に違反はなく、要件 7.1〜7.6 はすべて実現要素へ写像され、実機 3 脚で意図どおりの挙動が確認されている。Issue 1 は**設計文書が実装より広く約束している**という限定漏れであり、文面の限定で解消できる（挙動変更を要しない）。Issue 2・3 はいずれも安価な硬化で、受容する場合も条件の明記で足りる。**GO の条件は Issue 1 の限定明記のみ**——これを入れないと、GO が「全ゴーストで定常隣接を保証する設計」への承認と読まれてしまう。

**Next Steps**:
1. Issue 1 の限定を design.md C6 と requirements.md 要件 7 末尾へ追記する（必須）。
2. Issue 2（一発診断ログ）・Issue 3（左端割り込みの見送り）を実施するか受容するかを裁定する。
3. その他の Non-Goals 取り残しを是正する。
4. 以上ののち `spec.json` の承認扱いを確定し、`/kiro-complete` へ。

## 条件の充足（2026-08-13・同日・開発者指示「自明な点は修正しタスクを調整」による）

| 指摘 | 処置 | 所在 |
|---|---|---|
| Issue 1（限定明記・GO の条件） | **充足** — 確定点の実装定義と限定を追記 | requirements.md 要件 7 末尾／design.md C6「確定点の実装定義と限定」 |
| Issue 2（無言の見送り） | **実施をタスク化 → 2026-08-13 実装済み** — 有界の待ち 600 フレーム（60Hz で約 10 秒）を超えたら理由つき `warn!` を一度だけ。判定は純関数 `note_chain_deferral`、走査は理由列挙型 `ChainDeferReason` を返す形へ組み替え | tasks.md **6.5**（完了）／`placement/chain_finalize.rs`・`emo2_boot/frame/drain_resnap.rs`／檻 4 本（判定 2・結線 2、退行注入で両方向に較正済み） |
| Issue 3（P4 クランプ非経由） | **受容** — 独自緩和は第 3 の半端なクランプ実装となり wpl の設計判断を先取りするため。発生条件を明記 | design.md Revalidation Triggers（C6 第 1 項）「受容の記録」 |
| その他（Non-Goals 取り残し） | **是正** — `main.rs` 復元合成シームの変更を Non-Goals 側にも結線 | design.md Non-Goals :28 |

以上により GO の条件は満たされ、本判定は**タスク 6.5 の消化を残して GO** と読み替えてよい。

**2026-08-13 追記: タスク 6.5 は消化済み**（上表 Issue 2 の欄を参照）。これにより Round 2 の条件・宿題はすべて片づき、**無条件の GO** となる。残るのは開発者による追認（tasks.md「開発者裁定アジェンダ」#1）のみ。
