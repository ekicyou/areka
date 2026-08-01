# Implementation Plan

> **フェーズ順序の絶対制約**（design.md「実装フェーズ順序」・Req 2.7）: タスク 1〜4 が Phase A（観測増設・掃除・純関数抽出＝すべて挙動不変）と Phase B（実機採取）、タスク 5〜6 が Phase C（是正）、タスク 7 が Phase D（檻・最終サインオフ）。**タスク 4.5／4.7（実機採取）は必ずタスク 5 以降より前に完了させる**——S1/S2 是正を投入すると消失の実機再現自体が起きなくなり、Q1〜Q4 の確定材料が永久に失われるため。
>
> **Phase B′ の追加（2026-07-31・4.5 セッション②が S4 を確定）**: タスク **4.6** は是正だが Phase C ではなく **Phase B′（観測装置の修理）**に属する。編集面が `monitor.rs`／`monitor_systems.rs`／`runtime/mod.rs` の 3 ファイルに限られ、S1（`window_pos.rs`）・S2（`frame.rs`）・S3（`follow.rs`）の被検体に一切触れないため、「是正未投入のビルドで採取する」制約を破らない。実行順は **4.6 → 4.7（②採り直し）→ 5.1 → 5.2 → 6.1 → 6.2 → 7.x**。

- [ ] 1. Phase A 基盤: 観測語彙と恒久ログ基盤の新設
- [x] 1.1 配置観測モジュールの新設（経路語彙・専用ログ target・レコード型）
  - 窓位置を書き込んだ経路の語彙を列挙型として定義する（spawn 初期配置・位置復元・アンカー変化・毎フレーム再スナップ・DPI 再射影・位置据置きリサイズ・バルーン随伴の 7 種）
  - _改訂（2026-07-31・D13）_: 語彙は 9 種へ拡張（報告回収 `ReportedSizeReconcile`・`\![move]` 明示移動 `MoveCue` を追加）。完了時点の 7 種は当時の設計どおりで、拡張分の enum 追加と配線は 1.4 の是正が担う
  - 恒久観測の専用ログ target を定数として定義し、全出力を既定で無効（診断手順の設定でのみ点灯）となる水準に置く
  - モニタ 1 台分の観測レコード型（識別子・境界矩形・work area 矩形・DPI・プライマリ標識）を、表示基盤の型に依存しない純データとして定義する
  - レコード文字列の組立を純関数化し、診断手順書の grep 判定語が意図せず変化したら檻が失敗するよう固定する
  - 完了状態: 新モジュールが公開され、経路語彙とレコード組立の単体テストが緑になる。既定のログ設定では本 target の出力が 1 行も出ない
  - _Requirements: 1.2, 1.7, 2.4_

- [x] 1.2 起動時モニタスナップショットの出力
  - placement の全判断が読む権威 Resource の構築点を Requirement 1.1 の正典出力点とし、検出した全モニタの識別子・境界矩形・work area 矩形・DPI・プライマリ標識を物理 px で 1 回出力する
  - ゴースト窓配置準備のモニタ列挙点でも同じ共有ヘルパを呼び、呼出点タグで区別できるようにする
  - 完了状態: 診断設定で起動すると、全モニタの work area を含む行が呼出点タグ付きで出力され、ログだけからモニタ構成を再構成できる
  - _Requirements: 1.1_
  - _Depends: 1.1_

- [x] 1.3 (P) 表示基盤側の観測水準是正とフィールド補強
  - DPI 変化通知の受理時に「OS 提案位置に基づく位置変更を実際に行ったか否か」を出力する箇所の水準を、診断手順が有効化する水準へ引き上げる（現状は診断手順で点灯しない水準にあり、2026-07-18 の偽陰性の直接原因）
  - 実際の窓位置書込を行う共通経路の実施ログも同様に水準を是正する
  - モニタ列挙のログに work area 矩形とモニタ識別子のフィールドを追加する
  - 完了状態: 診断設定で DPI を変更すると「提案位置を書いた／書かなかった」が必ずログに現れ、モニタ列挙行に work area が含まれる
  - _Requirements: 1.1, 1.3_
  - _Boundary: wintf 表示基盤（ウィンドウメッセージ・モニタ列挙）_

- [x] 1.4 経路タグの単一ライター配管と窓移動レコードの出力
  - 窓位置・寸法を書き込む単一ライターおよびその上流（リサイズ・移動・位置据置きリサイズ・バルーン随伴）へ経路タグを引数として配管する（挙動は一切変えない純粋なリファクタリング）
  - 書込成功時に、経路・エンティティ識別子・窓種別（キャラ／バルーン）・scope・物理位置・物理寸・当該窓の DPI を 1 レコードとして専用 target へ出力する
  - エンティティ識別子は表示基盤側ログ（scope を持たない）との結合キーとして必ず含める
  - _是正（2026-07-31・実装レビュー #1・D13）_: `reconcile_window_size` へ route を**引数**で配管し、DPI 相（`dpi_phase_with` 経由）は `DpiReproject`・drain 相（`reconcile_reported_sizes` 経由）は `ReportedSizeReconcile` を渡す（起動時 k₀ 補正を DPI 由来と偽らない）。`frame.rs:717-718` の「2 呼出元はいずれも DPI 由来」の誤コメントを修正する
  - _是正（同上）_: `\![move]`（`move_window_to` の対象窓）へ `MoveCue` を割り当てて記録する（随伴バルーンは従来どおり `BalloonFollow`。既存檻「move cue は記録されない」は「MoveCue で記録される」形へ更新）
  - _是正（同上）_: frame.rs のテストへ route 割当の檻を追加する——①resnap 経由の書込が `Resnap` で記録される ②dpi 相経由と drain 相経由が**別々の**経路名で記録される（`placement::test_support::capture_logs` は `pub(crate)` で frame.rs から到達可能）。follow.rs:4252-4262／4281-4291 の引数間空行の整形傷も直す
  - 完了状態: 診断設定でゴーストを起動しドラッグすると、窓が動くたびに経路名つきレコードが出力される。既存テスト一式が緑のまま（挙動不変）。DPI 変化ゼロの起動で `DpiReproject` レコードが 1 行も出ない
  - _Requirements: 1.2, 2.4_
  - _Depends: 1.1_

- [ ] 2. Phase A: 判断分岐の純関数化（抽出のみ・配線しない）
- [x] 2.1 (P) OS 提案位置の採用可否を宣言する契約と純判断関数
  - 窓ごとに「DPI 変化時の OS 提案位置を適用するか」を宣言する component を新設し、未付与＝従来どおり適用（後方互換の既定値）とする
  - 提案位置の採用可否を返す純関数を追加する（適用する場合は書込先座標を返し、外部権威の窓では「書かない」を返す）
  - 全分岐（未付与・適用・外部権威）を網羅する in-source 檻を追加する
  - **この時点では wndproc へ配線しない**（配線は Phase C）。既定値のみが存在するため実行時挙動は不変
  - この単体檻は分岐網羅の補助であり、赤→緑の正証跡は 4.3 のディスパッチ檻が担う
  - 完了状態: 純関数の全分岐テストが緑。既存の表示基盤テストが緑のまま（挙動不変）
  - _Requirements: 4.3, 5.1_
  - _Boundary: wintf 表示基盤（DPI component・ウィンドウメッセージ補助関数）_

- [x] 2.2 (P) work area 解決の判別付き版と可視性の遷移ガード純関数
  - 既存の work area 解決関数の契約を変えないまま、「窓中心がモニタに帰属したか／どのモニタにも属さず最近傍フォールバックが発火したか」を判別して返す版を追加し、既存関数はそれへ委譲する
  - 可視性の遷移ガードを純関数として追加する。規則は「提案矩形がいずれかの work area と交差→そのまま／交差せず旧矩形は交差していた→X のみ clamp／旧矩形も交差していなかった（ユーザーの明示留置）→尊重してそのまま／旧矩形不明→安全側に clamp」
  - 判定は絶対 px の固定値ではなく交差・不変条件で表現し、Y は一切変更しない（Y は射影の所有）
  - 混在 DPI 複数モニタ（非対称 work area・負座標・3200 超座標）の合成レイアウトで全分岐を檻に入れる。**キャラ矩形とバルーン矩形の両ケース**を含める
  - **この時点では配線しない**（配線は Phase C）
  - 完了状態: 遷移ガードと判別付き解決の単体テストが 96/120/192 の各水準で緑。委譲の等価性（既存関数の戻り値が不変）がテストで固定される
  - _Requirements: 3.1, 3.2, 5.1, 5.3, 5.6_
  - _Boundary: areka placement（追従・work area 解決）_
  - _Depends: 1.4_

