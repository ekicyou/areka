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

> **本節はセッション①のみ記入済み（2026-07-31）。セッション②は未採取。** 埋めるのはタスク 4.5（開発者が実機で実行するゲートタスク）であり、**推測や代替で埋めてはならない**。採取は `diagnosis-procedure.md` の手順に厳密に従い、**Phase A 完了・S1/S2 是正未投入のビルド**で行う（是正投入後は消失の実機再現自体が起きなくなり、Q1〜Q4 の確定材料が永久に失われる）。
>
> **タスク 4.5 は未完である**——2 セッション採取が完了条件であり、①だけでは Req 2.4（ドラッグ**以外**の経路で消失した場合の主体名指し）を単独観測できない。②未採取のまま 4.5 を `[x]` にしてはならない。
>
> 本節の結果がどうであれ、**§1 の 4 件の確定と是正は取り消されない**（§0.2）。

### 2.1 採取メタ

| 項目 | 値 |
| --- | --- |
| 採取ビルドのコミット SHA | `0db483e`（Phase A 完了・S1/S2/S3/S3′ 是正未投入） |
| ビルドプロファイル | **release**（手順書 §1.2 は dev を指定するが、dev では描画スレッドが 1 コアの 45.3% を消費し手動ドラッグが困難だったため変更。release 実測 13.4%。§0.1 の絶対条件「是正未投入のビルド」は満たしており、`tracing` は release でも落ちない） |
| モニタ構成 | 2 台・混在 DPI・負座標あり（下表） |
| OS ビルド／採取日時 | Microsoft Windows 11 Pro build 26200／2026-07-31T16:35:44+09:00 |
| 生ログの保存パス（リポジトリ外） | `%LOCALAPPDATA%\areka-diag\20260731-163422-rel\session1-drag.log`（6,953,820 B・31,460 行） |
| 有界自動終了 | `AREKA_APP_SMOKE_EXIT_MS=1800000`（30 分）・実際の走行 07:34:22Z〜08:04:22Z |

モニタ構成（`[diag.monitor]` 全行の引用）:

```
[diag.monitor] index=0 handle=1574999 bounds=0,0,2880,1800     work_area=0,0,2880,1704     dpi=192 primary=true
[diag.monitor] index=1 handle=264283  bounds=-2560,195,0,1795  work_area=-2560,195,0,1795  dpi=144 primary=false
```

手順書 §1.3 の観測条件（混在 DPI・負座標・非対称 work area）をすべて充足している。monitor0 は下端 96px がタスクバー（1800→1704）、monitor1 は全面が work area（1795）で、**両者の下端が 91px 異なる**——この 91 が以下 §2.3／§2.5 の判定に繰り返し現れる定数である。

### 2.2 セッション充足（Req 1.9）

| セッション | `SESSION-QUOTA` | scope 別 `low2high` / `high2low` / `total` | 備考 |
| --- | --- | --- | --- |
| ①ドラッグのみ | **PASS** | scope0: 12 / 12 / 24　scope1: 10 / 10 / 20（キャラ窓のみ・下限は各 3） | 下限 12 に対し実績 **44**。手順書 §5 の 2 段 grep をそのまま適用 |
| ②ドラッグ禁止・OS 設定変更のみ | **FAIL (0 / 6)** | scope0: 0 / 0 / 0（本走行は scope 0 の 1 体のみ＝下限は 6） | `SESSION2-NO-DRAG: **PASS**`（`[start_preparing]` 0 件・`[DragEvent]` 0 件）。**採取の失敗ではなく Req 2.4 への回答**——拡大率を 7 回変更しても `WM_DPICHANGED` が 1 度も届かない。**§2.7（S4）が原因を確定** |

2 段 grep の実行結果（第 1 段＝`[diag.window_move]` からの `scope`→`entity` 対応表、第 2 段＝当該 `entity=` の受理行計数）:

```
scope->char   : {'0': ['4v0'], '1': ['6v0']}
scope->balloon: {'0': ['3v0'], '1': ['5v0']}

entity=4v0 kind=char    scope=0  low2high=12 high2low=12 total=24
entity=6v0 kind=char    scope=1  low2high=10 high2low=10 total=20
entity=3v0 kind=balloon scope=0  low2high=11 high2low=11 total=22
entity=5v0 kind=balloon scope=1  low2high=9  high2low=9  total=18
                                            全 entity 合計 = 84
SESSION-QUOTA: PASS  (44 / 12)
```

方向は同行の `old_dpi_x` と `new_dpi_x` の比較で判定した（192↔144 の往復のみ）。**全 entity で low2high と high2low が厳密に等しい**——ドラッグでモニタ間を往復した操作と整合し、受理の取りこぼしが無いことの傍証になる。

`[diag.window_move]` の route 内訳（総 4,296 件）:

| route | 件数 | 読み |
| --- | --- | --- |
| `BalloonFollow` | 4,211 | ドラッグ中の毎フレーム随伴（大半） |
| `DpiReproject` | 44 | **キャラ窓の DPI 受理 44 回と厳密に一致**＝全受理で射影が走った |
| `KeepPositionResize` | 40 | 位置据置きリサイズ |
| `ReportedSizeReconcile` | 1 | 起動直後の k₀ 補正 1 回のみ |

