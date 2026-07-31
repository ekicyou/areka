# 診断レポート＝確定台帳（areka-P0-dpi-window-vanish）

> 対象要件: **2.1 / 2.2 / 2.3 / 2.4 / 2.5 / 2.6 / 2.8 / 2.9**（＋ 5.4 の赤→緑実行記録の登記先）
> 対象タスク: **4.2（本タスク＝静的構造証跡の先行登記）**・4.3（S1 赤）・4.4（S2 赤）・4.5（実機 2 セッション）・7.1（赤→緑）・7.4（是正後の実機再サインオフ）
> 手順の正本: `diagnosis-procedure.md`（**手順は書かない**。判定語・grep 規則・起動設定はすべてあちらが持つ）

本書は本仕様における**確定の台帳を一元化**したものである（requirements.md「Introduction」・design.md「成果物 > diagnosis-report.md」）。消失原因に関する確定は、どのクラスで確定したかを明記したうえで、**すべて本書 1 つに集約する**。

---

## 0. 本書の規約（先に読む）

### 0.1 確定の証跡は 2 クラスある（2026-07-31 開発者裁定）

| クラス | 定義 | 再検証の手段 |
| --- | --- | --- |
| **静的構造証跡** | コード読解のみで確定できる欠陥。読む者が誰であれ `file:line` を引いて同じ結論に到達できるもの | 引用箇所を開いて構造を読む |
| **実機証跡** | 実機ログの該当行の引用によって確定するもの（OS が実際に提示する値・実モニタ列挙・操作の因果） | `diagnosis-procedure.md` の手順で再採取する |

「憶測修正の禁止」（requirements.md Introduction・Req 2.7）が禁じているのは*根拠なき修正*であって、*実機ログという媒体*ではない。現に 2026-07-18 の実機診断は `trace!` 水準の見落としで**偽陰性**を出し「反証済み」という誤結論を生んだ。よって本仕様は**静的構造証跡を実機ログと同等以上に強い確定手段として扱う**（Req 2.8 がこれを制度化している）。

### 0.2 **本書で最も重要な規則**——「再現しない」は静的確定分を落とさない

Req 2.6 と requirements.md「Boundary Context > 診断の縮退条項」が定める:

> `SESSION-QUOTA: PASS` の 2 セッション（Req 1.9 の受理回数下限を踏破したもの）の双方で消失痕跡が検出されず「再現しない」と結論した場合、**そこで除外できるのは「実機でしかその真偽を確定できない残余仮説」に対する追加修正だけ**である。**§1 に登記された静的構造証跡（S1・S2・S3・S3′）の是正と、それに対応する回帰檻は、除外の対象ではない。**

したがって、実機セッションが綺麗（`VANISH-TRACE: NONE`）であったことを根拠に、**タスク 5.1・5.2・6.1・6.2 とその檻（7.1・7.2）を取り消してはならない**。これらは §1 の登記によって既に着手条件を満たしている。実機採取（4.5）が §1 に対して果たす役割は「確定の可否を賭ける関門」ではなく、**因果の確認（Req 2.9）と、静的には確定できない残余仮説の確定**である。痕跡が観測されなくても §1 の確定は取り消さない（Req 2.9 の明文）。

Req 5.1 も同じ側から支える——「Requirement 2.8 の S1〜S3 は本項（96 以外の DPI での回帰檻）の対象として**既に確定している**」。Req 5.4 は S1・S2 について赤→緑の実行提示を要求している。いずれも実機採取の結果を条件にしていない。

### 0.3 引用の規約（本書のメンテ契約）

- 引用は `path:line` に加えて**必ず構造名**（関数・型・定数・分岐の識別子）を併記する。行番号は編集で動くが構造名は残るため、台帳が「静かに嘘になる」度合いを下げられる。
- §1 の各項目は「本タスク（4.2）実施時点のツリーで、当該行を実際に開いて確認した」ものである。設計時点（design.md 執筆時）の行番号は**タスク 1.4／2.1／2.2／3.1／3.2 の着地で全面的に陳腐化している**ため、本書は design.md の行番号を転記せず、**再測定した実値のみ**を載せる（対応は各項目の「引用の再測定」に残す）。
- 以後のタスクが §1 の引用先を編集した場合（とりわけ 5.1／5.2／6.1／6.2 の是正）、**同じコミットで本書の引用も更新する**こと。
- 本書の測定基準コミット: **`3310e1d`**（タスク 4.1 着地時点＝Phase A 途中。S1/S2/S3/S3′ の是正はいずれも未投入）。

---

## 1. 静的構造証跡（確定済み・実機採取の結果に依存しない）

> **本節の 4 件は、実機ログを 1 行も参照せずに確定している。** 確定日 2026-07-31・確定タスク 4.2・確定手段＝コード読解（下記 `file:line` を開けば第三者が同じ結論に到達できる）。

### 1.0 一覧