- [ ] 3. Phase A: ゴースト窓レジストリの despawn 掃除
- [x] 3.1 (P) レジストリからの scope 粒度除去と despawn フック
  - ゴースト窓レジストリに「指定エンティティが属する scope エントリを除去する」操作を追加する（不一致・二重除去は no-op で panic しない）
  - ゴースト窓マーカーの削除フックから当該操作を駆動し、あらゆる despawn 経路を呼出点結合なしで拾えるようにする
  - フックは Resource のみを操作し、生存しているエンティティの component には一切触れない
  - W5/W6 同居契約: 本タスクが触る spawn の編集内容を、干渉台帳の流儀で W6 `balloon-visibility` へ申し送る
  - 完了状態: キャラ窓を despawn すると当該 scope がレジストリから消え、対の後追い despawn は no-op になる。生存 scope の位置・寸法・追従関係は不変。W6 への申し送りが記録されている
  - _Requirements: 6.1, 6.4_
  - _Boundary: areka placement（spawn・ゴースト窓レジストリ）_

- [x] 3.2 統合タスク: 消費側の存在確認と警告水準の区別（追従層とフレーム層をまたぐ）
  - 本タスクは責務境界を意図的にまたぐ**統合タスク**である（リサイズ入口＝追従層／毎フレーム再スナップ・寸法報告の回収＝フレーム層）
  - 各消費側で処理対象エンティティの存在を確認する
  - 「エンティティ不在＝破棄済み」は正常終了系として debug 水準で打ち切り、他の scope の処理を継続する
  - 「エンティティは実在するが接地点規約の component が欠落」は従来どおり警告水準（真の異常）として区別する
  - W5 同居契約: 本タスクはフレーム層の DPI 相・再スナップ相・回収相の近傍ハンクのみを触り、テキストスケール相とバルーンモデル写像には手を入れない。**先着後 rebase を W5 `kero-balloon` へ申し送る**
  - 完了状態: 終了処理でゴースト窓が破棄された後のフレームで、破棄済み窓に対する警告以上のログが 1 行も出ない。接地点規約の欠落だけが警告として残る。kero-balloon への rebase 申し送りが記録されている
  - _Requirements: 6.2, 6.3_
  - _Boundary: areka placement（追従層の消費入口）＋ areka emo2_boot（再スナップ相・回収相）_
  - _Depends: 3.1_

- [ ] 4. Phase A/B: 診断手順書・確定台帳・是正前の赤証跡採取
- [x] 4.1 診断手順書の作成
  - 起動コマンド（絶対パス）・有効化するログ設定・有界自動終了の設定・ログ保存先を、第三者が同一手順を再実行できる粒度で記述する
  - 観測点 × ログ target × 水準の対応表を載せ、「手順で有効化されない水準にある観測点を『発生 0 回』の根拠に用いない」ことを明文化する
  - 実機採取を 2 セッション（①ドラッグによるモニタ跨ぎのみ ②ドラッグ禁止・OS 設定側から DPI 変更のみ）として規定し、ログを別々に保存する手順を書く
  - 充足条件を経過時間ではなく DPI 変化通知の受理回数（各 scope × 各方向 × 3 回＝12 回以上）で規定し、**2 段 grep 規則**（第 1 段でレコードから scope→キャラ窓エンティティの対応表を作り、第 2 段で当該エンティティの受理行を数える／方向は同行の新旧 DPI 比較で判定）を判定語とともに固定する
  - 決定論化できない残余（OS が実際に提示する提案矩形・実モニタ列挙）の実機サインオフ手順と合否判定語を記述する
  - 完了状態: 手順書だけを読んだ第三者が、同じ設定でゴーストを起動し、受理回数を機械的に数えてセッションの充足可否を判定できる
  - _Requirements: 1.4, 1.5, 1.6, 1.8, 1.9, 5.5_
  - _Depends: 1.2, 1.3, 1.4_

- [x] 4.2 診断レポートへの静的構造証跡の先行登記
  - 確定台帳としての診断レポートを新設し、コード読解のみで確定した位置権威の欠陥を「静的構造証跡」クラスとして file:line 引用付きで登記する
  - 登記対象は S1（接地点の X 成分を再計算せず OS 提示値を素通しする）・S2（位置の再射影が窓寸の再導出結果に条件付けられ、得られない経路で欠落する）・S3（キャラ窓の水平方向に可視性の不変条件が無く、work area 解決の最近傍フォールバックが異常を隠す）・S3′（バルーン矩形の可視性がどの経路でも検査されない）の 4 件
  - 各項目に、それが未充足にする受入基準の ID を明記する
  - 完了状態: レポートに 4 件が file:line と未充足 AC 付きで並び、実機採取の結果に依存せず確定済みであることが読み取れる
  - _Requirements: 2.8_

- [x] 4.3 (P) S1 の赤証跡採取（表示基盤ディスパッチ檻）
  - DPI 変化通知をヘッドレスにディスパッチする既存檻を拡張し、「外部権威を宣言した窓では DPI が更新される一方で OS 提案位置の書込コンテキストが確立されない／宣言の無い窓では確立される」ことを検証する檻を書く
  - **是正未投入のコミット（Phase A 完了時点）に対して実行し、失敗（＝提案位置が無条件に採用される）を実行記録として残す**
  - dpi=96 では提案矩形が現位置と一致し差が出ないため通過し、120/192 で失敗することを併せて記録する
  - 実行記録の追記先は診断レポートの **S1 専用節**とし、4.4 が書く S2 節とは重ならない
  - 完了状態: 檻が是正前コードに対して赤で落ち、その出力が診断レポートの S1 節に実行記録として引用されている
  - _Requirements: 5.4_
  - _Boundary: wintf 表示基盤（ウィンドウメッセージ・ディスパッチ檻）_
  - _Depends: 2.1, 4.2_

- [x] 4.4 (P) S2 の赤証跡採取（DPI 相の位置再射影檻）
  - 偽ウィンドウハンドルのヘッドレス World・合成マルチモニタ・偽の寸法報告源（「再導出結果なし」固定）・DPI 注入（96→120／96→192／120→192）で DPI 相を回す檻を書き、接地点（下端中央）が変化の前後で保たれることを検証する
  - **是正未投入のコミットに対して実行し、失敗（＝再導出結果が得られない経路で位置が再射影されない）を実行記録として残す**。dpi=96 では旧 Y と新 Y が自己整合して通過することも記録する
  - 判定は絶対 px ではなく接地点の不変条件として表現する
  - 実行記録の追記先は診断レポートの **S2 専用節**とし、4.3 が書く S1 節とは重ならない
  - 完了状態: 檻が是正前コードに対して赤で落ち、96 通過・120/192 失敗の非対称が診断レポートの S2 節に記録されている
  - _Requirements: 5.4_
  - _Boundary: areka emo2_boot（DPI 相の檻）_
  - _Depends: 1.4, 4.2_

- [x] 4.5 実機 2 セッションの採取と未確定 4 問の確定
  - **開発者が実機で実行するゲートタスク**（実マルチモニタ・混在 DPI・手動ドラッグ／OS 設定変更を要する）。自律実装モードではここで停止し、開発者へ実行を依頼すること——採取結果を推測や代替で埋めてはならない
  - **Phase A 完了・是正未投入のビルド**で、手順書どおり 2 セッション（ドラッグのみ／ドラッグ禁止・OS 設定変更のみ）を採取し、各セッションが受理回数の下限を踏破したことを 2 段 grep で確認する
  - 消失の有無を判定語で検査し、消失時のキャラ窓矩形を全モニタの work area と突合して「真の不可視」か「可視領域内の見落とし」かを判別する
  - ドラッグ中のマウス移動量と窓移動量の対応を数値で示し、暴走の有無を評価する
  - 消失がドラッグ以外の経路で起きた場合は、最終位置を書き込んだ主体を経路名で名指しし、バルーン消失がキャラ随伴か独立かを相対位置の保存で判別する
  - 受理回数の下限を踏破した 2 セッションで消失痕跡が検出されなかった場合は「再現しない」と結論し、除外できるのは実機でしか確定できない残余仮説への追加修正のみであること（静的確定分の是正と檻は除外しない）を明記する
  - S1〜S3′ の各項目について実機ログ上の痕跡の有無を記録する（痕跡が無くても確定は取り消さない）
  - 完了状態: 診断レポートに Q1〜Q4 の回答が実機ログの該当行引用付きで並び、受理回数の充足が数値で示されている
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.9_
  - _Depends: 4.1, 4.2, 4.3, 4.4_
  - _完了（2026-07-31・開発者が実機で 2 セッションを実行）_:
    - **①ドラッグ**（`20260731-163422-rel\session1-drag.log`・`0db483e`・release）: `SESSION-QUOTA: PASS`（44/12）・`VANISH-TRACE: NONE`（4,098 件）・**S1 陽性 84/84**・**S2 陽性 1/45**・`RESIDUE-A/B: PASS`・`TEARDOWN-SILENCE: FAIL`
    - **②OS 設定**（`20260731-session2\session2-osdpi.log`・同ビルド）: `SESSION2-NO-DRAG: PASS`・`SESSION-QUOTA: **FAIL (0/6)**`——**拡大率を 7 回変更しても `WM_DPICHANGED` が 0 件**。これは採取の失敗ではなく **Req 2.4 への回答**（「ドラッグ以外の経路」には書き手が存在しない）であり、原因 **S4** を `diagnosis-report.md` §2.7 に file:line で確定登記した
    - Q1〜Q4・S1〜S3′ の痕跡・残余 A/B は §2.3〜§2.6 に登記済み
  - _申し送り_: **②は S4 是正後に採り直す**（タスク 4.7）。Req 2.10 により、S4 是正前の②を「消失は起きない」の根拠に用いてはならない