**D13（route 語彙 9 種化）の実機的正当性がここで確認された**——`ReportedSizeReconcile` は起動時 1 回だけ立ち、`DpiReproject` は DPI 受理と 1:1 である。1.4 是正前の実装（両者に `DpiReproject` を貼る）であれば、DPI 変化ゼロの起動直後に偽の `DpiReproject` が 1 件混入し、この 44:44 の一致が崩れて突合が成立しなかった。

### 2.3 Q1〜Q4 の回答（Req 2.1〜2.5・実機ログの該当行を引用）

| 問い | 対応 AC | 回答（セッション①） | 根拠 |
| --- | --- | --- | --- |
| **Q1** 窓追従は暴走か操作どおりか | 2.3 | **操作どおり（暴走なし）** | 窓移動量／マウス移動量の比が中央値 1.000・p05=p95=1.000。\|比\|>3 は **0 / 3,488 組** |
| **Q2** 消失時の所在は真の不可視か可視領域内の見落としか | 2.2 | **消失そのものが発生せず**（判定不能ではなく、判定対象がゼロ） | `VANISH-TRACE: NONE`（4,098 件全数突合・全 work area 非交差はゼロ） |
| **Q3** 消失はドラッグ以外の経路か（最終位置を書いた主体） | 2.4 | **該当なし**（消失ゼロゆえ）。ただし**書込主体が 2 者いる事実は確認**——下記「二重ライターの実機確認」 | `applied=true` 84/84 → 直後に areka `DpiReproject` が上書き |
| **Q4** バルーン消失はキャラ随伴か独立か | 2.5 | **該当なし**（消失ゼロゆえ） | — |

消失痕跡の判定語（セッション①）: **`VANISH-TRACE: NONE`**（②は未採取）
終了時静穏（Req 6.2/6.3）: **`TEARDOWN-SILENCE: FAIL`**（下記 §2.3.4）

#### 2.3.1 Q1 の実測（Req 2.3・暴走の否定）

`wintf::ecs::drag` の `[DragEvent] Dispatching`（`trace` 水準・手順書 §3.1 O10）を 4,156 件採取し、各イベント直後の `[guarded_set_window_pos]` と対応付けて、マウスの水平移動量に対する窓の水平移動量の比を求めた。

```
DragEvent 件数: 4156   （entity=4v0: 2243 / entity=6v0: 1913）
マウス→窓 の対応が取れた組: 4156
窓移動量/マウス移動量 の比: n=3488
  median=1.000  p05=1.000  p95=1.000  min=-1.455  max=1.065
  |比|>3（暴走候補）の件数: 0 / 3488
```

**分布が 1.000 に集中し外れ値が存在しない**——ドラッグ経路そのものは 1:1 で追従しており、消失の原因ではない。`min=-1.455` はモニタ跨ぎ直後の 1 フレームに現れる符号反転で、DPI 受理と同時刻に集中する（Q3 の二重ライターの帰結であり、ドラッグ経路の暴走ではない）。

#### 2.3.2 Q2 の実測（Req 2.2・幾何突合）

`[diag.window_move]` の全レコードに対し、寸法を持たないレコード（`w=-`／`h=-`＝`SWP_NOSIZE` 経路）は同 entity の直前の実寸を引き継ぐ規則で矩形を復元し、§2.1 の 2 つの work area と交差判定した。

```
判定した窓移動レコード: 4,098 件
全 work area と非交差だったもの: 0 件
VANISH-TRACE: NONE
```

**セッション①では「真の不可視」も「可視領域内の見落とし」も発生していない。** ただし採取中、開発者は目視で**バルーンが見えなくなる**現象を報告している。上記のとおり幾何的には常に可視領域内にあったため、これは**位置の問題ではなく Z オーダーの問題**である（シェルが前面へ上がってもバルーンの Z オーダーが追随せず他アプリ窓の背後へ潜る）。本 spec の担当外として `.kiro/specs/areka-P0-ghost-window-zorder/` へ切り出し済み。**Req 2.2 の「可視領域内の見落とし」の実例が Z オーダー起因で存在した**という事実を、本 spec の結論の一部として記録する。

#### 2.3.3 二重ライターの実機確認（Q3 の副産物・S1 の直接証跡）

消失は起きなかったが、**位置を書く主体が 1 回の DPI 変化につき 2 者いる**ことが実機で確定した。

```
[WM_DPICHANGED] suggested position write decision : 84 件
  applied= の内訳 : {'true': 84}      ← false は 1 件も無い
```

wintf 側が OS 提案位置を **84/84 で無条件に窓へ書き**、その直後に areka 側の `DpiReproject` が自身の確定値で上書きしている。両者の差は小さくない:

```
OS提案=(-430, 701)  → 直後 areka DpiReproject 書込=(-361, 974)   dx=+69   dy=+273
OS提案=(-468, 701)  → 直後 areka DpiReproject 書込=(-829, 974)   dx=-361  dy=+273
OS提案=( 59,1104)  → 直後 areka DpiReproject 書込=(-368,1195)   dx=-427  dy=+91
```

**Y 差は常に +273 または ±91 の定数**（＝ DPI 比と work area 下端差から決まる射影量）である一方、**X 差は −861〜+861 の範囲でばらつく**。この非対称は §1.1（S1＝X を再計算せず OS 提示値を素通し）と §1.2（S2＝Y の再射影がゲートされる）の**出力上の指紋**そのもので、静的構造証跡の 2 件が実機で別々の症状として現れることを裏づける。