| ID | 欠陥（1 行要約） | 未充足にする受入基準 | 主たる所在（構造名） | 是正タスク | 赤→緑の檻 |
| --- | --- | --- | --- | --- | --- |
| **S1** | DPI 変化後のキャラ窓位置の再射影が、接地点規約の **X 成分を再計算せず OS 提示値を素通しさせる** | **4.3**（波及: 4.2） | `WM_DPICHANGED`（wintf）＋ `BottomSnapPolicy::resolve`（areka） | 5.1 | 4.3 → 7.1 |
| **S2** | 位置の再射影が**窓寸の再導出結果に条件付けられて**おり、再導出結果が得られない経路で位置の再射影ごと欠落する | **4.1・4.2・4.6** | `dpi_phase_with` の `Some` ゲート | 5.2 | 4.4 → 7.1 |
| **S3** | キャラ窓の**水平方向に可視性の不変条件が無く**、work area 解決の最近傍フォールバックが「どのモニタにも属さない」を無観測で吸収する | **3.1**（波及: 3.2） | `resize_window_to` ／ `work_area_for_window` | 6.1 | 7.2 |
| **S3′** | **バルーン矩形の可視性がどの経路でも検査されない**（追従は offset 恒等式のみを適用する） | **3.4** | `follow_balloon` ほかバルーン書込 3 経路 | 6.2 | 7.2 |

「未充足にする受入基準」の欄は、**Req 2.8 が明記している対応**を太字で載せている（S3′ は Req 2.8 が「少なくとも S1〜S3」と定めた枠外の追加登記であり、対応 AC は design.md「既存アーキテクチャ分析（欠陥の構造）」の表と「成果物 > diagnosis-report.md」が 3.4 と定める）。「波及」は本タスクの読解で併せて未充足と判断したもので、Req 2.8 の明文ではない——各項目の本文で根拠を示す。

### 1.1 S1: 接地点の X 成分を再計算せず OS 提示値を素通しする

**未充足 AC: 4.3**（If OS が DPI 変化に伴う推奨位置を提示した場合, the areka アプリケーション shall その推奨位置を最終位置としてそのまま残さない）
**波及: 4.2**（処理完了時の最終位置が接地点規約に従う値と一致すること——X が OS 由来である限り一致しない）

#### 欠陥の構造（4 段の連鎖・すべて再測定済み）

| 段 | 所在（`file:line`） | 構造名 | 何が起きるか |
| --- | --- | --- | --- |
| ① OS 提案位置を実窓へ**無条件で**書く | `crates/wintf/src/ecs/window_proc/window_pos.rs:369-379` | `WM_DPICHANGED`（`:285`）内の `guarded_set_window_pos(hwnd, None, suggested_rect.left, suggested_rect.top, 0, 0, SWP_NOSIZE \| SWP_NOZORDER \| SWP_NOACTIVATE)` | 窓ごとの分岐が存在しない。ゴースト窓であっても OS 提案の `left`／`top` がそのまま実窓へ入る |
| ①' 書込コンテキストも**無条件で**立つ | `crates/wintf/src/ecs/window_proc/window_pos.rs:343-346` | `DpiChangeContext::set(DpiChangeContext::new(new_dpi, suggested_rect))` | 直後の `WM_WINDOWPOSCHANGED` が「DPI 由来の外部変更」として扱われる |
| ② echo が `WindowPos.position` を**汚染**する | `crates/wintf/src/ecs/window_proc/window_pos.rs:109` ／ `:143-148` | `let use_bypass = is_echo && dpi_context.is_none();` → `window_pos.position = Some(corrected_pos);` | `dpi_context` が `Some` のため bypass されず、①で書かれた OS 由来座標が ECS 側の `WindowPos.position` へ landing する |
| ③ areka の再射影が汚染された X を**生位置として読む** | `crates/areka/src/placement/follow.rs:867-883` | `resize_window_to` 手順 3（`let Some(wp) = world.get::<WindowPos>(char_window)` → `let Some(pos) = wp.position` → `raw`） | 「直前に areka が確定した接地点」ではなく「OS が書いた位置」が `raw` になる |
| ④ 射影が **X を素通しする** | `crates/areka/src/placement/follow.rs:106-111`（`x: raw.x`＝`:107`） | `impl DragPositionPolicy for BottomSnapPolicy` の `resolve`（`:86-113`）。`project_anchor`（`:149`）は `Anchor::Bottom` を `:162-164` で全面委譲する | `Bottom` 射影は Y のみ（`wa.bottom - h`）を再計算し、**X は入力をそのまま返す**。汚染された X が最終位置として残る |

補助的に `resize_window_to` 手順 3b（`follow.rs:892-901`）が「旧寸の中央 x → 新寸の中央 x」へ付け替えるが、これは**汚染された X を基準にした付け替え**であり、X 成分を接地点規約から再導出するものではない（変換は寸差分のみに依存する）。

#### 是正機構は存在するが**未配線**である（＝欠陥は成立したまま）