- [x] 4.6 **S4 是正: 同一性判定と値の変化検出の分離（Phase B′＝観測装置の修理）**
  - 本タスクは **S1〜S3′ の被検体に一切触れない**。編集面は `crates/wintf/src/ecs/window/monitor.rs`／`.../layout/systems/monitor_systems.rs`／`crates/wintf/src/runtime/mod.rs` の 3 ファイルのみで、S1（`window_pos.rs`）・S2（`frame.rs`）・S3（`follow.rs`）と交差しない。ゆえに「是正未投入のビルドで採取する」絶対制約（tasks.md 冒頭）を破らない
  - モニタの**値の変化**を判定する述語を `Monitor` へ新設する。**`impl PartialEq for Monitor` と既存檻 `test_partial_eq_compares_handle_only`（`monitor.rs:254-264`）は無改変**（Req 7.6・D14 帰結⑴）——誤りは同一性判定ではなく、それを変化検出に流用した消費側にある
  - 新述語は追従対象フィールド（境界矩形・work area 矩形・DPI・プライマリ標識）を網羅し、**フィールド追加時にコンパイラが漏れを指摘できる形**（構造体分解パターン）で書く（D14 帰結⑵）
  - `detect_display_change_system` の更新分岐を新述語へ切り替える（`monitor_systems.rs:229-236`）
  - モニタ表が更新されたとき、当該モニタ上の窓の DPI・寸・位置を再導出する経路を **`WM_DPICHANGED` の受理有無に依存せず**駆動する（Req 7.3・D14 帰結⑷）。**`WM_DPICHANGED` が 0 件である機序は未確定であり、それに依存しない駆動路を用意することが是正の本体である**
  - `SetProcessDpiAwarenessContext` の戻り値を `runtime/mod.rs:111` の `let _ =` から救い出してログに残す（Req 7.4・D14 帰結⑸）。成否が観測できない設定は Req 1.5 が「発生 0 回」の根拠に使うことを禁じている
  - **赤証跡を先に採る**: 識別子が不変で値のみが変化するモニタ構成に対し「モニタ表が更新される」ことを主張する檻を書き、**是正未投入の状態で赤になることを実行記録として残す**（Req 7.5・本 spec の流儀＝`#[ignore]`＋再現コマンド inline＋レポート転記。追記先は `diagnosis-report.md` の **S4 専用節**とし §3.1／§3.2 と重ならない）
  - **檻の空虚性を避けること**: 「更新される」を総数や `handle` 一致で主張すると、更新が起きなくても緑になり得る。**更新後の `work_area`／`dpi` の実値**を assert し、探針が不動点でないことを自己検査する（[[2.2 の教訓]]・[[3.2 の教訓]]）
  - 完了状態: 識別子不変・値のみ変化の構成でモニタ表が実際に更新され、その檻が `#[ignore]` 無しで緑になる。DPI awareness 設定の成否がログに現れる。既存の wintf テスト一式が緑のまま
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 2.8, 2.10_
  - _Boundary: wintf 表示基盤（モニタ型・表示構成変更システム・ランタイム初期化）_
  - _Depends: 4.5_

- [x] 4.7 セッション②の再採取（**開発者が実機で実行するゲートタスク**）
  - **4.6 の是正のみを投入し、S1/S2/S3 是正は未投入のビルド**で、`diagnosis-procedure.md` §6.2 のセッション②を採り直す（ドラッグ禁止・OS 表示設定から拡大率のみ変更）
  - 自律実装モードではここで停止し、開発者へ実行を依頼すること——採取結果を推測や代替で埋めてはならない
  - `SESSION-QUOTA` が下限を踏破したことを 2 段 grep で確認する。踏破しなければ 4.6 の是正が不十分であり、4.6 へ差し戻す（Req 2.10）
  - 踏破したうえで、Req 2.4 の「ドラッグ以外の経路」について最終位置を書き込んだ主体を経路名で名指しし、消失の有無を `VANISH-TRACE` で判定する
  - S1〜S4 の実機痕跡を `diagnosis-report.md` §2.5 の②列へ追記し、§2.2／§2.3 の②行を埋める
  - 完了状態: §2 の②列がすべて埋まり、Req 2.4 が実機ログの引用付きで回答されている
  - _Requirements: 1.8, 1.9, 2.1, 2.4, 2.5, 2.9, 2.10_
  - _Depends: 4.6_
  - _完了（2026-08-01・開発者が実機で実行・commit `f8bcfd0`・release）_: 生ログ `%LOCALAPPDATA%\areka-diag\20260801-s2-crash\out.log`（1,861 行・`EXITCODE=0`）。**`SESSION-QUOTA: PASS (6+6 / 6)`**（scope0 3/3・scope1 3/3・Req 1.9 改訂版で判定）・`SESSION2-NO-DRAG: PASS`・`VANISH-TRACE: NONE`（37 件）・**`TEARDOWN-SILENCE: PASS`**（②は窓を閉じて終了＝`despawn_smoke_targets` を通らない経路ゆえ①の FAIL は打ち消さない）。`Updating Monitor entity` 6 件・`windows_redriven=4` ×6＝**S4 是正の実機成立**。Req 2.4 充足＝ドラッグ以外の経路で位置を書いた主体は **`DpiReproject` 単独**（`WM_DPICHANGED` 24 件はすべて `old==new` の空振りで位置を書いていない）。`diagnosis-report.md` §2.2〜§2.5 へ登記済み
  - _新規観測（S5 候補・§2.5.3）_: **work area の変化に接地点が追随しない**（192 時に `ground_y=2100` のまま・新下端 2064 に対し **+36px**・6/13 件・すべて低→高方向）。**Req 4.1／4.2 の違反ではない**（接地点は前後で保存されている）ため Req 2.7 により本 spec の修正対象へ自動的には入らない。**扱いは開発者の裁定待ち**（先送りなら 4 点セット必須）。**6.1 着手時に要確認**——遷移ガードは「work area と交差するか」を見るため、はみ出しは検出されない可能性がある

- [ ] 5. Phase C: 位置権威の是正
- [x] 5.1 S1 是正: OS 提案位置の源断ちとゴースト窓への外部権威宣言
  - DPI 変化通知のハンドラを純判断関数の消費へ組み替え、「書かない」判定のときは提案位置の書込も書込コンテキストの確立も行わないようにする（DPI component の更新と変化通知の発火は無条件で従来どおり）
  - ゴースト窓の spawn 時に、キャラ窓・バルーン窓の両方へ外部権威の宣言を付与する
  - 付与漏れが S1 再発の穴になるため、全 scope × 窓 2 種への付与を檻で固定する
  - 診断レポートで確定した機構のみを変更し、それ以外の機構には手を入れない
  - **4.3 の赤 4 件の `#[ignore]` を全て外す**（`window_pos.rs` の `s1_red_*`＝dpi96／dpi120／dpi192／no-write-context）。**dpi96 の 1 件も外すこと**——「96 では緑」は是正後も成立する性質であり、外して初めて 96/120/192 の非対称が回帰檻として保存される。外し忘れると本タスクの完了状態「4.3 の檻が緑に変わる」がゲートを掛けたまま形式的に満たされてしまう
  - 完了状態: 4.3 の檻が **`#[ignore]` 無しで**緑に変わる。DPI 変化後のゴースト窓の位置が areka の確定値のまま残り、非ゴースト窓は従来どおり提案位置を採用する
  - _Requirements: 2.7, 2.8, 4.3_
  - _Boundary: wintf 表示基盤（ウィンドウメッセージ）＋ areka placement（spawn）_
  - _Depends: 2.1, 4.5, 4.7_
  - （**4.7 より後でなければならない**——本タスクが投入されるとセッション②の実機再現が失われる＝tasks.md 冒頭の絶対制約と Req 2.10）

