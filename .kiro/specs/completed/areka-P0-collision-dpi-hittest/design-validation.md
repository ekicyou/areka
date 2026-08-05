# 設計バリデーションレポート: areka-P0-collision-dpi-hittest

> 実施日: 2026-07-31 ／ 対象: `design.md`（確定版）× `requirements.md`（R1〜R6・45 criteria）× `research.md` × 現行コードベース
> 手法: design の主張するコードアンカー・数式・裁定済み判断を **Grep/Read で全件現物照合**（design を鵜呑みにしない）。実施者: kiro-validate-design（非対話・レポートはディスクへ永続化）

---

## Review Summary

設計品質は極めて高い。全 45 criteria が Traceability 表で実現要素へ 1:1 対応し、DD-1〜DD-11 の裁定は要件ディスカッションの確定事項（DD-4 CLOSED・DD-8 VOID・f32 排除・k 明示引数の純関数）を一切蒸し返さずに実装形へ落としている。本レビューで design が引用する**コードアンカー約 30 箇所を全て現物照合し、ドリフトはゼロ**、DD-1 の数式も数学的に検証して主張どおり成立した。残る指摘は戻り値域の縮小規約 1 点（実装時に 1 行で吸収可能）のみである。

### 現物検証の結果（要点）

| 検証項目 | 結果 |
|---|---|
| (a) DD-1 式の k=1.0 厳密恒等 | **成立**。num=den=1 で `s(v) = (2v+1).div_euclid(2) = v`（負値含む全 i64。例 v=−3: `(−5).div_euclid(2) = −3`）。R1.5/R1.9/R3.4 の根拠は正確 |
| (b) 閉区間境界の k 非依存保存（R2.3） | **成立**。分子 `(2v+1)·den` は v に対し狭義増加・正の `2num` での床除算ゆえ s は単調非減少 → サーフェス px 閉区間の逆像が物理空間で連続区間になる。代表値も再計算一致（k=2: 100→50・101→50／k=5/4: 1→1・6→5／k=7/6 端: s(31)=27＝native 27px の範囲外→自然に None） |
| (c) R-1 静的解決の裏付け | **実在**。`wintf/src/ecs/window/components.rs:190-208`（`GetDpiForSystem()` で CreateWindowExW 前に DPI component 事前設定）・`wintf/src/ecs/window/window_handle.rs:207-246`（生成直後に `GetDpiForWindow(hwnd)` で差分時補正）を現物確認。「初回表示から実 k」の主張はコードに支持される |
| (d) W5 同居分析（balloon.rs 異ハンク） | **概ね成立**（Issue 2 参照）。改訂対象コメント行（:136-137/:154/:279/:322/:445/:481）は全て実在し「k=1.0 素通し」の誤理由を現に含む。変更はコメント＋`#[cfg(test)]` のみで判定コード無変更＝R6.7 例外条項の範囲内 |
| (e) i128 中間の桁溢れ | **安全**。`(2v+1)·den ≤ 2^64·2^32 = 2^96 < i128::MAX`・`num ≥ 1`（`ScaleRatio` 不変条件・scale.rs:45-48 現物確認）ゆえゼロ除算なし。ただし**戻り値の i64 縮小**に未規定あり（Issue 1） |
| その他アンカー | `presenter.rs` :108/:678-681/:687/:705/:858-861/:867、`hit.rs` :42-44/:57/:72・檻 :130/:204、`hit_region.rs` :54-56/:69-73、`mod.rs` :97/:104-105/:153-159/:184-190、`choice.rs` :260-289（f32 の `* k`）、`collision-probe.rs` :446-448/:477/:490-502/:551-561、`throttle.rs:58`、`click_selection` balloon.rs:242 — **全て design の引用と一致**。`ScaleRatio` の `PartialEq/Eq` は既に derive 済み（scale.rs:43・design の条件付き記述と整合）。`areka-emo-present/lib.rs` に `ScaleRatio` 再輸出が無いことも確認（再輸出追加の必要性は正確） |

---

## Critical Issues（≤3）

