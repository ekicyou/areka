# 設計検証レポート: areka-P0-host32-ipc

> 本書は `kiro-validate-design` による**技術設計の品質レビュー**（非対話・レポート永続化モード）。対象は x64↔i686 の bytes-over-wire transport 層ユニット。検証日: 2026-07-01。判定プロセス: Analysis → Critical Issues → Strengths → GO/NO-GO。

## 設計レビュー要約

設計は pilot（go 済）で実証済みの transport 構造を production クレートへ忠実に昇格する内容で、境界（Owns/Out/Allowed/Triggers）・要件トレーサビリティ（全 31 AC）・依存方向・跨ビットネス規約がいずれも明確で実装準備が整っている。要件ディスカッションで確定した方針（respond 平関数・一様 timeout・`shiori-abi` 非依存）が設計へ正しく反映され、唯一の真の未決事項（クレート配置）は brief 要求どおり Open Question として surface されている（papering over なし）。ワークスペース `Cargo.toml` の実地確認により、依存ピン（`wintf-winmsg-executor=0.0.5` / `windows=0.62.2`）と `Win32_System_DataExchange` feature 未含（＝設計が指摘する追加要件）は事実として裏付けられた。

## Critical Issues（最大 3・design discussion への入力）

以下はいずれも **NO-GO には至らない**が、design discussion で潰しておくと実装リスクが下がる論点である。

🟡 **Issue 1（Medium）: heartbeat スレッドと single-in-flight 再入モデルの整合が未明示**
- **Concern**: `ParentMessageWindow` は「別スレッドから `PostMessageW(WM_NULL)` で `GetMessage` を起こす heartbeat」を `pump_until_hello_or` の bounded ループ制御に用いる（design.md State Management）。一方 `send_request` は `SendMessageTimeout` でブロック中に RESPONSE を再入受領する。往復中（InFlight 状態）に heartbeat 由来の別スレッド起床や WM_NULL 配送が WndProc に混入した際の順序・無害性（single-in-flight / ResponseSlot の clear→store→take 不変条件を壊さないこと）が設計上言語化されていない。
- **Impact**: 再入受領はデッドロック回避の核（Req 4.4）であり、heartbeat と再入の相互作用が曖昧なままだと、production 化で稀な取りこぼし・状態機械（InFlight↔Ready）の破れを生む恐れがある。
- **Suggestion**: heartbeat が有効なフェーズ（ハンドシェイク pump 限定か、往復中も回るか）と、往復中に WM_NULL/別スレッド起床が来た場合の WndProc の扱い（無視して即 return）を design discussion で 1〜2 文明記する。
- **Traceability**: 4.2, 4.3, 4.4, 3.4
- **Evidence**: design.md 「ParentMessageWindow … State Management / Concurrency strategy」「System Flows > 再入受領（4.1〜4.4）」

🟡 **Issue 2（Low〜Medium）: 単一クレートで x64 lib と i686 `[[bin]]` を両ターゲットビルドする配線が「タスクで確定」に留まる**
- **Concern**: 推奨案 Option B-1 は 1 クレート内に x64 lib（`windows` + `wintf-winmsg-executor`）と i686 helper `[[bin]]` を同居させる。`ipc.rs` を lib 経由 `use` で共有するか `#[path]` 直取りするか、`windows` feature を新クレート側で明示するかワークスペース features を拡張するか、helper bin を x64 ビルド時にどう扱うか（`required-features` / target 条件）が全て「実装細部・タスクで確定」とされている。配置確定（§1）と分離されているため、配置が B-1 に決まってもビルド配線の未決が残る。
- **Impact**: Req 7.1（i686 ビルド可能）のゲート成立に直結する構成判断であり、配線が破れると「そもそもビルドが通らない」失敗様式になり得る。ただし pilot が同型の物理共有（`#[path]`）で実証済みゆえリスクは中以下。
- **Suggestion**: クレート配置確定（§1）の直後、同じ discussion 内で「B-1 採用時の最小ビルド配線（`ipc` 共有方式・`DataExchange` feature の置き場・helper bin の target 条件）」を 1 案に寄せておく（タスクへ丸投げしすぎない）。
- **Traceability**: 7.1, 7.2, 2.1
- **Evidence**: design.md 「File Structure Plan（Open Decision 注記 / Modified Files）」「Open Questions/Risks §1・§2」

🟡 **Issue 3（Low）: 不正フレーム（Req 2.5）の「観測可能」性が観測カウンタ止まりで、テスト戦略に単独ケースがない**
- **Concern**: 未知タグ・`cbData` 不整合は「crash させず記録のみ・上位へ渡さない」とされ、Monitoring では `unknown_tags` 等のカウンタで観測する。しかし Integration Tests（tests/echo_roundtrip.rs）の列挙（往復 echo / handshake timeout / 応答 timeout / 異常終了検出）に、実 WM_COPYDATA で不正フレームを送って隔離を確認する統合ケースが無い（Unit Tests §4 は framing 関数の純粋テストで、WndProc 経路の隔離は覆っていない）。
- **Impact**: Req 2.5 は受信経路の防御要件であり、関数単体テストだけでは「WndProc が破損フレームを上位へ渡さない」実経路が未検証のまま GO する。安全側の欠落だが、production 品質の主張には穴になる。
- **Suggestion**: design discussion で、不正フレーム隔離を Unit（framing 関数）で足りると割り切るか、統合で 1 ケース足すかを一言決める（軽微・追記のみ）。
- **Traceability**: 2.5
- **Evidence**: design.md 「Testing Strategy > Unit Tests §4 / Integration Tests」「Error Handling > 不正フレームは隔離」「Monitoring」

## 設計の強み（Strengths）

- **凍結 seam の粒度が正確**: 「凍結するのは WM_COPYDATA ワイヤ形式（MsgTag / u32 LE HWND / cbData 境界）であって responder 実装ではない」と一貫して切り分け、Revalidation Triggers・Boundary Commitments・respond 差し替え点まで矛盾なく貫かれている。下流 3 ユニットとの契約面が壊れにくく、YAGNI（trait を設けない）判断も seam 定義と整合する。
- **要件確定事項の反映が忠実**: 一様 timeout（distinct PeerGone なし・生死は `ExitKind` で別系統）、`shiori-abi` 非依存、respond 平関数という要件ディスカッション確定 3 点が Design Decisions（D2/D3/D4）と本文・エラー型・Requirements Traceability に一貫反映されている。跨ビットネス安全（`u64` cast での shift 評価）も pilot 由来トラップを構造的に封じている。

## 最終判定

**判定: GO**

**根拠**: 既存アーキテクチャ（別プロセス＝天然のアクター境界・`wintf-winmsg-executor` 鏡像利用）と整合し、全 31 AC がコンポーネント／契約／フローへトレースされ、依存方向・エラー戦略・テスト戦略・i686 可搬性が実装可能な粒度で示されている。実現可能性リスクは pilot go で解消済みであり、上記 3 件はいずれも品質・網羅を高める Medium 以下の論点で、design discussion での軽微な明文化で解消できる。クレート配置の openness は brief が依頼者確認を要求する意図的な未決事項であり、設計はこれを健全に surface しているため欠陥として扱わない。

**次のステップ**:
1. design discussion で Issue 1（heartbeat×再入の整合明記）・Issue 2（B-1 ビルド配線の 1 案化）・Issue 3（2.5 隔離テストの扱い）を確定する。
2. 併せて Open Questions §1（クレート配置・命名）を依頼者確認で確定する。
3. 確定後 `/kiro-spec-tasks areka-P0-host32-ipc` でタスク生成へ進む。