セッション①で消失に至らなかったのは、**キャラ窓の DPI 受理 44 回すべてで `DpiReproject` の上書きが後続したため**（§2.2 の route 内訳が 44:44 で一致）。この上書きが走らない経路——`reconcile_window_size` が再導出結果を返さない経路（§1.2）——が実在すれば、OS 提案位置がそのまま残る。**セッション②（ドラッグ禁止・OS 設定変更のみ）はまさにその経路を単独で叩く採取であり、①の結果では代替できない。**

#### 2.3.4 `TEARDOWN-SILENCE: FAIL`（Req 6.2/6.3・タスク 3.2 の穴）

終了処理で `WARN` が 3 件残った。

```
DEBUG wintf::ecs::window::window_handle: Entity being removed, sending WM_CLOSE entity=5v0 …
DEBUG …                                                                        entity=3v0 …
DEBUG …                                                                        entity=6v0 …
DEBUG …                                                                        entity=4v0 …
INFO  wintf::ecs::app: [App] Last window closed.
INFO  areka: smoke 自動 close: 起動窓（ダミー窓／ゴースト窓）を despawn しました count=4
WARN  bevy_ecs::world: Could not despawn entity: … ID 5v0 is invalid; its index now has generation 1.
WARN  bevy_ecs::world: Could not despawn entity: … ID 3v0 is invalid; …
WARN  bevy_ecs::world: Could not despawn entity: … ID 6v0 is invalid; …
```

**原因は特定済み**: `despawn_smoke_targets`（`crates/areka/src/main.rs:795-810`）が query で 4 体を集めてからループで `world.despawn(e)` を呼ぶが、**1 体目の despawn が連鎖して残り 3 体も破棄する**ため、ループの後半が既に無効な entity を叩いている。存在確認が無い。

これは**タスク 3.2 と同一の欠陥クラス**（「entity 不在＝破棄済みは正常終了系」）だが、3.2 は**消費側 4 入口**（`follow.rs` の `resize_window_to`／`resize_window_keep_position`・`frame.rs` の `resnap_with`／`reconcile_reported_sizes`）にガードを敷いたのに対し、**despawn の呼出点そのもの**は範囲外だった。Req 6.2 の完了状態「破棄済み窓に対する警告以上のログが 1 行も出ない」は現状**未達**である。→ タスク 7.3 の担当範囲として申し送る（§5 参照）。

#### 2.3.5 参考: 本 spec 範囲外だが実機で確認された事象

| 事象 | 件数 | 扱い |
| --- | --- | --- |
| `[WM_WINDOWPOSCHANGED] DPI center correction skipped: BoxStyle not found`（`wintf::ecs::window_proc::dpi_helpers`） | **84**（＝DPI 受理と同数） | **全 DPI 変化で center correction が不発**している。位置権威の是正（5.1）で当該経路ごと不要になる可能性が高いため、5.1 実装時に到達性を再確認すること |
| `kanade: Steady 待ち点で想定外の SHIORI 応答` | 79 | 本 spec 範囲外（SHIORI 応答系） |
| `atlas 未束縛 element: resolve 失敗 … purple/a/null.png` | 3 | 既知の α 無し `null.png`・表示無害 |

### 2.4 「再現しない」と結論する場合の適用範囲（Req 2.6）

2 セッションがいずれも `SESSION-QUOTA: PASS` かつ `VANISH-TRACE: NONE` であった場合にのみ「再現しない」と結論できる。そのとき記録すること:

- 再現しなくなった機構的理由
- **除外するのは「実機でしか確定できない残余仮説に対する追加修正」のみ**であることの明記（§0.2）
- 受理回数が下限に満たないセッションは「再現しない」の根拠に用いていないことの確認

**本条は適用できない（適用の前提が崩れた）。** ②の `SESSION-QUOTA` は `FAIL (0/6)` であり、**受理回数の下限に満たないセッションを「再現しない」の根拠に用いてはならない**（Req 1.9・本条 3 番目の確認項目）。

さらに②が下限に達しなかった理由自体が欠陥である（§2.7・S4＝OS 経由の DPI 変化が全経路で無視される）。**「DPI 変化が届かないので消失も観測されない」を「再現しない」と読んではならない。** これは観測装置が壊れているのであって、被検体が健全なのではない。

①についても、`SESSION-QUOTA: PASS` かつ `VANISH-TRACE: NONE` だが、§2.3.3 で示したとおり**消失に至らなかった機構的理由は「`DpiReproject` の上書きが 44/44 で後続したこと」**である。その上書きが走らない経路（§1.2 の S2 ゲート）を叩くには②が必要で、①の結果は②の代替にならない。

**結論: 本 spec は「再現しない」と結論しない。** S1〜S3′ の是正（5.1／5.2／6.1／6.2）と檻（7.1／7.2）は全て実施する。加えて S4 の是正タスクを新設する（§5）。S4 是正後は**②を採り直して初めて**「ドラッグ以外の経路」の観測が成立する。

なお②が `VANISH-TRACE: NONE` であっても、**§1 の S1〜S3′ の確定と是正 5.1／5.2／6.1／6.2、檻 7.1／7.2 は除外されない**（§0.2・Req 2.6／2.9／5.1／5.4）。とりわけ S1 は §2.5 のとおり**実機で 84/84 の陽性痕跡**を残しており、「再現しない」の対象ですらない。

### 2.5 S1〜S3′ の実機痕跡の有無（Req 2.9）

> **痕跡が観測されなくても §1 の確定は取り消さない**（Req 2.9 の明文）。本表は因果の確認であって確定の関門ではない。