タスク 2.1 が純関数と契約を新設済みだが、いずれも `WM_DPICHANGED` から呼ばれていない:

- `crates/wintf/src/ecs/window_proc/dpi_helpers.rs:32` `dpi_suggested_position_decision`——**本番呼出ゼロ**の実証は `crates/` 全 grep で一致がすべて定義行・doc・`#[cfg(test)]` 内（同ファイルのテストモジュールは `:150` 以降）であること。直上 `:31` の `#[allow(dead_code)]` は状況証拠にすぎず**根拠に使わない**（同属性は本番呼出のある `follow.rs:148` `project_anchor`・`:822` `resize_window_to` にも付いており、属性の有無は呼出ゼロを意味しない）
- `crates/wintf/src/ecs/window/dpi.rs:147` `DpiSuggestedRectPolicy`（`ApplyPosition` 既定／`ExternalAuthority`）——ツリー全体で areka 側の付与箇所が**存在しない**（`crates/` 全 grep で areka の一致ゼロ）

すなわち上表①の分岐は現在も存在せず、`window_pos.rs:359` の `let applied = true;` はその事実の表示である（`diagnosis-procedure.md` §3.3 が「`applied=true` を『政策判断が働いた』と読むのは誤り」と明記している当該定数）。

#### なぜ dpi=96 では隠れるか（Req 5.1／5.4 の根拠）

同一 DPI・同一モニタ内では OS 提案矩形の `left` が現在位置と一致するため、①〜④の連鎖を通っても最終 X が変わらない。**X の食い違いは「提案 X ≠ areka 確定 X」となる混在 DPI のモニタ跨ぎでのみ現れる**。これが「96 の自己整合が欠陥を隠す」性質そのものである。

#### 引用の再測定

design.md「既存アーキテクチャ分析」の表は `follow.rs:810-826`（`raw`）・`follow.rs:85-112`（X 素通し）・`window_pos.rs:359-369` を挙げていた。タスク 1.4／2.1／3.2 の着地で全て移動しており、現ツリーの実値は上表のとおり（`follow.rs:867-883` ／ `follow.rs:86-113` ／ `window_pos.rs:343-379`）。

### 1.2 S2: 位置の再射影が窓寸の再導出結果に条件付けられ、得られない経路で欠落する

**未充足 AC: 4.1（DPI 変化前後で接地点＝下端中央を保つ）・4.2（処理完了時の最終位置が接地点規約に従う）・4.6（不可視中に DPI が変化した窓は、可視化時点で規約準拠の位置と変化後 DPI 相当の寸で表示する）**

#### 欠陥の構造

| 所在（`file:line`） | 構造名 | 何が起きるか |
| --- | --- | --- |
| `crates/areka/src/emo2_boot/frame.rs:854-857` | `dpi_phase_with`（`:801`）の `if let Some(new_size) = source.refresh_scale_report(world, target) { reconcile_window_size(..., PlacementRoute::DpiReproject); }` | **`Some` のときだけ** `reconcile_window_size` を呼ぶ。`None` のとき当該窓に対して**何もしない**——寸を触らないだけでなく、**位置の再射影も行わない** |
| `crates/areka/src/emo2_boot/frame.rs:704-739` | `reconcile_window_size` → `GhostWindowKind::Char => resize_window_to(...)`（`:734`） | 位置の再射影（射影 T の再適用）は `resize_window_to` の内部にしかない＝上のゲートの**下流**にある |
| `crates/areka-emo-present/src/presenter.rs:754` | `EmoPresenter::refresh_scale` | `None` を返す経路が **5 つ**ある（下記） |

`refresh_scale` が `None` を返す経路（すべて再測定済み・`presenter.rs`）:

1. 未登録 target（`:757` の `self.targets.get(&target_id)?`）
2. **k 不変**（`:772-775`）
3. **不可視**（`:776-782`・Hide／全透明退化を蘇らせない）
4. **再表示入力なし**（`:783-789`・`last_show` が `None`＝一度も表示が成立していない）
5. 再表示が成立しなかった（`:796-814`）／成立したが丸め後の物理寸が同じ（`:818` の `take_pending_resize` が `None`）

このうち **3（不可視）と 4（未表示）は Req 4.6 が名指しで扱う状況**であり、**5 の「k は変わったが丸め後の寸が同じ」は正常系で日常的に起こる**（`refresh_scale` の doc が `frame.rs:850-853` で明言）。いずれの場合も窓の DPI は変わっている＝**接地点を保つべき work area が変わっている**のに、位置は一切再射影されない。

#### なぜ dpi=96 では隠れるか

DPI が変わらない（あるいは同一 work area 内の）走行では、旧 Y と「新 work area 下端 − h」が自己整合するため、再射影の欠落が観測されない。`120`／`192` の混在で work area 下端が変わって初めて Y のずれとして現れる（タスク 4.4 が採取する赤の非対称そのもの）。

#### Req 4.5 との関係（矛盾ではなく優先順位）