- [ ] 5.2 S2 是正: 位置の権威と寸の権威の分離
  - DPI 相を「DPI が変化したキャラ窓は、窓寸の再導出結果が得られたか否かに関わらず必ず射影を一度通る」形へ変更する。再導出結果が得られた場合は従来経路、得られない場合は現在の寸のまま再射影する
  - 表示側クレートの戻り値契約は変更しない（分離は本フェーズ側だけで解決する）
  - バルーン窓は再導出結果が無いとき位置据置きのままとし、随伴はキャラ窓確定後の追従が担う
  - 窓寸が未確定（窓生成前）の場合は現状維持のまま打ち切り、その事実をログに残す
  - 正常系では同寸・同 work area のため書込ゼロで抜ける（現状維持）ことを確認し、書込が発生するのは現位置が接地点規約に違反しているときだけであることを檻で示す
  - **4.4 の赤 4 件の `#[ignore]` を全て外す**（`frame.rs` の `s2_red_*`＝dpi96／96→120／96→192／120→192）。**dpi96 の 1 件も外すこと**（理由は 5.1 の同項と同じ）。areka crate 内の `#[ignore]` はこの 4 件だけなので、`grep -n '#\[ignore' crates/areka/src` がゼロになることで確認できる
  - 完了状態: 4.4 の檻が **`#[ignore]` 無しで**緑に変わる。不可視・未表示・寸不変のいずれの経路でも接地点が保たれ、可視化時に変化後の DPI 相当の寸と規約準拠の位置で表示される
  - _Requirements: 2.7, 2.8, 4.1, 4.2, 4.5, 4.6_
  - _Depends: 4.5, 5.1_
  - （現状維持のべき等性が成立する前提が「S1 是正により生位置が areka の確定値であること」ゆえ 5.1 の後でなければならない＝並行不可）

- [ ] 6. Phase C: 可視性の構造保証
- [ ] 6.1 S3 是正: キャラ窓経路への遷移ガード配線
  - 射影の下流・外側に遷移ガードを配線し、非ドラッグの配置系経路（アンカー変化・毎フレーム再スナップ・DPI 再射影・報告回収 `ReportedSizeReconcile`＝D13）でのみ発火させる
  - ドラッグ経路・位置復元経路・`\![move]`（`MoveCue`）には配線しない（明示操作の尊重・復元時の可視化保証は別 spec の所有・D13）
  - clamp 先の work area は射影が Y に用いたものを貫通させ、射影関数自体の契約は変えない
  - clamp 発火時と、非ドラッグ経路での最近傍フォールバック発火時を警告水準で記録する（ドラッグ経路は従来の水準を維持し spam させない）
  - 位置決定に必要な入力が取得できない場合は位置を変更せず現状維持のまま警告を残す
  - 完了状態: 混在 DPI 複数モニタの合成レイアウトで、非ドラッグ要因ではキャラ窓が全 work area 非交差の状態へ遷移しなくなる。ユーザーが自ら画面外へ留置した窓は引き戻されない
  - _Requirements: 3.1, 3.2, 3.3_
  - _Depends: 2.2, 4.5_

- [ ] 6.2 S3′ 是正: バルーン矩形への遷移ガード適用
  - バルーン随伴で相対位置の恒等式から提案位置を出した後、同一の遷移ガードをバルーン矩形（旧矩形＝現在位置・提案位置＋現在の寸）へ適用する
  - 完全不可視への遷移のみを X の clamp で防ぎ、ユーザーが留置したバルーンは尊重する（キャラ窓と完全に同一の規則・同一の純関数）
  - clamp によりバルーンがキャラと部分的に重なり得ることを許容する（見えない会話より重なった会話を優先する裁定）
  - 画面端での左右反転などの美観配置政策は本 spec の対象外とし、警告出力をその縮退シームとして残す
  - 完了状態: キャラ窓が画面端で clamp された合成レイアウトでも、バルーン矩形がいずれかの work area と交差する状態が保たれる
  - _Requirements: 3.4_
  - _Depends: 6.1_
  - （6.1 と同一ファイル・同一関数群を触るため並行不可）

- [ ] 7. Phase D: 回帰檻の完成と最終検証
- [ ] 7.1 赤→緑の実行記録の確定
  - 4.3・4.4 で採取した赤の記録に対応する緑を、是正コミット直後に実行して記録する
  - 是正前は dpi=96 の水準で通過し 96 以外の水準で失敗すること、是正後は全水準で緑になることを、診断レポートに実行出力付きで残す
  - **赤証跡のゲート（`#[ignore]` 等）が全て外れていることを確認する**——4.3 の `s1_red_*` 4 件（dpi96 含む）と 4.4 が置いたゲートの双方。ゲートが残っていれば赤→緑は成立していない
  - 完了状態: 診断レポートに S1・S2 それぞれの赤→緑が実行出力付きで並び、「96 の自己整合が欠陥を隠す」性質が檻として明示されている。**ゲートされた赤証跡が 1 件も残っていない**
  - _Requirements: 5.4_
  - _Depends: 5.1, 5.2_

- [ ] 7.2 (P) 混在 DPI・複数モニタ回帰檻の拡充
  - 再導出結果が得られた経路の非退行を檻で固定する（従来経路が走り、バルーンとキャラの相対位置の恒等式が保存される）
  - キャラ窓・バルーン窓のいずれも全 work area 非交差にならないことを、混在 DPI（120・192 を含む）と複数モニタ work area を注入した合成レイアウトで検証する
  - 実 GPU・実高 DPI モニタを要さず決定論的に成否が判定されることを確認する
  - 位置・寸法の判定を絶対 px の固定値ではなく DPI 水準に対する比または不変条件として表現する
  - 完了状態: 96/120/192 の各水準で檻一式が緑になり、実機を持ち出さずに回帰が検出できる状態になる
  - _Requirements: 4.4, 5.1, 5.2, 5.3, 5.6_
  - _Boundary: areka placement（追従・遷移ガードの檻）＋ emo2_boot（DPI 相の檻のみ）_
  - _Depends: 6.2_

- [ ] 7.3 レジストリ掃除の回帰檻（despawn 消費側の檻を単独所有）
  - despawn フック発火で該当 scope が除去されること・後追いの片割れが no-op になること・掃除の前後で他 scope の位置・寸法・追従関係が不変であることを檻で固定する
  - 掃除後に再スナップと寸法報告の回収が走っても警告が出ず、他の scope を処理し切ることをログ捕捉で確認する
  - _追加（2026-07-31・4.5 セッション①で実測）_: **`despawn_smoke_targets`（`crates/areka/src/main.rs:795-810`）に存在確認を敷く**——query で 4 体を集めてからループで `world.despawn(e)` を呼ぶが、1 体目の despawn が連鎖して残りも破棄するため、後半 3 回が既に無効な entity を叩き `bevy_ecs::world: Could not despawn entity` の `WARN` が 3 件出る（`TEARDOWN-SILENCE: FAIL` の唯一の原因）。3.2 が敷いた**消費側 4 入口**のガードは despawn の**呼出点そのもの**を覆っていなかった。3.2 と同じ区別（entity 不在＝正常終了系／規約 component 欠落＝真の異常）をここにも適用し、檻で固定する
  - 完了状態: 掃除に関する檻一式が緑になり、終了時ログから良性の警告が消えたことがテストで固定される。**`despawn_smoke_targets` 由来の `Could not despawn entity` が 0 件になる**
  - _Requirements: 6.1, 6.2, 6.3, 6.4_
  - _Depends: 3.2_
  - （再スナップ・回収相の檻を 7.2 と同じテストモジュールへ書くため、7.2 との並行は不可＝despawn 消費側の檻は本タスクが単独所有）

- [ ] 7.4 是正後の実機再サインオフ
  - 是正投入後のビルドで診断手順書と同一手順の 2 セッションを再実行し、受理回数の下限を踏破したうえで消失痕跡がゼロであることを判定語で確認する
  - 決定論化できない残余（OS が実際に提示する提案矩形の実値・実モニタ列挙）を実機ログで確認し、合否を判定語で記録する
  - S1〜S3′ の各項目について是正後の実機挙動を診断レポートへ追記する
  - 完了状態: 診断レポートに是正後セッションの受理回数・消失痕跡ゼロ・接地点保存の実測が記録され、実機サインオフが成立する
  - _Requirements: 2.9, 5.5_
  - _Depends: 7.1, 7.2, 7.3_

## Implementation Notes