| ID | 実機ログ上の痕跡（セッション①） | 根拠行 |
| --- | --- | --- |
| S1 | **陽性・決定的**。OS 提案位置が **84 回中 84 回**そのまま窓へ書かれた（`applied=false` はゼロ）。書かれた X は areka の権威 X と最大 861px 乖離する | `[WM_DPICHANGED] suggested position write decision entity=4v0 hwnd=HWND(0xbe0bb4) applied=true suggested_left=-430 suggested_top=701`（line 3902 ほか計 84 行） |
| S2 | **陽性・1 件**。キャラ窓の全書込 45 件のうち **1 件で接地点規約が破れた**（下記 §2.5.1） | `[diag.window_move] route=DpiReproject entity=4v0 kind=char scope=0 x=-287 y=883 w=573 h=821 dpi=144`（line 7968） |
| S3 | **陰性**（4,098 件すべて work area と交差・`VANISH-TRACE: NONE`）。§1.3 の確定は取り消さない（Req 2.9） | — |
| S3′ | **幾何的には陰性**。ただし**目視ではバルーンが消失**した——原因は Z オーダーであり位置ではない（§2.3.2） | — |

#### 2.5.1 S2 の実機痕跡（接地点規約の破れ・全数検査）

キャラ窓の寸法つき書込 45 件すべてについて、接地点 `ground = (x + w/2, y + h)` が**当該窓が乗るモニタの `work_area.bottom` と一致するか**を検査した（§0 の接地点規約＝下端中央）。

```
規約に合致 : 44 件
規約が破れ :  1 件

line7968  route=DpiReproject entity=4v0 scope=0 dpi=144
          rect=(-287, 883, 573x821)  ground=(-1, 1704)
          → 接地先は monitor1（x=-1 は monitor1 の領域）・その work_area.bottom=1795
          → ずれ = -91px（キャラが接地面から 91px 浮いている）
```

`ground_y` の全分布は以下のとおりで、**破れは分布上の単独の外れ値として現れる**:

| `ground_y` | `dpi` | 件数 | 判定 |
| --- | --- | --- | --- |
| 1704 | 192 | 23 | 正常（monitor0 の下端） |
| 1795 | 144 | 21 | 正常（monitor1 の下端） |
| **1704** | **144** | **1** | **破れ**——dpi=144 の窓が monitor1 上にありながら monitor0 の下端に接地している |

ずれ量 −91px は §2.1 で述べた**2 モニタの work area 下端差そのもの**であり、「変化後の DPI に対応する work area で射影し直されなかった」ことを意味する。これは §1.2 の S2（位置の再射影が窓寸の再導出結果に条件付けられ、得られない経路で欠落する）が**実機で 1 回顕在化した**ものである。

**1 件でも十分な証跡である**理由: この破れが起きるには「DPI は変化したが再導出結果が得られない」という条件が必要で、①のようにドラッグで能動的にモニタを跨ぐ操作では寸法報告がほぼ毎回間に合う。**セッション②（OS 設定側から拡大率を変える）はこの条件を系統的に発生させる**と予想され、②で件数が増えるか否かが S2 の実機的な重大度の指標になる。

なお破れた行の直前の決定行は `applied=true suggested_left=-383 suggested_top=701` で、書かれた `y=883` とは一致しない——**この破れは OS 提案の直接転写ではなく、areka 側の射影が古い work area で走った結果**である。S1（X の素通し）とは独立した欠陥であることが出力から区別できる。

### 2.6 決定論化できない残余のサインオフ（Req 5.5）

| 残余 | 判定語 | 実値 |
| --- | --- | --- |
| A: OS が実際に提示する提案矩形 | **`RESIDUE-A-SUGGESTED-RECT: PASS`**（84 件観測・すべて `applied=true`） | `entity=4v0 hwnd=HWND(0xbe0bb4) applied=true suggested_left=-430 suggested_top=701`（line 3902）／`entity=6v0 … suggested_left=59 suggested_top=1104`（line 15013・OS が正の X を提示した例） |
| B: 実モニタ列挙 | **`RESIDUE-B-MONITOR-ENUM: PASS`**（wintf 側・areka 側が完全一致） | 下記 |

残余 B の両側突合（D12 の「共有語彙 grep 突合」が実機で成立することの確認）:

```
wintf : [initialize_layout_root] Creating Monitor entity handle=1574999
        bounds_left=0 bounds_top=0 bounds_right=2880 bounds_bottom=1800
        work_area=0,0,2880,1704 dpi=192 is_primary=true
areka : [diag.monitor] index=0 handle=1574999
        bounds=0,0,2880,1800 work_area=0,0,2880,1704 dpi=192 primary=true

wintf : [initialize_layout_root] Creating Monitor entity handle=264283
        bounds_left=-2560 bounds_top=195 bounds_right=0 bounds_bottom=1795
        work_area=-2560,195,0,1795 dpi=144 is_primary=false
areka : [diag.monitor] index=1 handle=264283
        bounds=-2560,195,0,1795 work_area=-2560,195,0,1795 dpi=144 primary=false
```

`handle`・`bounds`・`work_area`・`dpi`・`primary` の 5 フィールドすべてが両側で一致した。**タスク 1.2／1.3 が揃えたフィールド名規約と、タスク 4.1 が追加した `wintf::ecs::layout::systems::monitor_systems=debug` の必要性が、ここで実機により裏づけられた**（この 1 語が無ければ wintf 側の行が暗転し、本突合は成立しなかった）。