Req 4.5 は「再導出結果が得られない場合は窓位置と窓寸を変更せずに現状を維持する」と読める。しかし Req 2.8 が S2 を **4.1／4.2／4.6 を未充足にする欠陥**として登記していることが、この場合に「現状維持」より「規約復元」が優先することの要件上の根拠である（design.md「dpi_phase 位置/寸分離 > Risks / Req 4.5 との整合」）。是正後の正常系では**同寸・同 work area ゆえ `resize_window_to` のべき等 skip が書込ゼロで抜ける**＝ 4.5 はそのまま成立し、書込が発生するのは現位置が接地点規約に違反しているときだけである。

#### 引用の再測定

design.md は `frame.rs:835`・`presenter.rs:772-818` を挙げていた。`frame.rs` 側は 1.4／3.2 の着地で `:854-857` へ移動。`presenter.rs` 側は**当時の範囲がそのまま有効**（`:772` が k 不変ゲート、`:818` が `take_pending_resize`）——本 spec は当該ファイルを編集していないため。

### 1.3 S3: キャラ窓の水平方向に可視性の不変条件が無く、最近傍フォールバックが異常を隠す

**未充足 AC: 3.1**（ユーザーの明示的なドラッグ以外の要因で、キャラ窓の矩形がいずれのモニタ work area とも交差しない状態になることを防ぐ）
**波及: 3.2**（モニタ構成情報と実際の画面構成が食い違っている状態で位置を決めたとき、食い違いを**警告として記録**する）——Req 2.8 の S3 定義の後段「work area 解決が最近傍フォールバックによって『どのモニタにも属さない』状態を異常として観測させない」が指す機構は、そのまま 3.2 の「警告として記録」を未充足にする。design.md「Requirements Traceability」も 3.2 の充足担当として同じ機構（`work_area_for_window_with_origin` ＋ ガード warn）を指名している。

#### 欠陥の構造（前段: 不変条件が存在しない）

`crates/areka/src/placement/follow.rs:823-974` `resize_window_to` の位置決定は次の 3 段で完結しており、**可視性を検査する段がどこにも無い**:

- `:892-901` 手順 3b（下端中央の付け替え）
- `:906-907` `let new_pos = project_anchor(anchor, raw, new_size, snapshot);`
- `:909-919` べき等 skip → `:924-933` `enqueue_window_set_pos(...)`（単一ライターへの書込）

この関数へ**非ドラッグの配置系経路が 4 本**入る（いずれも `PlacementRoute` を引数で受ける・タスク 1.4／D13）:

| 経路 | 呼出点（`file:line`） | route |
| --- | --- | --- |
| アンカー変化 | `crates/areka/src/placement/follow.rs:1025-1033`（`anchor_changed_system`・`:1009`）**※定義済みだが本番スケジュール未登録**（`add_systems(anchor_changed_system)` は `follow.rs:5406/5453/5487/6019`＝すべて `#[cfg(test)]` 内）。S3 は下の 3 本の生きた経路だけで成立する | `AnchorChange` |
| 毎フレーム再スナップ | `crates/areka/src/emo2_boot/frame.rs:1190`（`resnap_from_sizes`・`:1162`） | `Resnap` |
| DPI 相の再射影 | `crates/areka/src/emo2_boot/frame.rs:856`（`dpi_phase_with`） | `DpiReproject` |
| 報告回収（drain 相） | `crates/areka/src/emo2_boot/frame.rs:1067-1073`（`reconcile_reported_sizes`・`:1011`） | `ReportedSizeReconcile` |

**4 本のいずれも、書き込む矩形がどれかの work area と交差するかを検査しない。** ユーザーが触っていなくても、射影の入力（`raw`＝S1 で汚染され得る／モニタ構成情報が陳腐化している）次第で全 work area 非交差の位置が書かれ得る。

#### 欠陥の構造（後段: 異常が観測されない）

`crates/areka/src/placement/follow.rs:1284-1286` `work_area_for_window` は判別付き版へ委譲したうえで **`.map(|(wa, _)| wa)` で判別を捨てる**。射影が使うのはこの薄いラッパ側である（`BottomSnapPolicy::resolve` の `:102`・`project_anchor` の `:182`）。

判別を落とされる先が `crates/areka/src/placement/follow.rs:1332-1347` の最近傍フォールバック（`min_by_key` で clamp 点自乗距離最小）であり、**ログを 1 行も出さない**。したがって「窓中心がどのモニタにも属していない」＝モニタ構成情報と実画面の食い違い、あるいは窓が既に可視領域外、という**異常の兆候が、正常な帰属と区別されないまま吸収される**。

#### 是正機構は存在するが**未配線**である

タスク 2.2 が純関数を新設済みだが、`crates/` 全 grep で**本番呼出はゼロ**（一致はすべて定義行・doc・`#[cfg(test)]` 内）:

