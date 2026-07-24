# Implementation Plan

- [x] 1. バルーン対話サブモジュールの雛形作成と runtime アクセサ追加
  - `crates/areka/src/input_events` 配下にバルーン対話専用のサブモジュールを新設し、モジュール宣言を追加する
  - `Emo2Wiring` に既存 `presenter()` アクセサと同型の runtime 読み口を additive に追加する
  - Observable: 新設サブモジュールと新アクセサを含めてクレートがビルド成功する
  - _Requirements: 4.1_
  - _Boundary: input_events::balloon (skeleton), Emo2Wiring_

- [ ] 2. Core: 契約型と状態配線資源
- [x] 2.1 ChoiceSelection 契約型の定義
  - id／label／scope／references の 4 フィールドを持つ選択確定ワイヤ形を定義する（ordinal は含めない）
  - Clone／Debug／PartialEq／Eq を導出し、下流が型を import して消費できる形にする
  - Observable: 単体テストで 2 つの ChoiceSelection 値を構築し、等価性比較が意図通り成立する
  - _Requirements: 2.2, 2.6, 8.5_
  - _Boundary: ChoiceSelection_

- [x] 2.2 BalloonWiring 資源と ChoiceSelectionInbox シームの定義
  - scope→最後に注入した hover ordinal の自前追跡マップと、選択確定の発行シンク（mpsc Sender）を持つ NonSend 資源を定義する
  - Receiver を保持する M1 暫定受け口（下流 choice-select-events が受信処理へ置換するシーム）を定義し、`CuePlayer::resolve_choice` を直接呼び出さない境界を保つ
  - Observable: 単体テストで資源を World へ NonSend 挿入でき、シームの Receiver 経由で送信値を観測できる
  - _Requirements: 2.1, 2.4, 3.4, 5.3, 5.4, 8.2, 8.3_
  - _Boundary: BalloonWiring, ChoiceSelectionInbox_
  - _Depends: 2.1_

- [ ] 3. Core: 純関数判定核
- [x] 3.1 点包含 hit 判定と重なり規則の実装
  - 行矩形群への点包含判定を純関数として実装する（半開区間・物理 px 直接比較・DPI 変換なし）。判定対象は上流が供給する選択肢行ジオメトリのみとし、選択肢以外のバルーン内リンクは対象にしない
  - 病的重なり入力に対し決定的な単一選択規則（逆順走査・最終一致）を適用する
  - Observable: 単体テストで内側／境界／外側／空配列／重なりの各ケースが実窓・GPU 不要で決定的に判定される
  - _Requirements: 1.1, 1.5, 2.3, 4.2, 5.2, 8.6_
  - _Boundary: hit_choice_row_

- [x] 3.2 hover 遷移判定関数の実装
  - 表示中フラグ・hit 結果・前回注入値から hover 遷移（無変化／自前状態のみ整合／注入）を決定する純関数を実装する
  - Observable: 単体テストで全分岐（非表示時無処理／消滅時自前整合／同値維持／新規注入／解除注入）を網羅する
  - _Requirements: 1.2, 1.3, 1.4, 3.4_
  - _Boundary: hover_action_

- [x] 3.3 クリック確定判定関数の実装（stale／原子性）
  - クリック時点の現行行ジオメトリのみから ChoiceSelection を構成する純関数を実装し、非表示中・非 hit・キャッシュのみのケースを None とする
  - Observable: 単体テストでヒット時の ChoiceSelection フィールド一致、非 hit／非表示／stale 行での非構成（None）を確認する
  - _Requirements: 2.1, 2.2, 2.3, 3.1, 3.2, 6.2, 6.3_
  - _Boundary: click_selection_
  - _Depends: 2.1, 3.1_