### 2.7 **S4（新規・セッション②で発見）: OS 表示設定経由の DPI 変化が全経路で無視される**

> 本項はタスク 4.5 のセッション②が発見した**新規の欠陥**である。§1 の S1〜S3′ とは独立で、既存の是正タスク 5.1／5.2／6.1／6.2 のいずれも**これを直さない**。担当タスクの新設が必要（§5 参照）。

#### 採取条件

| 項目 | 値 |
| --- | --- |
| コミット | `0db483e`（`crates/` は HEAD `3498623` と差分ゼロ＝docs のみ）・release |
| 生ログ | `%LOCALAPPDATA%\areka-diag\20260731-session2\session2-osdpi.log` |
| モニタ | index0 `bounds=-2880,365,0,2165 dpi=192`／index1 `bounds=0,0,3840,2160 work_area=0,0,3840,2100 dpi=120 primary=true` |
| ゴースト所在 | index1（primary・125%）・scope 0 のみ（`entity=4v0` char／`3v0` balloon） |
| 操作 | **プライマリの拡大率を 125%↔200% で 7 回変更**（ドラッグ一切なし） |

#### 観測結果

```
[start_preparing]（ドラッグ痕跡）        :  0 件   → SESSION2-NO-DRAG: PASS
[DragEvent]                             :  0 件
[App] Display configuration changed     : 28 件（7 回 × 4）
[detect_display_change_system] …updating:  7 件   ← 検知はしている
[detect_display_change_system] Found monitors count=2 : 7 件
[detect_display_change_system] Updating Monitor entity:  0 件   ← ★ 一度も更新されない
WM_DPICHANGED（受信・O6）               :  0 件   ← ★ 一度も届かない
DPI component directly updated（O7）     :  0 件
[diag.window_move]（拡大率変更後）        :  0 件   ← ★ 窓は一切動かない・寸も変わらない
presenter の window_dpi（変更後）        : Some((120,120)) のまま（実モニタは 192）
```

**開発者による目視**: 拡大率を 200% にしてもゴーストの表示サイズが変わらない。DPI 追従が起きていれば大きくならなければならない。

`SESSION-QUOTA: FAIL (0 / 6)` である。**これは採取の失敗ではなく、Req 2.4 への回答そのもの**——「ドラッグ以外の経路」には**そもそも書き手が存在しない**。

#### 根本原因（静的構造証跡・file:line で確定）

`Monitor` の `PartialEq` が **`handle` のみ**で等価判定する:

```rust
// crates/wintf/src/ecs/window/monitor.rs:103-107
impl PartialEq for Monitor {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle          // bounds・work_area・dpi・is_primary を見ない
    }
}
```

一方その消費側は `!=` を**値の変化検出**に使っている:

```rust
// crates/wintf/src/ecs/layout/systems/monitor_systems.rs:229-236
if existing_monitor != new_monitor {                      // ← handle は不変ゆえ恒偽
    debug!(entity = ?entity, "[…] Updating Monitor entity");
    if let Ok((_, mut monitor)) = existing_monitors.get_mut(entity) {
        *monitor = new_monitor;                            // ← 到達しない
    }
}
```

**同一性の意味論（`handle` 一致）を、値の変化検出に使っている。** モニタの `handle` は拡大率を変えても不変なので、この分岐は**構造的に恒偽**である。結果、`Monitor` エンティティの `bounds`／`work_area`／`dpi` は**起動時の値のまま永久に凍結**する。

しかもこの誤りは**檻で固定されている**:

```rust
// crates/wintf/src/ecs/window/monitor.rs:254-264
#[test]
fn test_partial_eq_compares_handle_only() {
    // PartialEq は handle のみで等価判定する（bounds/dpi/is_primary は無視）。
    let a = make_monitor(42, 0, 0, 800, 600);
    let mut b = make_monitor(42, 100, 100, 1920, 1080);
    b.dpi = 144;
    assert_eq!(a, b, "handle が同一なら他フィールドが異なっても等価");
}
```

**檻は `PartialEq` の実装を正しく固定しているが、消費側がその意味論を誤用していることは誰も見ていない。** 「檻が緑でも欠陥は成立する」典型例であり、[[test-only-decision-branches-not-proven-wiring]]（配線は再テストしない）の裏面に当たる——**意味論の異なる 2 箇所を結ぶ配線こそ檻が要る**。

#### 帰結（なぜこれが「消失」の機構になるか）

OS 側で拡大率を変えると、areka の 2 つの位置権威が**同時に凍結**する:

| 権威 | 更新経路 | セッション②での実測 |
| --- | --- | --- |
| 窓の `DPI` component | `WM_DPICHANGED` ハンドラのみ | **0 件**＝旧 DPI のまま |
| `Monitor` の `work_area`／`dpi` | `detect_display_change_system` の更新分岐 | **恒偽**＝起動時のまま |

両方が凍結するため、**射影も遷移ガードも「正しい値で間違った答え」を出し続ける**。ユーザーが拡大率を大きく変えれば、旧 work area 基準の接地点は新しい可視領域の外へ出得るが、**それを検出して引き戻す経路が 1 本も無い**。S3（可視性の不変条件が無い）が最も危険な形で顕在化する条件である。