- `crates/areka/src/placement/follow.rs:1417` `guard_visibility`（直上 `:1416` に `#[allow(dead_code)]`）
- `crates/areka/src/placement/follow.rs:1315` `work_area_for_window_with_origin`（判別を返すが、返り値の `WorkAreaResolution`（`:1299`）を消費する本番コードが無い）
- `crates/areka/src/placement/follow.rs:1355` `VisibilityVerdict`（`:1353` に `#[allow(dead_code)]`）

**純関数が在ることは S3 の充足ではない。** 配線（タスク 6.1）が入るまで、上記 4 経路は依然として無検査で書き込む。`diagnosis-procedure.md` §3.3 が「`ClampX`／`NearestFallback` の `warn!` はどの directive でも 0 行」と記載しているのは、この未配線の帰結である——**Phase A/B のログでこれらを判定語に使うと確実に偽陰性になる**。

#### 引用の再測定

design.md は `frame.rs:1157-1228`・`follow.rs:1132-1160` を挙げていた。タスク 2.2／3.1／3.2 の着地で全て移動しており、現ツリーの実値は上記のとおり。

### 1.4 S3′: バルーン矩形の可視性がどの経路でも検査されない

**未充足 AC: 3.4**（混在 DPI のモニタ間をまたぐ移動の前後で、キャラ窓とバルーン窓の**どちらも**不可視状態に遷移させない）

Req 2.8 は「少なくとも S1〜S3」を登記対象と定めており、S3′ はその枠外の**追加登記**である（design.md「既存アーキテクチャ分析」の表と「成果物 > diagnosis-report.md」が S3′ と 3.4 の対応を定める＝要件改稿を要さない）。

#### 欠陥の構造

バルーン窓の位置を書く経路は 3 本あり、**いずれも可視性を検査しない**:

| 経路 | 所在（`file:line`） | 構造名 | 位置の決め方 |
| --- | --- | --- | --- |
| キャラ確定後の随伴 | `crates/areka/src/placement/follow.rs:505-525`（書込は `:517-524`） | `follow_balloon` | `pos + BalloonFollow.offset` の**恒等式のみ**。矩形 × work area の交差は見ない |
| `\![move]` の随伴 | `crates/areka/src/placement/follow.rs:765-772` | `move_window_to` のバルーン分岐 | 同上（`x + offset.x, y + offset.y`） |
| 位置据置きリサイズ | `crates/areka/src/placement/follow.rs:1573-1580` | `resize_window_keep_position`（`:1518`） | 現在位置をそのまま維持し寸だけ差し替える＝位置の妥当性を評価する段が無い |

`follow_balloon` はキャラ窓確定位置の下流に居る（`resize_window_to` の `:964-971` が呼ぶ）。ゆえに **キャラ窓が画面端で（6.1 の是正後は clamp されて）留まった合成でも、offset ぶん外側にあるバルーンだけが完全不可視になり得る**——「キャラは見えているのに会話が読めない」という、Req 3.4 が名指しで防ごうとしている状態である。

#### 是正機構は存在するが未配線である（S3 と同一の純関数）

`guard_visibility`（`follow.rs:1417`）はバルーン矩形にもそのまま適用できる設計であり、**バルーン矩形ケースの檻も既に書かれている**（`follow.rs:2258` 以降の「`guard_visibility`: バルーン矩形（S3′・Req 3.4）」ブロック）。しかし本番配線は無い——§1.3 と同じく `crates/` 全 grep で一致がすべて定義行・doc・`#[cfg(test)]` 内（`follow.rs` のテストモジュールは `:1587` 以降）であることが根拠であり、`#[allow(dead_code)]` の有無は根拠にしない。配線はタスク 6.2 が担う。

#### 縮退シームの明示（先送りの記録）

画面端での左右反転など**バルーン配置の美観政策は本 spec の対象外**（M2 SSP 互換へ先送り）。本 spec が持つのは「完全不可視への遷移を防ぐ安全網」までであり、clamp によりバルーンがキャラと部分的に重なり得ることを許容する（*見えない会話*より*重なった会話*を優先する裁定・design.md「バルーン適用（S3′ 是正）」）。**`ClampX` 発火時の `warn!` がこの先送りの縮退シームである**。

#### 引用の再測定

design.md は `follow.rs:880-907`（`follow_balloon` 経路）を挙げていた。タスク 1.4 の route 配管で移動しており、現ツリーの実値は `follow.rs:505-525` である。

---

## 2. 実機採取記録（Phase B・**タスク 4.5 が記入する**）

> **本節は空である。** 埋めるのはタスク 4.5（開発者が実機で実行するゲートタスク）であり、**推測や代替で埋めてはならない**。採取は `diagnosis-procedure.md` の手順に厳密に従い、**Phase A 完了・S1/S2 是正未投入のビルド**で行う（是正投入後は消失の実機再現自体が起きなくなり、Q1〜Q4 の確定材料が永久に失われる）。
>
> 本節の結果がどうであれ、**§1 の 4 件の確定と是正は取り消されない**（§0.2）。

### 2.1 採取メタ