- [ ] 4. Core: バルーンポインタハンドラ
- [ ] 4.1 移動ハンドラの実装（hover 追従駆動）
  - Bubble 相のみ処理する移動ハンドラを実装し、固定順の借用規律（共有借用→スナップショット→借用解放→可変借用で inject）に従う
  - `Emo2Wiring` 不在（boot 前／失敗）は正常縮退として `debug!` ＋ no-op（donor `presenter=None` 同型）、`BalloonWiring` 不在または `RefCell` 借用失敗は構成異常として `error!`（event = balloon_wiring_missing／balloon_runtime_borrow_failed）でログし no-op 縮退する（ログ無し失敗経路を作らない）
  - hover 遷移注入時は `debug!`（event = choice_hover_inject）を発行する
  - Observable: 合成 PointerState の直接呼び出しテストで hover 追従と各縮退経路（`debug!`／`error!` それぞれ正しい方）の双方が観測できる
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.6, 3.1, 3.3, 4.1, 4.2, 8.4_
  - _Boundary: on_balloon_pointer_moved_
  - _Depends: 2.2, 3.2_

- [ ] 4.2 押下ハンドラの実装（確定クリック→発行）
  - 左シングルクリックのみを確定として扱い（`double_click` フィールドは不参照）、ヒット時に一度だけ ChoiceSelection を送信するハンドラを実装する
  - 送信成功時は `info!`（event = choice_selected, scope, id, label, references_len）を 1 回発行する（実機サインオフの grep 対象）
  - 非表示中／非 hit での棄却は `debug!`（event = choice_click_rejected, reason）。`Emo2Wiring` 不在は `debug!` ＋ no-op（正常縮退）、`BalloonWiring` 不在／`RefCell` 借用失敗／`selection_tx.send` 失敗は構成異常として `error!`（event = balloon_wiring_missing／balloon_runtime_borrow_failed／choice_selection_send_failed）でログし no-op 縮退する
  - Observable: 合成 PointerState の直接呼び出しテストでヒット＆表示中クリックにつき一度だけの送信と対応する `choice_selected` ログ発火、非ヒット／非表示／Tunnel 相では送信ゼロと対応する棄却／縮退ログを観測する
  - _Requirements: 2.1, 2.3, 2.4, 2.5, 2.6, 3.1, 3.2, 4.2, 5.1, 8.4_
  - _Boundary: on_balloon_pointer_pressed_
  - _Depends: 2.1, 2.2, 3.3_

- [ ] 5. clear_balloon_hover_on_leave の実装
  - 窓外離脱マーカー（`PointerLeave`）を読み、親チェーンがバルーン窓を所有する entity のみを対象に、既存 `hover_action(active, None, last)` を再利用して hover 状態を解除する排他システムを実装する
  - Observable: bare World テストでバルーン所有 leave のみ hover 解除され、非バルーン窓の leave は無視されることを確認する
  - _Requirements: 1.3, 3.4_
  - _Boundary: clear_balloon_hover_on_leave_
  - _Depends: 2.2, 3.2_

- [ ] 6. Integration: 配線結合
- [ ] 6.1 post-spawn ハンドラ装着・資源結線・スケジュール登録
  - 全バルーン窓へポインタハンドラ（moved／pressed）を post-spawn 装着し、NonSend 資源とチャネルを結線し、leave 追随システムを Input スケジュール（dispatch 後）へ登録する
  - Observable: bare World 統合テストで、全バルーン窓にハンドラが存在しキャラ窓のハンドラ集合は不変であること、かつスケジュールへの leave システム登録存在を assert で確認する
  - _Requirements: 4.3, 4.4, 5.5, 6.6_
  - _Boundary: attach_balloon_pointer_handlers, wire_balloon_choice_
  - _Depends: 4.1, 4.2, 5_

- [ ] 6.2 main.rs 起動結線
  - 起動シーケンス（既存 `attach_char_pointer_handlers` 呼出の隣）へバルーン配線の結線呼び出しを追加する
  - Observable: クレートがビルド成功し、起動シーケンスが新規結線呼び出しを実行する
  - _Requirements: 4.3, 5.5, 8.1_
  - _Depends: 6.1_