- **5.1（源断ちは `guarded_set_window_pos` を飛ばすだけでは完成しない）**: `WM_DPICHANGED` に対して `None` を返して `DefWindowProcW` へ委譲すると、**`DefWindowProcW` 内から `SetWindowPos` が呼ばれて提案矩形が適用される**（本リポジトリ自身が `crates/wintf/src/ecs/window/components.rs:29-31` に明記）。ゆえに「書かない」判定でも **`Some(LRESULT(0))` を返すのが最後の防壁**である。初版はこれを実装していたが**檻が無く**、レビュアの変異（`None` を返す）で全緑になった＝**源断ちの最外殻が無検査**だった。是正版は `DpiChangedOutcome` へ `handler_result: Option<isize>` を追加して戻り値を観測し、両極性（`ExternalAuthority`＝ハンドル済み／既定政策＝ハンドル済み）を別々の檻で固定している。**wndproc の戻り値が意味を持つ是正では、戻り値そのものを檻に入れること。**
- **5.1（檻の空虚性・5 例目＝「load-bearing と自称した分岐が無検査」）**: `policy` の値語彙 4 種（`unset`／`ApplyPosition`／`ExternalAuthority`／`unreachable`）のうち、初版は **2 種が完全に空虚**だった。レビュアが `ApplyPosition`→`"unreachable"`・到達不能→`"unset"` と**取り違える**変異を当てても 555 passed の全緑。とりわけ `unreachable`／`unset` の区別は、実装者自身がコメントと診断レポートで「**Req 1.5 の趣旨**——読めなかったことを『宣言が無かった』と同じ語で報告すると事後の突合で偽の結論を作る」と **load-bearing だと宣言した当の分岐**である。**自分で「これは重要だ」と書いた分岐こそ、その主張を裏切る変異で検査すること。** 是正版は到達不能を **2 経路**（破棄済み entity／World 借用の再入）とも独立に固定し、レビュアが片方だけ潰す変異で**別々の assert 行**が赤くなることを実測している。
- **5.1 → 7.2 への申し送り**: `[WM_WINDOWPOSCHANGED] DPI center correction skipped: BoxStyle not found` の literal を固定する in-source 檻が**リポジトリ全体で 0 件**（`grep -rn "BoxStyle not found" crates --include=*.rs` の一致は `dpi_helpers.rs:110` の定義のみ）。5.1 がこれを `diagnosis-procedure.md` §6.5 の**判定語へ昇格させた**ので、書式が変われば手順書が静かに嘘になる。同書のメンテ規約が謳う「出力書式は in-source 檻がリテラル固定している」の体裁から外れているため、**7.2 で当該 literal を固定する檻を足すこと**。
- **5.1（`BoxStyle not found` は触らないのが正解）**: `correct_position_for_dpi_center_preserve`（`dpi_helpers.rs:96`）は冒頭 `:104` の `let Some(_ctx) = dpi_context else { return client_pos; }` で打ち切るため、`BoxStyle` 判定（`:110`）へ到達するのは**書込コンテキストが立つ窓だけ**。`DpiChangeContext::set` の本番呼出は `window_pos.rs:411`（`if let` の中）**ただ 1 箇所**（レビュアが grep で独立確認）ゆえ、全ゴースト窓が `ExternalAuthority` になった 5.1 以後は当該経路に一切手を入れずに warn が 0 件になる。**同 warn は非ゴースト窓で `BoxStyle` が欠ける真の異常を指す観測点として意味を保つので削除しない。**
- **5.1 → 7.4 への必須申し送り**: 実機判定は `diagnosis-procedure.md` **§6.5（`S1-SOURCE-CUT`）**に一本化した。合否判定語は `S1-SOURCE-CUT: PASS (external=N/N, boxstyle_warn=0, x_divergence=0)`。**⑴肯定側（`applied=false` が受理と同数）と⑷否定側（`BoxStyle not found` 0 件）は必ず対で読む**——⑷単独の 0 件は「そもそも DPI 受理が起きていない」と区別が付かない。**前提として当該セッションが `SESSION-QUOTA: PASS` であること**（`N = 0` は `PASS` ではなく `N/A`）。`policy=unreachable` が 1 行でも出たらその受理は計数から除外して原因を先に潰す。
- **5.1（`route=SpawnInitial`／`Restore` は本 spec では出ないまま＝開発者の追認事項）**: 5.1 は spawn へ `PlacementRoute` を配線しなかった。理由は構造的で、**spawn は単一ライター `enqueue_window_set_pos` を通らない**（entity 組立時に `WindowPos` を直接持たせる）ため配線先が存在しない。D13 帰結⑷「語彙のみ保持・将来の配線先として予約」に合致するので `diag.rs` の `#[allow(dead_code)]` も存置し、手順書 §3.3 は当該 2 語を「**未定**——本 spec では出ないまま。0 行を配線漏れと読まないこと」へ付け替えた（担当の付け替えではなく**明示的な先送り**）。
- **5.1 → 5.2 への申し送り**: S1 是正により `WindowPos.position` が OS 由来座標で汚染されなくなった＝**design.md:490 の「`None` 経路の再射影は正常系ではべき等」の前提が実際に成立する状態**になった。4.4 が置いた `s2_dpi_phase_writes_nothing_when_the_ground_point_already_holds`（Req 4.5 の書込ゼロ）はこの前提の上で読むこと。`grep '#\[ignore' crates/areka/src` の**実属性**は `s2_red_` 4 件のみ（コメント行 3 件が誤爆するので `-n` で内訳を見ること）。
- **4.6（檻の空虚性・4 例目だが型が新しい）**: 過去 3 例（2.2／3.2／4.4）は「檻が主張を確かめていない」型だったが、本件は**「doc が約束した契約を実装が満たしておらず、檻が doc ではなく実装をなぞっていた」**型。新設 `window_center` の doc は「位置または寸が未確定なら `None`」と約束していたのに、判定していたのは `Option::None` だけで wintf 正典の未確定表現 `CW_USEDEFAULT`（`== i32::MIN`）を素通しし、**本番経路に整数桁溢れ panic を新設**していた（レビュアが一時プローブで実測再現）。**純関数の doc に「未確定・無効・既定」と書いたら、その語がその crate で何を指すかを既存実装から確認すること**——`Option` だけが未確定とは限らない。同 crate に先例が 3 箇所（`graphics/systems/window_pos.rs:41`／`layout/systems/window_pos_systems.rs:147`／`window_pos/mod.rs:378`）ありながら新設コードだけが規約を外していた。
- **4.6（dev で緑・release で緑は同義ではない）**: `CW_USEDEFAULT` ガード撤去の変異は **dev では panic**（`attempt to add with overflow`）、**release では wrap して「たまたま全モニタ矩形外へ落ち黙って skip」**——機序が違う。是正版の檻は「打ち切りログが出ていること」を assert しているため両プロファイルで赤化する（レビュアが `cargo test -p wintf --lib --release` で独立実測）。`Cargo.toml:95-103` に `overflow-checks` の上書きが無いことも確認済み。**桁溢れを含む檻は release でも走らせて確認すること**。
- **4.6 → wintf 側ログ檻を書く全タスクへの必須申し送り**: bevy の既定 `Schedule` は多スレッド実行器でシステムを別スレッドへ流すため、`capture_under_filter`（スレッドローカル dispatcher 差替）が**1 行も捕捉できず、否定 assert が空出力で自明に緑になる**。`Schedule` を回すログ檻では **`ExecutorKind::SingleThreaded` を明示**すること。`crates/wintf/src/ecs/test_support.rs` の module doc には無い落とし穴で、4.6 で実際に踏みかけた。
- **4.6 → 4.7 への判定語の申し送り**（`diagnosis-procedure.md` 側の管轄・レポートのメンテ規約 3）: 新設の実機 grep 語は 3 つ。⑴`[detect_display_change_system] Updating Monitor entity`（**同行に `old_dpi=`／`new_dpi=`／`old_work_area=`／`new_work_area=`／`old_bounds=`／`new_bounds=`／`old_primary=`／`new_primary=` が載る**＝何が変わったかを実機ログだけで復元できる）⑵`Redriving window DPI from updated Monitor`（同行に `entity=`／`handle=`／`center=`／`old_dpi_x=`／`new_dpi_x=`）⑶`[detect_display_change_system] Display configuration change applied` の **`windows_redriven=N`**（「モニタ表は更新されたが窓が 1 つも駆動されなかった」を 1 行で切り分けられる）。①〜③はいずれも target が `wintf::ecs::layout::systems::monitor_systems`＝**D-BASE に既収録ゆえ追加語不要**。加えて ⑷`[SetProcessDpiAwarenessContext] DPI awareness set`（成功・`info!`）／`... failed`（失敗・`warn!`）は target `wintf::runtime`＝D-BASE の大域 `info` に載る。**②の再採取では必ず ⑷ を先に確認すること**——`WM_DPICHANGED` 0 件の機序（per-monitor v2 が実際に効いているか）の第一次切り分けがここで初めて可能になる。
- **4.6 → 4.7 の判定基準に関する未解決事項（開発者の裁定が要る）**: `SESSION-QUOTA` は `diagnosis-procedure.md` §5 で **`WM_DPICHANGED` の受理回数**として定義されているが、4.6 の是正は**`WM_DPICHANGED` 非依存の駆動路**を通すものである。したがって `WM_DPICHANGED` が 0 件のままでも追従は成立し得て、その場合 4.7 で `SESSION-QUOTA` が永久に踏破できず、4.7 のタスク文「踏破しなければ 4.6 へ差し戻す」と噛み合わない。**判定基準を「モニタ表更新（⑴）＋DPI 再導出（⑵⑶）の 2 判定語」へ拡張する必要がある**。4.7 の採取前に手順書 §5 を改訂すること。
- **4.6（`WM_DPICHANGED` 0 件の機序は未確定のまま）**: 本タスクが成立させたのは「それに依存しない駆動路」であって、なぜ OS が `WM_DPICHANGED` を送らないかの解明ではない。⑷のログが実機で `set`（成功）を示すなら per-monitor v2 は効いており、別の機序（WUC/`WS_EX_NOREDIRECTIONBITMAP` 窓の特性等）を疑うことになる。**4.7 で ⑷ を確認するまで推測で結論を書かないこと**（Req 1.5）。
- **4.5 セッション①（2026-07-31）→ 5.1／5.2／6.1／7.3 への必須申し送り**: 実機で**S1 が 84/84 の陽性**（`applied=true` のみ・`false` ゼロ）、**S2 が 45 件中 1 件の陽性**（接地点 −91px）。消失（`VANISH-TRACE`）は NONE だが、**その理由は「キャラ窓の DPI 受理 44 回すべてで `DpiReproject` の上書きが後続したから」**であって欠陥が無いからではない（route 内訳が 44:44 で一致）。**「再現しなかった」を 5.1／5.2 の縮小根拠に使ってはならない**（§0.2）。
- **4.5 セッション①（S1/S2 の出力上の指紋）**: 二重ライターの差分は **Y が定数（±91 / ±273）・X が可変（−861〜+861）**という非対称を示す。Y の定数性＝S2（射影量が固定量として欠落）、X の可変性＝S1（OS 提示値の素通し）。**5.1 是正後は X 差が消え、5.2 是正後は §2.5.1 の `ground_y=1704 dpi=144` の外れ値が消える**——7.4 の再サインオフはこの 2 点を判定語にできる。
- **4.5 セッション①（D13 の実機的裏づけ）**: `ReportedSizeReconcile` は起動直後 **1 件のみ**、`DpiReproject` は **44 件＝キャラ窓 DPI 受理と 1:1**。1.4 是正前の実装（両者に `DpiReproject`）なら起動時の 1 件が偽の DPI レコードとして混入し、この 44:44 の一致が崩れて突合そのものが成立しなかった。**語彙完全化が診断の前提条件だったことが実測で確認された**。
- **4.5 セッション①（4.1 手順書の 2 語が実機で効いた）**: `wintf::ecs::layout::systems::monitor_systems=debug` が無ければ残余 B（両側モニタ列挙突合）が成立せず、`wintf::ecs::drag=`**`trace`** が無ければ Req 2.3 の比（4,156 件）が測れなかった。**design.md:476 の例のままなら両方とも欠落していた**。
- **4.5 セッション①（本 spec 範囲外の発見 2 件）**: ⑴目視のバルーン消失は**幾何ではなく Z オーダー**が原因（4,098 件すべて work area と交差）→ `areka-P0-ghost-window-zorder` へ切り出し済み。⑵`[WM_WINDOWPOSCHANGED] DPI center correction skipped: BoxStyle not found` が **84 件＝DPI 受理と同数**＝全 DPI 変化で center correction が不発。**5.1 で当該経路を触るため、実装時に到達性を再確認すること**（不要になるなら削除、必要なら `BoxStyle` 前提の是正）。
- **1.4 レビュー #1（REJECTED・2026-07-31）と D13 裁定**: 独立レビューが 3 欠陥を確定——①`reconcile_window_size`（frame.rs:690）の 2 呼出元（dpi 相 frame.rs:841／drain 相 frame.rs:1028）の両方へ `DpiReproject` を貼り、**DPI 変化ゼロの起動直後にも偽の DPI レコードが毎回出る**（drain 相は初回表示 k₀ 補正を含み `Changed<DPI>` 非依存＝frame.rs:983 doc が明言）②frame.rs 側 route 割当に檻ゼロ（「境界制約で書けない」は事実誤認＝`capture_logs` は `pub(crate)` で到達可能）③`\![move]`（move_cue.rs:619→`move_window_to`）の対象窓書込が無記録。**開発者裁定**:「分かるようにログを出せばいい。あとで識別できることが重要。方法は任せる」→ 語彙を 9 種へ完全化（design.md D13 が正本・3 案の解決/未解決対比と帰結もそこに登記）。requirements.md は無改変（Req 1.2/2.4 は経路を一般語で要求＝要件 gap ではない）。**是正済み・レビュー #2 で APPROVED**——レビュアが独立ミューテーションで 4 変異→意図した 4 檻が 1:1 で赤になることを実証（`boot_without_any_dpi_change_emits_no_dpi_reproject_record` は先に「k₀ 補正の書込自体は起きている」を固定してから `DpiReproject` 不在を主張＝非空虚）。
- **D13 → 4.1（手順書）への申し送り**: grep 判定語の route 語彙は 9 種（`ReportedSizeReconcile`・`MoveCue` を含む）。セッション②の突合では「DPI 由来の書込＝`DpiReproject` のみ」であり `ReportedSizeReconcile` を数えないこと。`\![move]` は `MoveCue`、ドラッグは wintf `[drag]`（diag target 外）が担う——両 target の対応表を必ず載せること。
- **D13 → 6.1 への申し送り**: 遷移ガードの発火 route 集合は `AnchorChange`/`Resnap`/`DpiReproject`/`ReportedSizeReconcile` の 4 種。`MoveCue` は適用外（スクリプト明示操作の尊重）。
- **1.4**: route の識別は「共有末端に route を**引数**で渡す」で担保する（`reconcile_window_size(…, route)` → `resize_window_to(…, route)` → `enqueue_window_set_pos(…, Some(route))`）。共有末端の内部で route を再導出・既定値埋めすると呼出元の区別が消える——それが D13 の欠陥①そのもの。**共有末端に route を書き込む設計を今後も禁じること**。
- **1.4**: 経路割当の檻は**分岐の下流ではなく割当点のあるファイルに置く**。レビュー #1 で「境界制約で frame.rs に檻が書けない」は事実誤認と確定（`crate::placement::test_support::capture_logs` は `pub(crate)`）。以後 route を新設・変更する際は割当点ファイルに「別経路が別名で記録される」檻を必ず添えること。
- **1.4**: 同一文言の誤りが**複数ファイルに複製されている**ことがある（レビュー #1 が起票した `frame.rs` の誤コメントと同文が `follow.rs` の `route` 引数 doc にも存在し、レビュー #2 で検出）。doc 是正時は文言を**ツリー全体 grep** で掃くこと。
- **1.1／1.4**: `placement/diag.rs` の `#![allow(dead_code)]`（モジュール全体）は 1.4 の配線完了により**撤去済み**。残る真の dead は `PlacementRoute::SpawnInitial`／`Restore`（D13 帰結⑷＝語彙のみ予約・未配線）と `ALL`（檻専用）の 2 箇所だけで、それぞれ実在理由を書いた狭い `#[allow(dead_code)]` に置き換えた。**5.1 で spawn へ配線したら `SpawnInitial`／`Restore` の属性を外すこと**。
- **1.1**: `WindowMoveRecord.size`／`.dpi` は `Option`（`SWP_NOSIZE` 経路・`DPI` component 未付与に対応）。値なしは `-` sentinel で**フィールド自体は落とさない**＝経路によらず grep 語が不変。**1.4 の配線では寸を伴う経路で必ず実寸を詰めること**（`None` は move-only 呼出に限る）。手順書（4.1）の判定語表には `w=-`／`dpi=-` の意味と、`w=-` が `w=-12` の接頭辞である旨（トークン境界でアンカーする）を記載すること。
- **1.1**: 檻の実効性は「format 変異（`dpi=`→`DPI=`）で 2 件赤・水準変異（`debug!`→`info!`）で 3 件赤」でレビュアが独立再現済み。`EnvFilter` 実濾過による Req 1.7 の静穏檻は非空虚。
- **1.3 → 4.1 への必須申し送り**: 窓位置書込の共通経路 `guarded_set_window_pos` の target は `wintf::ecs::window::command` であり、design.md:476 が記す `RUST_LOG=…,wintf::ecs::window_proc=debug,…` では **EnvFilter の文字列前方一致に掛からず点灯しない**（ミューテーションで実測確認済み）。**手順書には `wintf::ecs::window=debug` を入れること**（`window_proc` も包含する）。入れ忘れると Req 1.5 が排除しようとしている偽陰性を再生産する。
- **1.3 → 5.1 への申し送り**: `window_pos.rs` の実施可否行の `applied` は Phase A では `let applied = true;` の定数（分岐は 5.1 の所有）。design.md:319 が挙げる `policy` フィールドも未出力＝`DpiSuggestedRectPolicy` を新設する 2.1 の後、5.1 で `applied` の分岐化と同時に追加すること。
- **2.1 → 5.1 への申し送り**: `dpi_suggested_position_decision`（`crates/wintf/src/ecs/window_proc/dpi_helpers.rs`・`pub(super) fn(Option<&DpiSuggestedRectPolicy>, &RECT) -> Option<(i32, i32)>`）の戻り値 `Option` は **`DpiChangeContext::set` と `guarded_set_window_pos` の双方を 1 個の `if let Some((x, y))` で束ねて分岐させる**ためのもの（D3 帰結）。**シグネチャを広げる必要はない**——レビュアが design.md:319 と突合して確認済み。配線時に同関数の `#[allow(dead_code)]`（dpi_helpers.rs:31）を外すこと。
- **2.1**: `DpiSuggestedRectPolicy` は `crates/wintf/src/ecs/mod.rs` の curated `pub use window::{…}` へ載せてある（areka は `wintf::ecs::{…}` 経由でしか import しない＝`placement/spawn.rs:63` の流儀）。5.1 の areka 側付与は wintf を編集せずに書ける。
- **4.4 → 5.2／7.1 への必須申し送り**: S2 の赤証跡 4 件は `crates/areka/src/emo2_boot/frame.rs:2911-3368` に `#[ignore = "…再現: cargo test -p areka -- --ignored s2_red_"]` で置いてある（`s2_red_ground_point_preserved_at_dpi96`／`_from_dpi96_to_dpi120`／`_from_dpi96_to_dpi192`／`_from_dpi120_to_dpi192`）。**5.2 で 4 件とも外すこと**。areka crate 内の `#[ignore]` はこの 4 件だけなので `grep -n '#\[ignore' crates/areka/src` がゼロになれば完了。常時走る随伴 2 件も置いてある——⑴`Some` 経路の非退行（新寸・X 保存・Y 再射影・バルーン offset 恒等式・route は `DpiReproject` のまま）⑵**Req 4.5 の書込ゼロ**（接地点が既に成立していれば書かない）。レビュアが「無条件に書く 5.2」「`Some` 経路を据置きへ流す 5.2」の両変異で⑵⑴がそれぞれ赤化することを実測済み。
- **4.4（赤の読み方）**: 採取した赤は接地点が**変化前後でバイト同一**（`ground: (1700, 1444)` → `(1700, 1444)`）で `wa_bottom` だけが 1444→1432/1396 と動く＝**一切書かれていない**。しかも失敗するのは **Y のみ**（X は 1700 のまま）で、S1（X 汚染）と出力そのもので区別が付く。合成レイアウトは work area 下端を **DPI の関数**（`1492 − 48·dpi/96`＝タスクバーの物理成長）とし、「下端が実際に動くこと」を檻自身が self-check する（レビュアが下端を DPI 非依存に変異させると self-check が先に発火することを確認）。
- **4.3 → 5.1／7.1 への必須申し送り（最も落としやすい）**: S1 の赤証跡 4 件は `crates/wintf/src/ecs/window_proc/window_pos.rs` に `#[ignore = "…再現: cargo test -p wintf -- --ignored s1_red_"]` で置いてある（`s1_red_external_authority_establishes_no_write_context`／`..._preserves_anchor_at_dpi96`／`_dpi120`／`_dpi192`）。**5.1 で 4 件とも `#[ignore]` を外すこと**（dpi96 も）。外さないと回帰檻として機能せず、5.1／7.1 の完了状態が形式的にだけ満たされる。5.1／7.1 のタスク文にも明記済み。
- **4.3（赤証跡の作り方）**: 赤は**常時走る檻として置けない**（永久に赤いスイートは以後全タスクの検証ゲートを壊す）。`#[ignore]`＋再現コマンドの inline 明記＋レポートへの実出力転記、が本 spec の流儀。加えて**常時走る随伴 2 件**を置いてある——⑴非退行の対照（政策なし窓は従来どおり提案位置を採用）⑵**D3 の対分岐不変条件**（`DpiChangeContext::set` と `guarded_set_window_pos` が必ず一緒に分岐する）。⑵は是正前は恒真で緑だが、**5.1 が片側だけ分岐させると赤になる**（レビュアが split fix を当てて dpi=96 で赤化することを実証）。位置檻が盲目な 96 でも効く点が価値。
- **4.2 の正本＝`diagnosis-report.md`（確定台帳）。4.3／4.4／4.5／7.1／7.4 はここへ**追記**する**——節骨格は 4.2 が作成済み（§3.1＝S1 専用・§3.2＝S2 専用で非重複・§2＝4.5 の Q1〜Q4・§4＝7.4）。他タスクの節を上書きしないこと。S1〜S3′ の 4 件は現ツリーで**現存を機械確認済み**（2.1／2.2 の是正機構はいずれも本番呼出ゼロ＝純関数が在ることは AC の充足ではない）。レビュアが 54 箇所の `file:line` を全数再測定し**誤り 0**。
- **4.2 の載せた最重要規則（4.5 で誤読しないこと）**: 実機 2 セッションが受理回数を踏破して消失痕跡ゼロでも、除外できるのは**実機でしか確定できない残余仮説への追加修正のみ**。**静的確定分（S1〜S3′）の是正 5.1／5.2／6.1／6.2 と檻 7.1／7.2 は除外されない**（Req 2.6／2.9／5.1／5.4）。
- **4.2（証跡の書き方）**: 「本番呼出ゼロ」の根拠に `#[allow(dead_code)]` の有無を使わないこと——同属性は本番呼出のある `follow.rs:148` `project_anchor`・`:822` `resize_window_to` にも付いている。根拠は `crates/` 全 grep で一致がすべて定義行・doc・`#[cfg(test)]` 内であること（テストモジュール開始＝`follow.rs:1587`／`dpi_helpers.rs:150`）。また「呼出点がある」と「本番スケジュールに載っている」は別物（`anchor_changed_system` は `add_systems` が全て `#[cfg(test)]` 内＝定義済み未登録）。
- **4.1 の正本＝`diagnosis-procedure.md`（4.5／7.4 はこれに従って採取する）**。確定した `RUST_LOG`（D-BASE・レビュアが `capture_under_filter` で実濾過を独立実証）:
  `info,areka::placement::diag=debug,wintf::ecs::window=debug,wintf::ecs::layout::systems::monitor_systems=debug,wintf::ecs::drag=trace`
  handoff の 1 語に加え **2 語が実測で必要と判明**——⑴`wintf::ecs::layout::systems::monitor_systems=debug`（wintf 側モニタ列挙行は `wintf::ecs::window` 配下に**無い**＝D12 の両側 grep 突合が不成立になる）⑵`wintf::ecs::drag=`**`trace`**（毎移動の `[DragEvent] Dispatching` は `trace!`＝`debug` では暗転し Req 2.3 のマウス対窓の数値対応が測れない）。design.md:476 の例が点灯しないことも負の assert で実証済み。