なお `WM_DPICHANGED` が 0 件である理由は本セッションでは未確定（プロセスの DPI awareness 問い合わせは `PER_MONITOR_DPI_AWARE`＝レガシー API では v1/v2 を区別できない）。`SetProcessDpiAwarenessContext` の戻り値が [`runtime/mod.rs:111`](../../../crates/wintf/src/runtime/mod.rs) で `let _ =` により**捨てられている**ため、設定失敗が起きていても観測できない。**担当タスクはまずこの戻り値をログ化すること**（Req 1.5 の「点灯しない観測点を根拠に使わない」の直接適用）。

#### 未充足にする受入基準

- **Req 2.4**（消失がドラッグ以外の経路で起きた場合の主体名指し）——書き手が不在という形で回答
- **Req 3.1／3.2**（可視性の不変条件）——凍結した work area では判定自体が無意味になる
- **Req 4.1／4.2**（DPI 変化時の寸と位置の再導出）——OS 経路では一度も走らない

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
| 檻の所在 | `crates/areka/src/emo2_boot/frame.rs:2911-3368`（`mod tests` 内の「task 4.4: S2 の赤証跡＝DPI 相の位置再射影檻」ブロック）。赤 4 件＝`s2_red_ground_point_preserved_at_dpi96`（`:3196`）／`_from_dpi96_to_dpi120`（`:3209`）／`_from_dpi96_to_dpi192`（`:3217`）／`_from_dpi120_to_dpi192`（`:3226`）。常時走る随伴 2 件＝`s2_control_some_report_path_reprojects_and_keeps_balloon_offset`（`:3239`・`Some` 経路の非退行対照）／`s2_dpi_phase_writes_nothing_when_the_ground_point_already_holds`（`:3308`・Req 4.5 の書込ゼロを守る前方ガード） |
| 檻の構成 | 偽 HWND のヘッドレス World（既存 `dpi_world()`＝2 scope×char/balloon・書込 witness 付き）×**合成マルチモニタ**（`s2_snapshot`＝ゴーストの居るモニタ＋負座標 `top=-140`／3200 超 `right=3874` の隣接モニタ）×**「再導出結果なし」固定の偽寸法報告源**（`FakeReports` の空マップ＝`refresh_scale_report` が常に `None`）×**DPI 注入**（`run_s2_probe(from,to)`・96→96／96→120／96→192／120→192）。実 GPU・実高 DPI モニタ不要の決定論（Req 5.2） |
| 赤の採取コミット | **`db8bd1a`**（タスク 4.3 着地時点＝Phase A 完了・S1／S2 是正いずれも未投入）に本檻のみを載せたツリー。差分は `frame.rs` の `#[cfg(test)] mod tests` 内に閉じた **459 行の追加のみ**（`git diff --stat` で 1 file / +459 / −0・単一ハンク `@@ -2908,6 +2908,465 @@ mod tests`）＝本番の `dpi_phase_with` は無改変。W5 同居契約どおり `run_text_scale_phase`・`balloon_models` 写像には触れていない |
| 赤の再現コマンド | `cargo test -p areka -- --ignored s2_red_`（赤 4 件は `#[ignore]` ゲート下。理由は下記「ゲート機構」） |
| 赤の実行出力 | 下記コードブロック（実行実測・`--test-threads=1`・`RUST_BACKTRACE=0`） |
| dpi 水準ごとの挙動 | **96 は通過・120／192 は失敗**。work area 下端はタスクバーの論理高（48）が `dpi/96` 倍で物理へ伸びる分だけ上がる＝`1492 − 48*dpi/96`（96→**1444**／120→**1432**／192→**1396**）。96 では変化の前後で下端が動かないため旧 Y と「新 work area 下端 − h」が自己整合し、再射影の欠落が観測されない（診断レポート §1.2「なぜ dpi=96 では隠れるか」）。120／192 では下端が 12px／48px 動くのに位置が一切書かれず、接地点 Y が旧下端 1444 に据え置かれる＝足元がタスクバーの下へ潜り込む |
| 判定の表現 | 絶対 px の固定値ではなく **接地点（下端中央）の不変条件**で表現している（Req 5.6）——⑴変化**前**の接地点 Y ＝そのときの work area 下端（探針の前提）⑵接地点の **X 成分**が変化の前後で保存される ⑶接地点の **Y 成分**が「今いるモニタの work area 下端」であり続ける。work area 自体も絶対値を直書きせず **DPI 水準の関数**（`s2_work_area_for_dpi`）として組み、判定側は `work_area_for_window_with_origin` で**窓の実位置から解決**した値と突き合わせる。非退化の自己検査として ⒜`s2_assert_work_area_bottom_moves` が「その 2 水準で下端が実際に動く」ことを檻自身が `assert` し（不動点に落ちた空虚な緑を防ぐ・記憶〈2.2 の教訓〉）⒝`s2_resolved_work_area` が解決が `WorkAreaResolution::Contains` であること（最近傍フォールバックで解決していない＝合成レイアウトが退化していない）を毎回 `assert` する |
| 緑の採取コミット | _（7.1）_ |
| 緑の実行出力 | _（7.1・全水準）_ |

#### 赤の実行出力（`cargo test -p areka -- --ignored s2_red_ --test-threads=1`）