- [ ] 7. Validation: 決定論檻・退行防止・実機サインオフ
- [ ] 7.1 貫通ポインタ列の決定論檻
  - 選択肢行 population がテストから決定論的に可能か確認したうえで、可能なら合成ポインタ列（移動→クリック）から `ChoiceSelectionInbox` 観測までの一気通貫テストを追加する。GPU 依存で不可能な場合のみ、純関数全網羅＋mpsc 観測の分解形で本要件を満たすことを明記する
  - Observable: 注入座標列に対する hover 追従と一度きりの発行、stale／非 hit での非発行が実窓・sleep 不要で観測される
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_
  - _Depends: 6.1_

- [ ] 7.2 資源縮退・Tunnel 素通し回帰檻
  - `Emo2Wiring`／`BalloonWiring` 不在時の no-op 縮退と、`Phase::Tunnel` での両ハンドラの素通し（副作用ゼロ・`false` 返却）をテストする
  - Observable: 縮退時の対応するログ（`debug!`／`error!`）と副作用ゼロ、Tunnel 相での副作用ゼロが確認される
  - _Requirements: 8.1, 8.4_
  - _Depends: 4.1, 4.2_

- [ ] 7.3 実機サインオフ手順の実施
  - 実 emo2・実 pasta.dll・絶対パス・実 DPI（≠96）で起動し、本番ゴースト表示を先行させたうえでポインタ追従を目視し、クリック確定を有界 `AREKA_APP_SMOKE_EXIT_MS` auto-exit 後の `RUST_LOG` ログ grep（`event="choice_selected"`）で確認する
  - カスケード発火・遷移（下流 choice-select-events の領分）は本判定に混ぜない
  - Observable: 実機でポインタ追従ハイライトが目視でき、`choice_selected` ログの grep 一致でクリック確定到達が確認される
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_
  - _Depends: 6.2_

- [ ] 7.4 ワークスペース全体回帰確認
  - i686 host-32 成果物を事前ビルドしたうえで `cargo test --workspace` を実行し、新規テストを含め exit 0 で成功することを確認する。新規外部依存が追加されていないこと、上流契約・cue ワイヤ形・キャラ窓 DPI 素通し規約が変更されていないことを確認する
  - Observable: 全既存テスト＋新規テストが緑で exit 0
  - _Requirements: 8.1, 8.2, 8.3, 8.5, 8.6_
  - _Depends: 7.1, 7.2, 7.3_

## Implementation Notes

- 環境: worktree の `vendors/pasta` submodule が未populate だと areka ビルドが失敗する。`git submodule update --init --recursive vendors/pasta` で解消（既知の worktree quirk・source 変更ではない）。
- 検証コマンド: 各タスクは `cargo build -p areka` ＋ `cargo test -p areka`（bin の in-source `#[cfg(test)]` を既定で走らせる）。バルーン限定は `cargo test -p areka --bin areka input_events::balloon`。最終回帰(7.4)のみ `cargo test --workspace`（i686 host-32 成果物の事前ビルド前提）。
- 上流型: `ChoiceHitRow`（`areka_emo_text::actor`・実体 `crates/areka-emo-text/src/actor.rs:150`）・`HitRectPx`（`crates/areka-emo-text/src/choice.rs:155`・`left/top/right/bottom` は `pub` f32）。areka から実型で fixture 構築可能——ローカル並行型を作らないこと（4.1/8.5）。
- **レビュアー厳守**: RED 再現やスタブ確認で `git checkout`/`git reset --hard` を絶対に使わない（未コミット実装を破棄する既知ハザード）。差替え検証はファイルバックアップ（cp）→復元で行い、復元後に diff 存在とテスト緑を必ず再確認する。
- `#[allow(dead_code)]` は各シンボルへ narrow 付与（本番未消費のうちだけ・後続タスクの本番結線で撤去見込み）。crate-wide 抑止は禁止。