🔴 **Critical Issue 1**: `unscale_coord` の戻り値域（i128 → i64 縮小）の規約が未指定
**Concern**: Service Interface は `pub fn unscale_coord(self, v: i64) -> i64`・「Preconditions: なし（全 i64 で定義・負値可）」「panic なし」を宣言するが、k<1（num<den・例 author_dpi 192 × 96dpi モニタ＝k=1/2）では `s(v) ≈ v·den/num` が極値近傍の v で i64 に収まらない。i128 中間からの最終縮小を `as` キャストにするとラップ（単調性 postcondition が破れる）、`try_into().unwrap()` にすると panic（panic なし宣言が破れる）。
**Impact**: 実座標は Win32 の i32 域に束縛されるため実害は起きないが、design が「全 i64 で定義」「単調非減少」「panic なし」を檻で固定する以上、縮小規約を決めないと檻の期待値自体が書けない（テスト計画 scale.rs #1「i64 極値近傍」が k=ONE に限定されているのはこの穴の兆候）。
**Suggestion**: doc と実装に「i64 域へ**飽和**（saturating）で縮小する。単調非減少は非飽和域で成立し、飽和域では定値」と 1 行明記し、k<1×極値の飽和挙動を檻に 1 本追加する（tasks 生成時に反映すれば足りる。設計の作り直しは不要）。
**Traceability**: R2.1（決定性）・R2.5（panic なし）
**Evidence**: design.md「DD-1 桁溢れ」および「ScaleRatio::unscale_coord — Service Interface / Preconditions」

🔴 **Critical Issue 2**: balloon.rs のコメント改訂ハンクが choice-select-events の drain 席と隣接（git 文脈衝突リスク）
**Concern**: 改訂対象の :136-137 は、W5 同居 `choice-select-events` の編集席（Inbox :130・`ChoiceSelection` :43）と 6 行以内で隣接する。「異ハンク」は論理的には正しいが、diff の文脈行（±3 行）が重なり、両者が並走着地するとテキスト衝突が現実に起き得る。
**Impact**: 判定挙動の衝突ではなく rebase 時の機械的 conflict に留まる（設計の「着地順に従い後着側が rebase して吸収」裁定で解消可能）。ただし実装者が conflict 解消時に相手 spec のハンクへ誤って踏み込む事故は本 spec が最も警戒する類型（誤一般化で balloon へ ÷k を足す事故と同根）。
**Suggestion**: tasks の balloon.rs タスクに「conflict 解消時は自分の増分＝コメント＋`#[cfg(test)]` のみを保持し、drain/status 系ハンクには一切触れない」を明記する。設計変更は不要。
**Traceability**: R6.7（同居エスケープ条項・例外条項）
**Evidence**: design.md「DD-11」「バルーン明文化＋檻」／現物 balloon.rs:130-137

（Critical Issue 3: なし——上記 2 件以外に成功を有意に脅かす欠陥は発見できなかった）

---

## Design Strengths

1. **裁定の数理的裏付けと現物整合が完全**: DD-1 は「resample が実際に用いた画素中心写像の最近傍逆」という定義から導出され、本レビューの独立再計算（恒等・単調性・代表値・k=7/6 端）で全て一致した。約 30 箇所のコードアンカーにドリフトが 1 件も無く、research.md §9 の「設計前現物再検証」が実際に機能している。R-1 を実測主張でなく wintf の生成時初期化コード 2 箇所の現物で解決し、probe から DPI 追従駆動を削った単純化も正当。
2. **欠陥クラスそのものを構造で封じる檻設計**: 「÷k の呼び忘れ」が本仕様の欠陥クラスであることから、縮約単体でなく**縮約＋照合の合成純関数**を檻の最小単位に置き（DD-6）、正常経路の縮約実行点を 1 箇所に限定して二重縮約を構造排除。さらにバルーン側には「÷k を足すと外れる」逆向きの退行檻（R3.7）を置き、シェル側是正がバルーン側破壊へ波及する誤一般化を両側から挟んでいる。W6.5 への公開面申し送り（`unscale_coord` のみ・num/den アクセサ棄却）も先着規律として的確。

---

## Final Assessment

**Decision: GO**

**Rationale**: 要件 45 criteria への被覆は完全で、確定済み裁定（DD-4 CLOSED・DD-8 VOID・f32 排除・純関数化）と矛盾せず、引用アンカー・数式・依存方向の全検証にドリフトゼロ。指摘 2 件はいずれも設計の骨格に触れない実装時規約の追記（飽和縮小の明記・conflict 解消規律の明記）であり、tasks フェーズで吸収可能な受容可能リスクである。

**Next Steps**:
1. `/kiro-spec-tasks areka-P0-collision-dpi-hittest` でタスク生成へ進む
2. その際 Issue 1（`unscale_coord` の i64 飽和縮小＋檻 1 本）と Issue 2（balloon.rs conflict 解消規律）をタスク記述へ反映する
