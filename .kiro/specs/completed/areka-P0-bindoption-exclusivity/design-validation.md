# 設計バリデーションレポート: areka-P0-bindoption-exclusivity

**実施日**: 2026-08-11（kiro-validate-design・非対話実行）
**入力**: requirements.md（確定）・design.md（確定）・research.md・steering・現行コードベース（file-slimming PR#103 マージ後）

## アンカー実測（レビュー前検証）

design/research が主張する file:line を文書信用でなく現物 Grep/Read で抜き取り検証した。**全件一致**:

- 2 値分岐 `if on && bind_resolver.is_mustselect(...)`: `crates/areka-seriko/src/actor.rs:367` ✓（Changed=info :374-384／Unchanged=debug :387-396 ✓）
- `parse_bindoption_mustselect`（multiple 破棄）: `crates/areka-parsers/src/package/resolve.rs:196-205`・走査 :172-188 ✓
- `is_mustselect` :121-127・`empty()` :90・`category_ids` :134-146: `crates/areka-seriko/src/bind.rs` ✓
- `BindResolver::new` 呼出元台帳: **実測 8/8 完全一致**（assets.rs:267 本番唯一・bind.rs:356/:370・actor_bind_loop_tests.rs:64/:76/:202・bind_e2e.rs:123/:244。他に new 呼出なし）
- `empty()` areka 側 4 箇所（balloon_test_support.rs:73・frame_test_support.rs:71・emo2_boot/mod.rs:374・spine.rs:671）✓・actor_dispatch_tests は empty() のみ使用＝無改変見込みの主張 ✓
- `apply_bind_exclusive` :342-360・`commit_bind` :374-402（カテゴリ非依存の汎用形）: state.rs ✓
- looper bind ゲート :215 ✓・**ルーパー発火ログは info レベルで `animation_id` を含む**（looper.rs:227）＝J1 判定式の観測データは `RUST_LOG=info` 実走で成立 ✓
- 檻 `bind_non_mustselect_accumulates_via_actor`（実体＝腕/肩の**異カテゴリ**加算）: actor_bind_loop_tests.rs:195 ✓＝D6 改名・実体保持の方針は実体と整合

## レビューサマリ

既存の「転写→構築→純関数判定→単一発行点」直列を保ったまま、述語の意味反転（is_mustselect → policy 3 値）と搬送追加に変更を局在させた、実装準備度の高い設計である。要件 34 ID 全件のトレーサビリティ・(on×policy) 直積 6 セルの分岐網羅・意味反転監査（§7.1・独立 grep と一致）・W6 並走無干渉条件（`empty()` 署名不変）の境界固定まで揃い、steering 規律（判断分岐の決定論檻・ログ無し失敗経路の禁止・有界 auto-exit＋ログ grep・新規依存なし）に適合する。指摘は実機サインオフ判定式の縁の未定義と、先送り語彙の登記漏れ 1 件に限られ、いずれも設計骨格を揺るがさない。

## Critical Issues（≤3）

🔴 **Critical Issue 1**: 実機サインオフ判定式 J1/J2 の縁が未定義（偽赤リスク）
**Concern**: J1 は「各まばたき発火の直前の最新まばたき Changed(info) と id 一致」を走査するが、**最初の Changed 以前の発火**や **Hidden scope 中の StateOnly 適用**（debug のみ・info 痕跡なし）後の発火の判定規則が未定義。J2 の |Changed(まばたき)−Changed(目)| ≤ 2 は「目とまばたきが常に主題ペアで変わる」ことを暗黙前提とし、複数表情が同一まばたきパーツを共有するゴースト運用では正しい挙動でも閾値を超え得る。
**Impact**: 5.1 の「決定論的に判定できる形」が縁ケースで崩れ、偽赤ならサインオフが不必要に停滞、規則の場当たり補正なら判定の決定論性を失う。
**Suggestion**: タスク分割時に走査規則を確定する——(a) 最初のまばたき Changed 以前の発火の扱い（emo2 静的既定に 14xx なし＋looper ゲートゆえ「Changed 痕跡なしの発火＝即赤」で良いか）、(b) StateOnly 経路の痕跡の扱い、(c) J2 閾値を是正前保全ログの目/まばたきペア実測から較正。既知ケース較正（是正前ログで必ず赤）は維持。
**Traceability**: 5.1, 5.2, 5.4
**Evidence**: design.md §Testing Strategy「Real Machine Sign-off」J1/J2

🔴 **Critical Issue 2**: mustselect「ちょうど 1 個」の起動時保証が語彙未登記
**Concern**: 正典の mustselect は「必ず 1 つ選択」だが、本設計は off 無視（D1）のみ拾い、**mustselect カテゴリに `default,1` が 1 つも無い shell では起動時ゼロ個のまま**という残余の正典乖離を D7（char2+）のような先送り語彙として登記していない。design §資産構築の既定集合注記は非宣言カテゴリの重複側のみ扱う。
**Impact**: emo2 では既定集合が mustselect カテゴリを被覆し実害なしだが、steering「正典機能の先送りは完全語彙＋追跡明記」に照らし、次の読み手が「mustselect は完全適合済み」と誤読する余地を残す。
**Suggestion**: 設計ディスカッションで一文の語彙登記（D7 同型・「起動時充足は shell の default 宣言に委譲＝既存縮退維持」）を追補するか、roadmap 追記（6.3）に含める。要件改訂・実装変更は不要。
**Traceability**: 3.2（隣接）・6 章の登記趣旨
**Evidence**: design.md D1／§build_boot_assets 設計上の注記

## Design Strengths

1. **アンカー精度と反転監査の実証性**: `BindResolver::new` 8 呼出元台帳・`empty()` 監視 4 箇所・意味反転監査（§7.1「期待値変更ゼロ」）が本レビューの独立 grep と全件一致。atomic 追随の失敗モード（取り違え・漏れ）を `BindOptionDecls` 名前付き構造体とコンパイル結合で構造的に潰している。
2. **最小差分で正典 3 値を貫通させる分岐設計**: `apply_bind_exclusive`/`commit_bind` 無改変再利用＋(on×policy) 直積表で、是正の全分岐が決定論檻に写像可能（steering「檻に入れるのは判断分岐のみ」に正確に一致）。D1 の warn 採用根拠（debug では info 実走で不可視＝握り潰しの再来）も本件の教訓と整合する。

## Final Assessment

**Decision: GO**

**Rationale**: アーキテクチャ整合・要件被覆・境界固定・steering 適合のいずれにも致命的欠陥がなく、file:line 主張は全件現物一致。指摘 2 件はサインオフ走査規則の具体化（タスク工程で確定可能）と一文の語彙登記であり、設計骨格の再作業を要しない。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で Issue 1 の走査規則（縁 3 点）と Issue 2 の登記先を裁定
2. `/kiro-spec-tasks areka-P0-bindoption-exclusivity` でタスク生成（J1/J2 走査スクリプトの規則確定と既知ケース較正をタスク化）