| 項目 | 値 |
| --- | --- |
| 採取ビルドのコミット SHA | _（4.5 が記入）_ |
| ビルドプロファイル | _（4.5）_ |
| モニタ構成（台数・拡大率・座標） | _（4.5・`[diag.monitor]` の全行を引用）_ |
| OS ビルド／採取日時 | _（4.5）_ |
| 生ログの保存パス（リポジトリ外） | _（4.5）_ |

### 2.2 セッション充足（Req 1.9）

| セッション | `SESSION-QUOTA` | scope 別 `low2high` / `high2low` / `total` | 備考 |
| --- | --- | --- | --- |
| ①ドラッグのみ | _（4.5）_ | _（4.5）_ | — |
| ②ドラッグ禁止・OS 設定変更のみ | _（4.5）_ | _（4.5）_ | `SESSION2-NO-DRAG` の判定も併記 |

### 2.3 Q1〜Q4 の回答（Req 2.1〜2.5・実機ログの該当行を引用）

| 問い | 対応 AC | 回答 | 引用行 |
| --- | --- | --- | --- |
| **Q1** 窓追従は暴走か操作どおりか | 2.3 | _（4.5）_ | _（マウス移動量と窓移動量の数値対応）_ |
| **Q2** 消失時の所在は真の不可視か可視領域内の見落としか | 2.2 | _（4.5）_ | _（矩形 × 全 work area の突合結論）_ |
| **Q3** 消失はドラッグ以外の経路か（最終位置を書いた主体） | 2.4 | _（4.5）_ | _（`route=` 語 or drag or os-suggested で名指し）_ |
| **Q4** バルーン消失はキャラ随伴か独立か | 2.5 | _（4.5）_ | _（相対位置の保存の実測）_ |

消失痕跡の判定語（両セッション分）: _（4.5・`VANISH-TRACE: NONE` または `FOUND …`）_
終了時静穏（Req 6.2/6.3）: _（4.5・`TEARDOWN-SILENCE`）_

### 2.4 「再現しない」と結論する場合の適用範囲（Req 2.6）

2 セッションがいずれも `SESSION-QUOTA: PASS` かつ `VANISH-TRACE: NONE` であった場合にのみ「再現しない」と結論できる。そのとき記録すること:

- 再現しなくなった機構的理由
- **除外するのは「実機でしか確定できない残余仮説に対する追加修正」のみ**であることの明記（§0.2）
- 受理回数が下限に満たないセッションは「再現しない」の根拠に用いていないことの確認

### 2.5 S1〜S3′ の実機痕跡の有無（Req 2.9）

> **痕跡が観測されなくても §1 の確定は取り消さない**（Req 2.9 の明文）。本表は因果の確認であって確定の関門ではない。

| ID | 実機ログ上の痕跡 | 根拠行 |
| --- | --- | --- |
| S1 | _（4.5・`diagnosis-procedure.md` §7.1 の残余 A 手順で「提案位置が実際に窓へ書かれた」ことを確認する）_ | _（4.5）_ |
| S2 | _（4.5）_ | _（4.5）_ |
| S3 | _（4.5・Phase A では `NearestFallback` warn が未実装ゆえ幾何突合で判定する）_ | _（4.5）_ |
| S3′ | _（4.5・同上）_ | _（4.5）_ |

### 2.6 決定論化できない残余のサインオフ（Req 5.5）

| 残余 | 判定語 | 実値 |
| --- | --- | --- |
| A: OS が実際に提示する提案矩形 | _（4.5・`RESIDUE-A-SUGGESTED-RECT`）_ | _（4.5・提案矩形の実値 1 例）_ |
| B: 実モニタ列挙 | _（4.5・`RESIDUE-B-MONITOR-ENUM`）_ | _（4.5）_ |

---

## 3. 赤→緑の実行記録（Req 5.4）

> 赤は**是正未投入のコミット**に対して採取する。緑は**是正コミット直後**に採取する。両者はコミット SHA で区別できる形で残す。
> 「96 の自己整合が欠陥を隠す」性質（Req 5.1／5.4）は、**是正前に dpi=96 の水準で通過し 120／192 で失敗する**という非対称としてここに現れる——記録から読み取れるようにすること。

### 3.1 S1 の実行記録（**S1 専用節**）

> **記入担当**: 赤＝タスク **4.3**（wintf 表示基盤のディスパッチ檻）／緑＝タスク **7.1**。
> **本節は 3.2 節（S2）と重ならない。** S2 の記録をここへ書かないこと（tasks.md 4.3／4.4 の明示制約）。