- **4.1 → 5.1／6.1 への申し送り（design の判定語が Phase A で存在しない）**: design.md:484 の⑤合否判定語が挙げる `ClampX`／`NearestFallback` の warn は **6.1/6.2 で初めて配線**される（`guard_visibility` は無ログ）。Phase A/B のセッションでこれを判定語に使うと**確実に偽陰性**になるため、手順書は幾何突合（`[diag.monitor]` の work area × `[diag.window_move]` の矩形）へ置き換えてある。6.1/6.2 着地後に手順書 §6.3 を warn 判定語へ戻すか併記すること。
- **4.1（実機採取時に確認したい未検証点）**: `[guarded_set_window_pos]` の `x/y` は `SetWindowPos` へ渡す窓座標、`[diag.window_move]` の `x/y` は `WindowPos`（クライアント座標のミラー）。`enqueue_window_set_pos` は同一値を双方へ入れており枠なしゴースト窓では一致するはずだが**実機未確認**。4.5 で系統的ずれを観測したらレポートへ事実として記録すること。
- **4.1**: 2 段 grep の結合キーは **`entity=`**（`{index}v{generation}` 形式）。`hwnd` は target ごとに書式が違う（`command.rs`／drag は `"0x{:X}"` の引用符付き大文字・`window_pos.rs` は Debug の `HWND(0x…)` 小文字）ため、`hwnd` で突合すると**静かにゼロ件**になる。
- **3.2 → 5.2／7.3 への申し送り**: 消費側 4 入口（`follow.rs` の `resize_window_to`／`resize_window_keep_position`・`frame.rs` の `resnap_with`／`reconcile_reported_sizes`）に `world.get_entity()` 存在確認を敷き済み。判定語は `crate::placement::diag::DESPAWNED_SKIP_TAG` の共有定数。**frame.rs へ新しい消費点を足すときは同じ区別を敷くこと**（entity 不在＝`debug!` で打ち切り他 scope 継続／実在するが規約 component 欠落＝`warn!`）——混ぜると終了時ログの良性ノイズが本物の異常を埋める。`resnap_from_sizes` は本番呼出元が `resnap_with` のみで下流 `resize_window_to` のガードが受けるため未防護（実測確認済み）。
- **3.2（檻の空虚性・2 例目）**: 完了状態檻の初版は**総数計数で空虚**だった——フレーム層のガードを外しても、下流の追従層が同じ判定語を同数出して総数が偶然一致する。是正版は**相ごとの計数**（`dpi reconcile:` 4 件・`resnap:` 2 件）へ強化し、さらに探針を**意図的に陳腐化したレジストリ**（破棄済み entity を指す写し）で組んでいる（3.1 の hook が掃除した綺麗なレジストリで回すと自明に緑＝空虚）。ヘルパ `despawn_scope_and_restore_stale_registry` が「hook が実際に落とした」「写しが破棄済みを指す」の両方を実行時 assert する。[[2.2 の教訓]] と同型。
- **3.2 → W5 `kero-balloon` への干渉申し送り（`.kiro/steering/roadmap.md` 干渉台帳 `van(W5)⇄ker(W5)`）**: `emo2_boot/frame.rs` は**同一ファイル・異ハンク**。van の編集面 4 点と、触っていない ker 編集面（`run_text_scale_phase`・`balloon_models` 写像・`dpi_phase_with` 本体）を明記済み。**先着後 rebase 必須**（git が自動マージしても行番号を引いた doc・檻コメントが静かに嘘になる）。van はタスク 5.2 で `dpi_phase_with` を判断分岐ごと改造予定＝ker が同関数へ触るなら着手前に相談。
- **3.1 → 3.2 への申し送り**: 掃除は `GhostWindowMarker` の `on_remove` hook（`crates/areka/src/placement/spawn.rs:117-138`）が駆動し、**Resource のみを触る**（`DeferredWorld` は `iter_entities` を露出すらしない＝Req 6.4 は構造的帰結）。`GhostWindows::remove_entry_of` は全域・冪等で、非登録・空・二重除去・Resource 不在のいずれも静かに `None`。本番の `remove_entry_of` 呼出は hook 1 箇所のみ＝呼出点結合なし。3.2 の消費側存在確認はこの前提の上に書くこと。
- **3.1 → W6 `balloon-visibility` への干渉申し送り（`.kiro/steering/roadmap.md:80` の干渉台帳が正本）**: ゴースト窓の despawn は**その scope のレジストリ登録を静かに消す**。`GhostWindows` の構築点は `spawn.rs:314`（`spawn_ghost_windows` 内）ただ 1 箇所ゆえ、**hide/show を despawn/respawn で実装すると登録が復活しない**——vis の hide は可視性トグルで実装すること。5.1 が同じ spawn バンドルを再度触る（`ExternalAuthority` 付与）点も台帳に記載済み。
- **3.1**: `placement/test_support.rs:165-183` の `capture_logs` は **EnvFilter 濾過ではなく全捕捉**（`enabled()` 常時 true）で、イベントが実際に出した metadata 水準を読む。水準 assert は非空虚（`debug!`→`warn!` 変異で赤）で design.md:524 の「既存 tracing テスト流儀」に合致。Req 1.7 の静穏檻（1.1）が使う `EnvFilter` 実濾過とは別物なので混同しないこと。
- **2.2 → 6.1／6.2 への申し送り**: `guard_visibility`（`crates/areka/src/placement/follow.rs:1397`）は**意図的に無ログ**——`ClampX`／`NearestFallback` の warn は route で分岐する（Req 3.3・D13 帰結⑴・design.md:432）ため純関数層では書けない。**配線側で `warn!` を必ず出すこと**（出さなければ Req 3.1/3.2 の観測が欠落する）。`clamp_wa` は引数ゆえシグネチャ拡張は不要だが、**射影が Y に用いたのと同じ work area を渡すこと**（別モニタの矩形を渡すと事後条件が崩れる・follow.rs:1373-1376 に明記）。配線時に `#[allow(dead_code)]` 3 箇所（`:1334`／`:1345`／`:1396`）を外すこと。
- **2.2（檻の空虚性の教訓・重要）**: 「不変を主張する檻」は**変異が不動点に落ちないか**を確かめること。初版の Y 不変檻は全探針の Y が `wa.bottom - h`（＝work area Y clamp の不動点）だったため、ガードが Y も clamp する変異を入れても 578 件全緑＝**空虚**だった（`y + 1` のような粗い変異だけ捕まえていた）。是正版は clamp 範囲**外**の Y を探針に加え、さらに「探針 Y が不動点でないこと」自体を `assert!` で自己検査し、分岐識別（`matches!(verdict, ClampX(_))`）も同時に固定している。以後 X/Y/寸の不変を主張する檻は同じ形で書くこと。
- **2.2**: 委譲等価性の檻（`follow.rs:2010` 付近）は委譲が成立する限り構造上失敗し得ない**同語反復**であり、将来の乖離ガードとしてのみ保持。「既存関数の戻り値が不変」の実証は既存 5 檻（`:1748`／`:1771`／`:1785`／`:1806`／`:1826`）が担う。等価性檻を証跡として数えないこと。
- **2.1**: dpi=96 の檻は**政策分岐に対して意図的に無差別**（`ExternalAuthority` 変異で緑のまま・座標変異で赤）。これが Req 5.1 の「96 が欠陥を隠す」性質そのもので、分岐網羅は専用 3 檻が担う。以後 96 の檻を「分岐を見ていない」と誤診して強化しないこと。
- **1.3**: wintf 側に `crates/wintf/src/ecs/test_support.rs`（`#[cfg(test)]` 限定・`capture_under_filter(directives, f)`）を新設済み。`tracing-subscriber` は dev-dependency のみで本番バイナリ非到達。以後の wintf 側ログ檻はこれを使うこと。
- **1.3**（既知の既存不良・本 spec 範囲外）: `cargo clippy -p wintf` は `com/d2d/command_sink.rs:107,128,155` の `not_unsafe_ptr_arg_deref` で失敗する（本 spec 以前から）。clippy を DoD に使わないこと。
- **1.2**: 実機出力の行書式は `[diag.monitor] index=N handle=… bounds=l,t,r,b work_area=l,t,r,b dpi=… primary=…` ／ヘッダは `[diag.monitor_snapshot] context=<tag> count=N`。呼出点タグは `monitor_snapshot`（main.rs 正典＝`MonitorSnapshot` 構築点）と `prepare_ghost_windows`（placement 列挙点）の 2 種。**タスク 1.3 の wintf 側列挙ログのフィールド名はこの書式に合わせること**（D12 の「共有語彙 grep 突合」が成立する条件）。手順書（4.1）の判定語もこれ。
- **1.2**: `prepare_ghost_windows` の snapshot 出力は `enumerate_monitors()` 直後・`prepare_stages` より**手前**に置いてある（準備が失敗してダミー窓へ縮退した走行でもモニタ構成がログに残るため）。位置を後ろへ動かさないこと。
- **1.2**: `MONITOR_SNAPSHOT_CONTEXT` の `#[allow(dead_code)]` は実在の理由あり——`examples/window-placement.rs:107`・`collision-probe.rs:138` が `#[path]` で `src/placement/mod.rs` を include し、example ビルドでのみ dead になる（`--force-warn dead_code` で再現確認済み）。
- **1.1**: `placement/test_support.rs` の `ensure_interest_probes` を `pub(crate)` へ昇格（`mod test_support;` は `#[cfg(test)]` 限定ゆえ本番非影響）。tracing の callsite interest キャッシュ毒化回避に必要。