```text
running 4 tests
test emo2_boot::frame::tests::s2_red_ground_point_preserved_at_dpi96 ... ok
test emo2_boot::frame::tests::s2_red_ground_point_preserved_from_dpi120_to_dpi192 ... FAILED
test emo2_boot::frame::tests::s2_red_ground_point_preserved_from_dpi96_to_dpi120 ... FAILED
test emo2_boot::frame::tests::s2_red_ground_point_preserved_from_dpi96_to_dpi192 ... FAILED

failures:

---- emo2_boot::frame::tests::s2_red_ground_point_preserved_from_dpi120_to_dpi192 stdout ----

thread 'emo2_boot::frame::tests::s2_red_ground_point_preserved_from_dpi120_to_dpi192' panicked at crates\areka\src\emo2_boot\frame.rs:3165:13:
assertion `left == right` failed: dpi 120→192: 接地点 Y が変化後の work area 下端から外れている（work area が動いたのに位置が再射影されていない＝S2・Req 4.1/4.2/4.6）: scope=0 before=S2Row { scope: 0, ground: (1700, 1432), wa_bottom: 1432 } after=S2Row { scope: 0, ground: (1700, 1432), wa_bottom: 1396 }
  left: 1432
 right: 1396

---- emo2_boot::frame::tests::s2_red_ground_point_preserved_from_dpi96_to_dpi120 stdout ----

thread 'emo2_boot::frame::tests::s2_red_ground_point_preserved_from_dpi96_to_dpi120' panicked at crates\areka\src\emo2_boot\frame.rs:3165:13:
assertion `left == right` failed: dpi 96→120: 接地点 Y が変化後の work area 下端から外れている（work area が動いたのに位置が再射影されていない＝S2・Req 4.1/4.2/4.6）: scope=0 before=S2Row { scope: 0, ground: (1700, 1444), wa_bottom: 1444 } after=S2Row { scope: 0, ground: (1700, 1444), wa_bottom: 1432 }
  left: 1444
 right: 1432

---- emo2_boot::frame::tests::s2_red_ground_point_preserved_from_dpi96_to_dpi192 stdout ----

thread 'emo2_boot::frame::tests::s2_red_ground_point_preserved_from_dpi96_to_dpi192' panicked at crates\areka\src\emo2_boot\frame.rs:3165:13:
assertion `left == right` failed: dpi 96→192: 接地点 Y が変化後の work area 下端から外れている（work area が動いたのに位置が再射影されていない＝S2・Req 4.1/4.2/4.6）: scope=0 before=S2Row { scope: 0, ground: (1700, 1444), wa_bottom: 1444 } after=S2Row { scope: 0, ground: (1700, 1444), wa_bottom: 1396 }
  left: 1444
 right: 1396


failures:
    emo2_boot::frame::tests::s2_red_ground_point_preserved_from_dpi120_to_dpi192
    emo2_boot::frame::tests::s2_red_ground_point_preserved_from_dpi96_to_dpi120
    emo2_boot::frame::tests::s2_red_ground_point_preserved_from_dpi96_to_dpi192

test result: FAILED. 1 passed; 3 failed; 0 ignored; 0 measured; 594 filtered out; finished in 0.02s
```

#### この赤が §1.2 のゲートを名指しで撃っていること

`S2Row` の 3 フィールドは §1.2 の欠陥構造をそのまま外形化したものである:

- `ground: (1700, 1444)` が**変化の前後で 1 bit も動いていない** — §1.2 の「`None` のとき当該窓に対して**何もしない**——寸を触らないだけでなく、**位置の再射影も行わない**」そのもの。窓へ書込が起きていれば接地点は必ず動く
- `wa_bottom` が `1444 → 1432`／`1444 → 1396`／`1432 → 1396` と動いている — **接地すべき下端は確かに変わっている**（＝「保つべき work area が変わっているのに位置が再射影されない」の左辺）
- `ground.0`（下端中央の X）は全水準で `1700` のまま一致 — 失敗しているのは **Y 成分だけ**であり、S1（X 成分の汚染）とは別の欠陥であることが出力から読み取れる

檻の非空虚性は `s2_assert_ground_point_invariant` 冒頭の `refresh_targets` 検査が担う（`Changed<DPI>` が発火せず DPI 相が窓を訪れてすらいなかった場合は、接地点の assert より**先に**その旨で落ちる）。すなわちこの赤は「DPI 相が当該窓を実際に訪れ、報告源を引き、`None` を受け取り、そのまま何もせずに抜けた」ことによる失敗であって、檻の組み違いによる失敗ではない。

#### ゲート機構（`#[ignore]`）と 5.2／7.1 への申し送り

赤 4 件は `#[ignore = "S2 赤証跡（是正未投入では失敗する・タスク 4.4）。再現: cargo test -p areka -- --ignored s2_red_"]` で通常実行から外してある（`at_dpi96` の 1 件のみ理由文が「120/192 が失敗する」形）。常時失敗する檻を置くと `cargo test` が門として無価値になり以後の全タスクの検証を潰すためである（S1 側 `window_pos.rs` の `s1_red_*` と同一の流儀・§3.1）。`cargo test -p areka` は本タスク着地後も**緑**（592 → **594**・常時走る随伴 2 件の増分。赤 4 件は ignored 計上）。

**タスク 5.2／7.1 は是正配線と同時に `#[ignore]` を 4 件とも外し、常時走る回帰檻へ昇格させること**（Req 5.1 の常時テスト化）。**dpi96 の 1 件も外す**——「96 では緑」は是正後も成立する性質であり、外して初めて 96 通過／120・192 失敗という非対称の記録が回帰檻として保存される。外し忘れると 5.2 の完了状態「4.4 の檻が緑に変わる」がゲートを掛けたまま形式的にだけ満たされる。

#### 常時走る随伴 2 件（5.2 が是正を**誤って**実装した場合に赤になる前方ガード）