| 項目 | 内容 |
| --- | --- |
| 檻の所在 | `crates/wintf/src/ecs/window_proc/window_pos.rs:550-845`（`mod tests` 内の「S1 の赤証跡＝表示基盤ディスパッチ檻」ブロック）。赤 4 件＝`s1_red_external_authority_preserves_anchor_at_dpi96`（`:739`）／`_dpi120`（`:746`）／`_dpi192`（`:753`）／`s1_red_external_authority_establishes_no_write_context`（`:766`）。常時走る随伴 2 件＝`s1_control_default_policy_windows_apply_suggested_origin`（`:798`・非退行）／`s1_write_context_and_position_write_are_branched_together`（`:828`・D3 の分割禁止） |
| 赤の採取コミット | **`77411c0`**（タスク 4.2 着地時点＝Phase A 完了・S1 是正未投入）に本檻のみを載せたツリー。`WM_DPICHANGED`（同ファイル `:285`）は無改変で、差分は `#[cfg(test)] mod tests` 内に閉じている |
| 赤の再現コマンド | `cargo test -p wintf -- --ignored s1_red_`（赤 4 件は `#[ignore]` ゲート下。理由は下記「ゲート機構」） |
| 赤の実行出力 | 下記コードブロック（実行実測・`--test-threads=1`） |
| dpi 水準ごとの挙動 | **96 は通過・120／192 は失敗**（下記出力の 4 行が水準ごとに分かれている理由）。96 では `suggested_rect_for` が組む提案原点が現位置 `(1200,400)` と一致するため、提案位置を書いても書かなくても最終位置が変わらず政策分岐が観測できない。120 では提案原点が `(1500,500)`・192 では `(2400,800)` へ離れ、無条件書込が接地点を破壊する |
| 判定の表現 | 絶対 px の固定値ではなく **DPI 水準に対する比**（`suggested_rect_for` が `dpi/96` で提案原点を組む）と、**「`ExternalAuthority` 窓の最終位置＝現接地点」の不変条件**で表現している（Req 5.6）。探針の自己検査として「96 では提案＝現位置」「96 以外では提案 X ≠ 現位置」を檻自身が `assert` する（不動点に落ちた空虚な緑を防ぐ・記憶〈2.2 の教訓〉） |
| 緑の採取コミット | _（7.1）_ |
| 緑の実行出力 | _（7.1・全水準）_ |

#### 赤の実行出力（`cargo test -p wintf -- --ignored s1_red_ --test-threads=1`）

```text
running 4 tests
test ecs::window_proc::window_pos::tests::s1_red_external_authority_establishes_no_write_context ... FAILED
test ecs::window_proc::window_pos::tests::s1_red_external_authority_preserves_anchor_at_dpi120 ... FAILED
test ecs::window_proc::window_pos::tests::s1_red_external_authority_preserves_anchor_at_dpi192 ... FAILED
test ecs::window_proc::window_pos::tests::s1_red_external_authority_preserves_anchor_at_dpi96 ... ok

panicked at crates\wintf\src\ecs\window_proc\window_pos.rs:727:9:
assertion `left == right` failed: dpi=120: ExternalAuthority 窓の最終位置は現接地点のままであるべき（OS 提案位置が無条件に採用されている＝S1・Req 4.3/4.2）: DpiChangedOutcome { dpi_x_after: 120, context_established: true, written_origin: Some((1500, 500)) }
  left: (1500, 500)
 right: (1200, 400)

panicked at crates\wintf\src\ecs\window_proc\window_pos.rs:727:9:
assertion `left == right` failed: dpi=192: ExternalAuthority 窓の最終位置は現接地点のままであるべき（OS 提案位置が無条件に採用されている＝S1・Req 4.3/4.2）: DpiChangedOutcome { dpi_x_after: 192, context_established: true, written_origin: Some((2400, 800)) }
  left: (2400, 800)
 right: (1200, 400)

panicked at crates\wintf\src\ecs\window_proc\window_pos.rs:779:13:
dpi=120: ExternalAuthority 窓で DpiChangeContext が確立されている（残置コンテキストが後続 WM_WINDOWPOSCHANGED を DPI echo と誤認させる・D3）: DpiChangedOutcome { dpi_x_after: 120, context_established: true, written_origin: Some((1500, 500)) }

test result: FAILED. 1 passed; 3 failed; 0 ignored; 0 measured; 537 filtered out
```

#### この赤が §1.1 の①①' を名指しで撃っていること

`DpiChangedOutcome` の 3 フィールドは §1.1 の連鎖の①①'をそのまま外形化したものである:

- `dpi_x_after: 120` — DPI component は更新されている（§1.1 の是正は DPI 受理を止めるものではない）
- `context_established: true` — §1.1 ①'（`window_pos.rs:343-346` の `DpiChangeContext::set` が無条件）
- `written_origin: Some((1500, 500))` — §1.1 ①（`window_pos.rs:369-379` の `guarded_set_window_pos` が無条件・提案原点そのもの）。値は `guarded_set_window_pos` の実施ログ（`wintf::ecs::window::command`）を `capture_under_filter` で実濾過して復元しており、宣言ではなく**実際に書かれた座標**である

すなわち赤は「`ExternalAuthority` を宣言しても分岐が存在しない」ことによる失敗であって、檻の組み違いによる失敗ではない。`dpi_suggested_position_decision`（`window_proc/dpi_helpers.rs:32`）は本番から呼ばれていない（§1.1「是正機構は存在するが未配線である」）。

#### ゲート機構（`#[ignore]`）と 7.1 への申し送り

赤 4 件は `#[ignore = "S1 赤証跡（是正未投入では失敗する・タスク 4.3）。再現: cargo test -p wintf -- --ignored s1_red_"]` で通常実行から外してある。常時失敗する檻を置くと `cargo test` が門として無価値になり、以後の全タスクの検証を潰すためである（先例: `crates/areka-emo-atlas/src/emo2_golden.rs:228`）。`cargo test -p wintf` は本タスク着地後も**緑**（1032 → 1034・常時走る随伴 2 件の増分。赤 4 件は ignored 計上）。

**タスク 5.1／7.1 は是正配線と同時に `#[ignore]` を 4 件とも外し、常時走る回帰檻へ昇格させること**（Req 5.1 の常時テスト化）。dpi96 の 1 件も外す——「96 では緑」は是正後も成立する性質であり、外して初めて非対称の記録が回帰檻として保存される。

#### 緑側の先行確認（本タスクで実施・是正は投入していない）

檻が是正後に緑へ反転することを、5.1 相当の分岐（entity から `DpiSuggestedRectPolicy` を読み、`dpi_suggested_position_decision` の `None` で `DpiChangeContext::set` と `guarded_set_window_pos` を**まとめて**飛ばす）を**一時的に当てて実測し、直後に完全に戻した**。結果は赤 4 件すべて `ok`・随伴 2 件も `ok`。**是正はツリーに残していない**（4.5 の実機採取が是正未投入ビルドを要求するため・§2 の冒頭注記）。これにより本檻は「今赤・是正後緑」の両側が実行で確かめられている（空虚な赤ではない）。

### 3.2 S2 の実行記録（**S2 専用節**）

> **記入担当**: 赤＝タスク **4.4**（DPI 相の位置再射影檻）／緑＝タスク **7.1**。
> **本節は 3.1 節（S1）と重ならない。**

| 項目 | 内容 |
| --- | --- |
| 檻の所在 | _（4.4 が記入）_ |
| 赤の採取コミット | _（4.4）_ |
| 赤の実行出力 | _（4.4・「再導出結果が得られない経路で位置が再射影されない」ことで失敗する出力）_ |
| dpi 水準ごとの挙動 | _（4.4・96 は旧 Y と新 Y が自己整合して通過／120・192 で失敗）_ |
| 判定の表現 | _（4.4・絶対 px ではなく接地点＝下端中央の不変条件として）_ |
| 緑の採取コミット | _（7.1）_ |
| 緑の実行出力 | _（7.1・全水準）_ |

---

## 4. 是正後の実機再サインオフ（**タスク 7.4 が記入する**）

> 是正投入後のビルドで `diagnosis-procedure.md` と**同一手順**の 2 セッションを再実行する。

| 項目 | 内容 |
| --- | --- |
| 再サインオフのコミット SHA | _（7.4）_ |
| セッション①／②の `SESSION-QUOTA` | _（7.4・受理回数の下限踏破）_ |
| 消失痕跡 | _（7.4・`VANISH-TRACE: NONE` であること）_ |
| 接地点保存の実測 | _（7.4）_ |
| 残余 A（提案矩形） | _（7.4・**判定が反転する**——ゴースト窓では `applied=false` を報告し、提案座標が窓へ書かれないことが PASS 条件）_ |
| 残余 B（モニタ列挙） | _（7.4）_ |
| S1〜S3′ の是正後の実機挙動 | _（7.4・§1 の 4 件それぞれについて）_ |

---

## 5. 記入担当と本書のメンテ規約

| 節 | 記入するタスク | 内容 |
| --- | --- | --- |
| §0・§1 | **4.2（完了）** | 規約と静的構造証跡 4 件の先行登記 |
| §3.1 | 4.3（赤）→ 7.1（緑） | S1 専用の実行記録 |
| §3.2 | 4.4（赤）→ 7.1（緑） | S2 専用の実行記録 |
| §2 | 4.5 | 実機 2 セッションの採取結果・Q1〜Q4・S1〜S3′ の痕跡 |
| §4 | 7.4 | 是正後の実機再サインオフ |

**メンテ規約**:

1. §1 の引用先を編集したコミットは、**同じコミットで §1 の `file:line` も更新する**。行番号だけの更新でも構わないが、構造名が変わったときは構造名も直す。
2. 既存の節を**上書きしない**。後続タスクは自分の担当節にのみ追記する（§3.1 と §3.2 は互いに独立）。
3. 手順・判定語・grep 規則は本書に書かない（`diagnosis-procedure.md` が正本）。本書に載せるのは**結論と証跡**だけである。
4. 実機ログは該当行の引用のみを転記し、生ログの保存パスと採取コミット SHA を併記する（リポジトリ配下へ生ログを置かない）。