| 件名 | 何を固定するか | 5.2 のどの誤りを撃つか |
| --- | --- | --- |
| `s2_control_some_report_path_reprojects_and_keeps_balloon_offset`（`frame.rs:3239`） | 寸の再導出結果が**得られる**（`Some`）経路の非退行——新物理寸へ reconcile され、接地点の X が保存され Y が変化後の work area 下端へ再射影され、随伴恒等式 `balloon − char ≡ BalloonFollow.offset`（Req 4.4）が保たれ、route は `DpiReproject` のまま（D13） | 分離の実装が `Some` 経路まで作り替えた場合 |
| `s2_dpi_phase_writes_nothing_when_the_ground_point_already_holds`（`frame.rs:3308`） | **書込が起きるのは現位置が接地点規約に違反しているときだけ**（design「dpi_phase 位置/寸分離 > Risks / Req 4.5 との整合」）。work area が動かず既に接地している走行では、`Changed<DPI>` が立っていても char／balloon とも書込ゼロ | `None` 経路の再射影を「常に書く」形で実装し、DPI 通知のたびに同値の再配置が走って Req 4.5（現状維持）を壊した場合 |

後者は「書込ゼロ」の主張が空虚にならないよう、**同一ハーネスが書込を検出できること**を先に positive witness（異寸報告のある DPI 相では `Arrangement.offset` の sentinel が実際に動く）で示してから否定側を主張する（記憶〈3.2 の空虚性・2 例目〉）。

#### 緑側の先行確認（本タスクで実施・是正はツリーに残していない）

檻が是正後に緑へ反転することを、5.2 相当の分岐（design D7＝`refresh_scale_report` が `None` かつ char 窓なら現 `WindowPos.size` のまま `resize_window_to(..., DpiReproject)` を通す）を**一時的に当てて実測し、直後に完全に戻した**。結果は赤 4 件すべて `ok`（`4 passed; 0 failed`）・`cargo test -p areka` 全体も **594 passed / 0 failed** で随伴 2 件を含め緑のまま。**是正はツリーに残していない**（4.5 の実機採取が是正未投入ビルドを要求するため・§2 の冒頭注記）。これにより本檻は「今赤・是正後緑」の両側が実行で確かめられている（空虚な赤ではない）。

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
| §2 | 4.5（**セッション①のみ記入済み・②未採取**） | 実機 2 セッションの採取結果・Q1〜Q4・S1〜S3′ の痕跡 |
| §4 | 7.4 | 是正後の実機再サインオフ |

**§2 は 2 セッションとも採取済み**（①=2026-07-31 16:35／②=同 21:02）。ただし②は S4（§2.7）により **DPI 変化が 1 度も届かず** `SESSION-QUOTA: FAIL (0/6)`。**S4 是正後に②を採り直すこと**——それまで「ドラッグ以外の経路」は未観測のままである。

### 5.1 S4 の是正タスク新設が必要（tasks.md への申し送り）

§2.7 の S4 は**既存タスクのどれにも属さない**:

| 既存タスク | S4 を直すか | 理由 |
| --- | --- | --- |
| 5.1（S1 是正） | ✗ | `WM_DPICHANGED` ハンドラ内の分岐が対象。**メッセージが来ない**問題は範囲外 |
| 5.2（S2 是正） | ✗ | DPI 相の射影ゲートが対象。DPI 相が**起動されない**問題は範囲外 |
| 6.1／6.2（S3／S3′ 是正） | ✗ | 遷移ガードの配線が対象。ガードが読む work area が**凍結している**問題は範囲外 |

新設タスクが最低限満たすべきこと:

1. `Monitor` の**値の変化検出**を `PartialEq` から分離する（`handle` 同一性の意味論は維持したまま、`bounds`／`work_area`／`dpi`／`is_primary` の差分判定を別途用意する）。**既存檻 `test_partial_eq_compares_handle_only` は正しいので壊さない**——誤りは消費側にある
2. `detect_display_change_system` が実際に `Monitor` を更新することを檻で固定する（handle 不変・値だけ変わる探針で赤→緑）
3. `SetProcessDpiAwarenessContext` の戻り値をログ化する（[`runtime/mod.rs:111`](../../../crates/wintf/src/runtime/mod.rs) の `let _ =` を撤去）
4. モニタ表更新後に**窓の DPI・寸・位置を再導出する経路**を通す（`WM_DPICHANGED` に依存しない側の駆動）
5. S4 是正後にセッション②を採り直し、本書 §2.2／§2.3／§2.5 の②列を埋める

**順序**: S4 是正は 5.1／5.2 と**独立に着手できる**（触るファイルが `monitor.rs`／`monitor_systems.rs`／`runtime/mod.rs` で重ならない）が、**②の採り直しは 5.1／5.2 の投入より前**に行うこと（§0.1 の絶対制約——是正投入後は実機再現が失われる）。

**メンテ規約**:

1. §1 の引用先を編集したコミットは、**同じコミットで §1 の `file:line` も更新する**。行番号だけの更新でも構わないが、構造名が変わったときは構造名も直す。
2. 既存の節を**上書きしない**。後続タスクは自分の担当節にのみ追記する（§3.1 と §3.2 は互いに独立）。
3. 手順・判定語・grep 規則は本書に書かない（`diagnosis-procedure.md` が正本）。本書に載せるのは**結論と証跡**だけである。
4. 実機ログは該当行の引用のみを転記し、生ログの保存パスと採取コミット SHA を併記する（リポジトリ配下へ生ログを置かない）。
