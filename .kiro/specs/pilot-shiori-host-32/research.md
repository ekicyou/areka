# ギャップ分析: pilot-shiori-host-32

> 種別: **先進坑（pilot・使い捨て）**。本書は確定済み requirements.md と既存コードベースの差分を分析し、
> 設計フェーズと要件ディスカッションの判断材料を提供する（決定はしない・情報提供に徹する）。
> 規律正本: `.kiro/steering/two-tunnel.md`／設計判断正本: `doc/COMPAT_ARCHITECTURE.md`／実輪郭: `doc/emo2-conformance-scope.md`。
> 分析日: 2026-06-30。

## 0. サマリ（3–5 行）

- **既存資産はほぼゼロ流用**: x64↔32bit ブリッジの実コードは皆無。既存の SHIORI 資産（`shiori-abi` の `IShiori` COM、`reference_brain.rs` の native in-proc 脳）は **x64 in-proc COM 経路**であり、本先進坑が検証する **32bit 別プロセス＋flat-C＋自前 IPC 経路とは別物**。命綱（葉ノード隔離）ゆえ本先進坑はこれら production 資産に**依存できない**（依存禁止）。流用できるのは「設計判断（`COMPAT_ARCHITECTURE.md §5`）」と「ビルド/隔離の構造（`crates/pilot` 検疫所・`_template`）」と「メッセージループ運用の知見（`pilot/examples/wintf-winmsg-executor`）」の 3 つ。
- **新規構築の主軸 5 ブロック**: (1) 32bit helper の別ターゲットビルド＋プロセス起動/監視/clean shutdown、(2) 自前 IPC（フレーミング＋タイムアウト＋プロセス生存監視）、(3) `LoadLibrary(pasta.dll)`＋`load/unload/request` flat-C エントリ解決、(4) SHIORI/3.0 `OnBoot` リクエスト組み立て＋`Value:` marshal（UTF-8）、(5) helper 側自前メッセージループの N 秒生存→clean unload。
- **最大の技術リスク**: 32bit ターゲットの**ビルド成立**（i686-pc-windows-msvc target 導入・cargo の per-example ターゲット指定不可問題）と、emo2 実物 `pasta.dll` の**入手・配置**（本リポジトリ外・`emo2-conformance-scope.md §0` の実物）。この 2 点が go 判定の前提を握る。
- **品質バーは緩い（使い捨て）**が**葉ノード隔離だけは厳守**。成果物はコードでなく知見（go／違う／直す＋学び）で、一次記録は README 3 幕。
- **要研究（design へ持ち越し）**: IPC 方式の選択（named pipe / stdio / 共有メモリ）、32bit helper のビルド成果物の配置と親からの起動方法、`pasta.dll` を CI/worktree で確保する手段、`request` flat-C シグネチャ（HGLOBAL 所有権規約）の正確な再現。

## 1. 現状調査（既存資産の棚卸し）

### 1.1 存在する土台と「流用可否」

| 資産 | 位置 | 内容 | 本先進坑での扱い |
|---|---|---|---|
| `crates/pilot`（検疫所） | `crates/pilot/` | 空 lib＋examples-only。`publish=false`・葉ノード。`wintf-winmsg-executor`/`event-listener`/`windows`/`windows-core` を探索依存として既に保持 | **受け皿として流用**。`examples/shiori-host-32/` を新設（1 仕様 = 1 フォルダ・`main.rs` 必須） |
| `examples/_template/` | `crates/pilot/examples/_template/` | `main.rs`（依存ゼロ）＋ README 3 幕雛形 | **コピー元**（要件 7.3）。これを `shiori-host-32/` へコピーして起点化 |
| `pilot/examples/wintf-winmsg-executor` | 同 examples 配下 | `wintf-winmsg-executor` の `block_on`/`spawn_local`/`Window::new_ex`／自前メッセージループ運用の実走知見（並行 tick・nested message・clean 終了） | **メッセージループ運用パターンの参照知見**。ただし x64 UI 用で**コピペ donor 禁止**（要件 5.3・two-tunnel 掘り直し禁止）。helper の自前メッセージループ実装の発想元としてのみ参照 |
| `crates/shiori-abi` | `crates/shiori-abi/src/` | `IShiori`/`IShioriHost` の **x64 内部 COM ABI**（HSTRING/UTF-16・move-out 所有権規約・PENDING/Complete 遅延応答・単一 in-flight） | **最終橋渡し先の内部契約**（upstream 前提）だが**本先進坑は依存しない**（命綱）。先進坑の IPC は flat-C バイト列で、COM 面には触れない。「最終的にここへ接続される」という方向性の確認のみ |
| `crates/areka/src/reference_brain.rs` | `crates/areka/src/` | `#[implement(IShiori)]` の **native x64 リファレンス脳**（in-proc COM 直結・`shiori_create` 純 C コンストラクタ・PENDING/Raise/Complete 正解見本） | **native 経路の見本**。32bit 過去互換経路（本先進坑）とは**経路が異なる**。流用不可・参照のみ |
| 設計判断正本 | `doc/COMPAT_ARCHITECTURE.md §5` | 過去互換経路＝32bit Rust ホスト（flat-C cdecl `load/unload/request`・HSTRING→charset byte 化・自前 IPC・HGLOBAL 規約・自前メッセージループ・毎秒 OnSecondChange ポーリング） | **設計判断の正本**。本先進坑はこの判断の「実現可能性一点突破検証」に徹する |
| emo2 実輪郭 | `doc/emo2-conformance-scope.md §0/§1` | `pasta.dll` = PE Machine 0x14C(x86/32bit)・SHIORI/3.0・UTF-8・SAORI 不使用・`Value:`＋`Charset: UTF-8` 応答 | **検証ターゲットの仕様根拠**。SHIORI/3.0 のヘッダ/応答フォーマットの一次根拠 |

### 1.2 「存在しないもの」（新規構築が必要）

- **x64↔32bit プロセス間ブリッジの実コードは皆無**（brief「存在しないもの」のとおり）。
- `crates/pilot/examples/shiori-host-32/` フォルダ自体が**未作成**。
- リポジトリ全体で `LoadLibrary`/`GetProcAddress`/`GlobalAlloc`/`HGLOBAL`/named-pipe/共有メモリ/IPC フレーミングの**実装コードはゼロ**（grep ヒットは全て spec/doc/steering の散文か無関係箇所）。
- 32bit ターゲット（i686）向けの**ビルド設定・cross target の例は本リポジトリに前例なし**（既存は全て x64／一部 ARM64 検証のみ。MEMORY: ARM64 ビルド知見はあるが i686 は別物）。

### 1.3 既存の規約・制約（遵守すべき構造）

- **命綱（葉ノード隔離・要件 7.2／two-tunnel）**: production クレート（wintf/dola/areka/shiori-abi）は本先進坑コードに依存してはならない。`crates/pilot` の空 lib＋examples-only 構造で構造的に担保。唯一の inbound 経路（他クレート Cargo.toml への一行依存追加）は人手レビューで捕捉。
- **品質基準（要件 7.4／two-tunnel 5.4）**: 整形・命名・テストの厳格さは production 品質まで求めない。緩めてよいのは品質であって隔離ではない。
- **Rust 2024・プロセス分離（要件 7.5）**: helper は 32bit ターゲット、親は x64。32bit/x64 境界を崩さない。
- **既知ビルド制約（brief Constraints）**: worktree で example を実ビルド/実行する際は前段で `git submodule update --init --recursive`（`vendors/pasta` 未 populate 回避）。ビルドは **PowerShell**（Git Bash の GNU `link.exe` が MSVC link を遮蔽する既知トラップ＝MEMORY arm64-windows-build と同根）。
- **掘り直し禁止（two-tunnel 5.3）**: 本坑（`areka-P0-host32-*`）は本先進坑のコードをコピペ donor 流用せず、README 知見を見て一から綺麗に掘り直す。

## 2. 要件→資産マッピング（ギャップ表）

タグ: **Missing**=既存資産になし新規／**Constraint**=既存構造/規約による制約／**Unknown**=design で要研究。

| 要件 | 技術的必要事項 | 既存資産 | ギャップ |
|---|---|---|---|
| R1 helper ライフサイクル | 32bit helper の別プロセス起動・生存監視・clean shutdown・異常検出 | なし（`pilot/wintf-winmsg-executor` は単一プロセス内 UI スレッド運用のみ） | **Missing**: 子プロセス起動（`CreateProcess`/`std::process::Command`）・終了コード/生死監視。**Unknown**: 32bit 成果物の配置と親からの起動パス解決 |
| R2 自前 IPC 往復 | メッセージフレーミング・応答受領・タイムアウト・生存監視併用 | なし | **Missing**: IPC レイヤ全体。**Unknown**: 方式選択（named pipe / stdio / 共有メモリ）＝option 節参照 |
| R3 SHIORI DLL 動的ロード | `LoadLibrary(pasta.dll)`＋`load/unload/request` flat-C 解決＋load(ghostdir)＋失敗観測 | なし（既存は COM `#[implement]` のみ・flat-C ロードは皆無） | **Missing**: `LoadLibraryW`＋`GetProcAddress`＋関数ポインタ transmute。**Constraint**: 32bit cdecl `extern "C"`（COMPAT §5）。**Unknown**: `pasta.dll` 実物の入手・配置 |
| R4 OnBoot 組み立て／Value 受領 | SHIORI/3.0 `OnBoot`(初回 `OnFirstBoot`) ヘッダ組み立て・`request` 渡し・`Value:` 抽出・UTF-8・x64 へ返送 | SHIORI/3.0 フォーマットは `emo2-conformance-scope §1` に根拠あり（実装はなし） | **Missing**: リクエスト文字列ビルド・レスポンスパース・`Value:` 抽出。**Constraint**: charset=UTF-8 固定（emo2）。**Constraint**: HGLOBAL 所有権規約（要求 DLL 解放／応答ホスト解放・COMPAT §5）|
| R5 自前メッセージループ生存 | helper 側メッセージループを N 秒回す→clean unload→go 基準(2)観測 | `pilot/wintf-winmsg-executor` に**メッセージループ運用の実走知見**あり（参照のみ・流用不可） | **Missing**: helper プロセス内の自前ループ（COM アパートメント回避・COMPAT §5）。**Constraint**: 32bit 側で `winmsg-executor` を使うか素の `GetMessage`/`PeekMessage` ループにするかは未決 |
| R6 README 3 幕記録 | 動機（本坑名指し）→概要・実行法→検証結果（go/違う/直す＋学び＋日付） | `_template/README.md` 3 幕雛形・two-tunnel README 規約 | **Missing**: 実内容。**Constraint**: subagent は `.md` Write 不可ハーネス制約（two-tunnel 3.6・MEMORY harness-shell-quirks）＝親が書く or PowerShell here-string |
| R7 二坑規律 | examples 隔離・production 非依存・`_template` コピー起点・緩品質・32bit/x64 境界保持 | `crates/pilot` 構造で構造的担保済み | **Constraint**: 構造維持のみ（新規実装なし）。隔離だけは厳守 |

## 3. 実装アプローチの選択肢

本先進坑全体は「新規 example 1 フォルダの新設」で確定（既存 example の拡張ではない・1 仕様 = 1 フォルダ規約）。
選択肢は**内部構成の取り方**で評価する。特に IPC 方式とビルド構成が主要分岐。

### 3.1 IPC 方式（R2 中核・要研究）

#### Option A: 名前付きパイプ（named pipe）
- **概要**: `CreateNamedPipe`/`ConnectNamedPipe`（親）↔ `CreateFile`（helper）。バイトストリーム＋自前フレーミング（長さ prefix）。
- **トレードオフ**: ✅ COMPAT §5 が第一候補に挙げる王道・双方向・メッセージモード可。✅ プロセス生存と接続断が連動し監視しやすい。❌ ハンドル受け渡し・接続ハンドシェイクの記述量が中程度。

#### Option B: stdio（子プロセスの stdin/stdout 継承）
- **概要**: 親が `std::process::Command` で helper を起動し stdin/stdout をパイプ継承。フレーミングは長さ prefix か行区切り。
- **トレードオフ**: ✅ 最小コード・`std` だけで完結・先進坑の使い捨て品質に最適。✅ プロセス終了＝EOF で生存監視が自然。❌ 窓持ち SHIORI の自前メッセージループと stdin ブロッキング読みの両立に工夫が要る（別スレッド読み or PeekMessage 併用）。❌ バイナリ安全性は自前フレーミング前提。

#### Option C: 共有メモリ（file mapping＋イベント同期）
- **概要**: `CreateFileMapping`＋`MapViewOfFile`＋名前付きイベントで signal。
- **トレードオフ**: ✅ 大容量・低コピー。❌ 同期プリミティブ自前管理が重く、使い捨て先進坑には**過剰**。1 往復検証には不要な複雑性。

#### Option D: WM_COPYDATA（窓メッセージによるプロセス間バッファコピー）★再評価で浮上・2026-06-30★
- **概要**: 親が `SendMessage(helperHwnd, WM_COPYDATA, selfHwnd, &COPYDATASTRUCT)`。OS が `lpData`（`cbData` バイト）を受信側アドレス空間へコピーし WndProc へ**同期配送**。応答は helper→親へ **2nd WM_COPYDATA**（親は SendMessage 待機中も着信 sent message を処理＝再入受領）。HWND 交換は起動時の小ハンドシェイク（親 HWND を arg、helper HWND を hello で返す）。
- **跨ビットネス**: x64↔x86 OK。ペイロードは OS がバイトコピー（bitness 安全）。HWND はシステムハンドルで跨いで有効。`dwData`(ULONG_PTR) はタグ用途・低 32bit のみ使えば可搬。
- **トレードオフ**: ✅✅ **helper の窓メッセージループに WndProc へ自然配送**＝overlapped / `MsgWaitForMultipleObjectsEx` / reader スレッド / 手動フレーミングが**全て不要**（`cbData` が長さ）。✅ 旧ローカル SSTP の実機構＝伺かドメイン idiom・実績。✅ single-in-flight 同期 req/resp にピタリ。❌ 両側に窓が要る（areka も helper も既に窓持ち＝非問題・pilot は message-only window 1 個）。❌ 応答は 2nd WM_COPYDATA（再入 SendMessage か `SendMessageTimeout`）。❌ SendMessage 専用（PostMessage 不可）。△ UIPI は integrity 跨ぎのみ（同ユーザ同 integrity 非問題）。crash 監視はプロセスハンドル wait で別途（IPC 直交）。
- **本評価**: host-32 の実ワークロード（小メッセージ・req/resp・single-in-flight・helper 窓持ち・legacy は pull 専用 COMPAT §89）には**最も素直**。**(B)=named pipe の動機「overlapped pipe × メッセージループ統合の難所を前倒し de-risk」は、WM_COPYDATA では当該難所が存在しないため消滅する**。

> **推奨の方向性（決定でなく情報）**: 先進坑の「最小 1 往復＋使い捨て」目的には **Option B（stdio）か Option A（named pipe）** が妥当。stdio は最小実装で go 基準(1)往復を最短検証でき、named pipe は本坑 `areka-P0-host32-ipc`（pipe＋handshake/lifecycle と明記・roadmap）に方向が近く知見の転用価値が高い。**どちらを先進坑で掘るかは要件ディスカッション/design の判断事項**。共有メモリは先進坑では非推奨（過剰）。

#### 3.1.1 決定因子＝メッセージループ共存（要件ディスカッションで深掘り・2026-06-30）

IPC 方式の真の決定因子は throughput でも payload サイズでもなく、**helper の窓持ち SHIORI 用メッセージループ（要件 5）と IPC read の共存**である。IPC read が `GetMessage` ループを塞いではならない。

- **stdio（匿名パイプ）**: overlapped 不可ゆえ blocking read → **専用 reader スレッド必須**（I/O スレッドが read → UI スレッドへ post → UI スレッドが `pasta.dll` request → 応答を I/O スレッドへ戻して write）。最小コード・`std::process::Command` で完結・プロセス終了=EOF で生存監視が自然。
- **named pipe**: overlapped I/O ＋ `MsgWaitForMultipleObjectsEx` で**単一スレッドで窓メッセージとパイプ完了を同時待機**可。全二重・broken pipe で crash 即検出・roadmap `host32-ipc`（pipe＋handshake/lifecycle）一致・OnSecondChange タイマも自然。コード量は中。

**方式の決定（要件ディスカッション・2026-06-30）**:
- **最終決定は設計議題**（`/kiro-design`）。要件フェーズでは IPC 方式をハード固定しない（開発者判断）。
- **強く推奨 = Option D（WM_COPYDATA）**: HWND 交換も含め通信を **Window Message に一本化**。helper の `wintf-winmsg-executor` メッセージループの WndProc へ自然配送＝(B) が de-risk しようとした「overlapped pipe × メッセージループ統合」の難所**そのものが存在しない**。最小・single-in-flight 同期 req/resp・伺か idiom。技術調査（§6）で i686 ビルド／`wintf-winmsg-executor` i686／cdecl flat-C を実証済で前提も固い。
- **named pipe（Option A）は設計上の強い理由がある場合のみ受容**（全二重/大データ/push が要るとき。但し legacy=pull 専用・payload 小ゆえ host-32 では通常不要）。
- 速度は非決定因子（下記補足）。選択はメッセージループ共存エルゴノミクスで決する。
- **HWND ハンドシェイク**: 別 side-channel（pipe 等）を混ぜず **Window Message パラダイムに統一**（例: 親 HWND を seed し helper が自 HWND を hello メッセージで返す）。具体は design で確定。

**bitness 安全規約（方式共通）**: 跨ぐのは生バイト列のみ（§5.4）。フレーミングは**固定幅 LE 長さ prefix（u32 LE）**。payload に**ポインタ/HANDLE/struct を載せない**（同一マシンで endian 共通だが固定幅で明示）。親 x64 が i686 helper exe を `CreateProcess`/`std::process::Command` で起動（exe パスは §3.2 の 2 段ビルド成果物を arg/env で受け渡し）。

**速度は決定因子でない（補足・2026-06-30）**: ローカル pipe 往復は現代 Windows で **~µs オーダ**、SHIORI の cadence は **OnSecondChange=1 Hz ＋人間操作ペース（体感予算 ~数十 ms）**ゆえ IPC レイテンシは要求を 3〜6 桁下回り**非律速**。応答性を決めるのは脳（`pasta.dll`）処理＋描画＋ **`IShiori` の `PENDING`/`Complete` 非同期**（UI スレッドを脳で塞がない）であって transport の生速度ではない。共有メモリの優位は**大 payload 時のみ**で SHIORI の小メッセージ（数百 B〜数 KB）には不要。modern browser（Chromium Mojo / Firefox IPDL）も **「制御＝named pipe/unix socket」＋「bulk＝共有メモリ＋handle passing」のハイブリッド**で、小メッセージは pipe・大データのみ shm。host-32 は payload 小ゆえ **bulk 路不要＝named pipe 単独で十分**（本坑）。旧 SSTP 期の「共有メモリで速度を稼ぐ」発想は host-32 の通信レートには非適用。→ stdio/named pipe の選択は**速度でなくループ共存エルゴノミクス**で決する。

### 3.2 32bit helper のビルド/配置構成（R1/R7・要研究）

#### Option A: 同一 example 内に親 main＋helper を内包し実行時に自己再起動
- 1 つの `main.rs` が引数/環境変数で「親モード」「helper モード」を分岐。親が自分自身を 32bit で…は**不可**（example は単一ターゲットでビルドされ x64/32bit を 1 バイナリで両立できない）。→ **却下寄り**（32bit/x64 分離を崩す）。

#### Option B: helper を別 example（または別バイナリ）として 32bit ターゲットでビルドし、親 x64 example が起動
- helper を `cargo build --target i686-pc-windows-msvc` で別途ビルドし、x64 親がそのパスを起動。
- **トレードオフ**: ✅ 32bit/x64 分離（要件 7.5）を素直に満たす。✅ COMPAT §5 の「随伴バイナリ」像と一致。❌ cargo の example は per-target 指定が難しく、ビルド手順が 2 段（親 x64＋helper i686）になる。❌ helper バイナリのパス解決を親に渡す段取りが要る。**Unknown**: i686-pc-windows-msvc target の導入（rustup target add）と MSVC 32bit リンカの可用性。

#### Option C: helper を pilot 内の独立サブクレート/別 example にして build.rs/手動でビルド連携
- **トレードオフ**: ✅ 構成が明示的。❌ 先進坑の「緩く速く」に対し段取りが重い。検疫所一葉（`crates/pilot`）に閉じる原則と、サブクレート増設の是非は要検討。

> **推奨の方向性（情報）**: **Option B** が 32bit/x64 分離規律と COMPAT §5 像に最も整合。ビルド 2 段手順と helper パス受け渡しは README の「実行法」幕に明記する前提で受容可能。先進坑ゆえ手作業ビルド手順でよい（CI 自動化は不要）。**i686 target の導入可否は go の前提**であり design で先に潰すべき要研究項目。

### 3.3 自前メッセージループの実装（R5）
- **Option A: 素の Win32 ループ**（`PeekMessage`/`GetMessage`＋`DispatchMessage`）を helper に直書き。最小・COMPAT §5 の「自前メッセージループ」像に直結。stdio 読みと併用するなら別スレッド I/O＋`PeekMessage` 非ブロッキング。
- **Option B: `wintf-winmsg-executor` を helper でも使う**（pilot は既に依存保持）。ただし 32bit ターゲットでの当該クレート可用性は**未検証**（Unknown）。先進坑の最小性からは Option A が無難。

## 4. 工数・リスク評価

| ブロック | 工数 | リスク | 一行根拠 |
|---|---|---|---|
| 32bit ターゲットのビルド成立（i686 target＋MSVC linker） | S–M | **High** | リポジトリに前例なし・PowerShell 必須トラップ・target 導入要（MEMORY arm64 と同種だが i686 は別）|
| `pasta.dll` 実物の入手・配置 | S | **High** | リポジトリ外（`ghost_dev` 配下）・worktree/CI での確保手段が未確立 |
| 子プロセス起動・生存監視・clean shutdown（R1） | S | Low | `std::process`/Win32 の素直な利用 |
| 自前 IPC（フレーミング＋タイムアウト＋監視）（R2） | M | Medium | 方式選択次第・フレーミングとタイムアウトは定石だが自前 |
| `LoadLibrary`＋flat-C エントリ解決＋load(ghostdir)（R3） | S–M | Medium | flat-C cdecl・関数ポインタ transmute・unsafe 境界。実物 DLL があれば素直 |
| OnBoot 組み立て＋Value 抽出＋UTF-8 marshal（R4） | S | Low | SHIORI/3.0 は単純な `key: value` CRLF＋空行終端（`emo2-conformance-scope §1`）|
| 自前メッセージループ N 秒生存→clean unload（R5） | S–M | Medium | `wintf-winmsg-executor` 知見あり（参照）・stdio との両立に工夫 |
| README 3 幕記録（R6） | S | Low | 雛形あり・記述のみ |

> **総合**: 工数は **M（3–7 日）**規模。コア難所は**ビルド成立**と**実物 DLL 確保**の 2 つ（共に High）で、ここが go の前提。残りは既知パターンで Low–Medium。

## 5. 設計フェーズへの引き継ぎ

### 5.1 推奨アプローチ（情報・最終は人間判断）
- 構成: `examples/shiori-host-32/` を `_template` からコピー新設。**親 x64 example＋32bit helper（別ターゲットビルド・Option B 3.2）**。IPC は **stdio（最小・3.1 B）または named pipe（本坑転用価値・3.1 A）** のいずれかを design で選択。メッセージループは **素の Win32 ループ（3.3 A）**。
- 検証順序: ①ビルド成立（i686）→②`pasta.dll` ロード＋エントリ解決→③1 往復（load→OnBoot→Value→unload）＝go 基準(1)→④メッセージループ N 秒生存→clean unload＝go 基準(2)。前段が立たないと後段は観測不能ゆえ、**ビルド成立と DLL 確保を最優先で潰す**。

### 5.2 要研究項目（design で先に潰す・Research Needed）
1. **i686-pc-windows-msvc target の導入可否と 32bit MSVC リンカの可用性**（go の前提・High）。
2. **emo2 実物 `pasta.dll` の入手元・配置方法**（リポジトリ外・worktree/手動実行での確保手段）。
3. **IPC 方式の最終選択**（stdio vs named pipe）と、選択に応じた**フレーミング/タイムアウト/生存監視**の具体。
4. **`request` flat-C シグネチャと HGLOBAL 所有権規約**の正確な再現（要求 HGLOBAL は DLL 解放／応答 HGLOBAL はホスト解放・COMPAT §5）。実物 DLL のエクスポート規約（cdecl）確認。
5. **helper の自前メッセージループ実装方式**（素 Win32 か `wintf-winmsg-executor` か）と stdio/pipe 読みとの両立。
6. **`load` の ghostdir 引数**に何を渡すか（emo2 ゴーストフォルダの実パス・`ghost/master` 相当）。

### 5.3 要件ディスカッションへ上げる設計判断アイテム（番号付き）
1. **IPC 方式の選択**: 先進坑で stdio（最小・最速往復検証）と named pipe（本坑 `areka-P0-host32-ipc` への知見転用）のどちらを掘るか。共有メモリは非推奨で合意してよいか。
2. **32bit helper のビルド/配置構成**: Option B（別ターゲット別バイナリ・2 段ビルド）で確定してよいか。helper を別 example にするか pilot 内サブクレートにするか。
3. **`pasta.dll` 実物の調達方針**: 検証時に手動配置（リポジトリ外パス指定）で許容するか、worktree への取り込み手段を design で定めるか。go の前提を「DLL が手元にある環境でのみ検証」と明示してよいか。
4. **メッセージループ実装方式**: 素 Win32 ループ（最小）と `wintf-winmsg-executor` 流用（既存知見）のどちらを先進坑で採るか。32bit での当該クレート可用性未検証リスクをどう扱うか。
5. **`Value:` 受領の x64 親到達の検証粒度**: 先進坑では「x64 親プロセス側で `Value:` 文字列を受領・標準出力で確認」までで go 基準(1)充足とみなすか（COM `IShiori` 面への接続は本坑領分で対象外、で合意してよいか）。
6. **N 秒運転の N の値**と clean unload の合否観測方法（プロセス終了コード／ログ／メッセージループ停止確認のどれを一次記録にするか）。
7. **OnBoot/OnFirstBoot の区別**: 先進坑では `OnBoot` 1 種で足りるか（初回扱いの `OnFirstBoot` 相当を 1 リクエストで代表させてよいか・要件 4.1 の「1 種」と整合）。

### 5.4 確定した設計方向（要件ディスカッションで合意・本坑へ引き継ぎ）

> 本節は先進坑 spec の範囲を超える**本坑アーキ方向**だが、先進坑の知見転用先として要件ディスカッションで合意したため記録する。正本反映は本坑 design ／ `doc/COMPAT_ARCHITECTURE.md` 更新時に行う。

**用語（便宜上の呼称）**:
- **SHIORI4** = areka 正準 content（json-rpc 相当・構造化）。`IShiori` 境界を流れる**不透明 HSTRING**（`interface.rs`：本層ではパースしない・正準 content）。
- **SHIORI3** = レガシーワイヤ形式（`key: value` CRLF ＋空行終端・SHIORI/3.0）。

**変換配置（どこで SHIORI4⇄SHIORI3 を変換するか）**:
- conductor/main は **SHIORI4 のみ**を扱う（ワイヤ非依存・呼び出し側に native/過去互換の分岐を出さない＝COMPAT §75）。
- **SHIORI4 ⇄ SHIORI3 変換 ＋ charset 符号化は x64 過去互換 `IShiori` アダプタ**が担う（`IShiori` の**下**・IPC の**上**）。32bit helper は **SHIORI3 ロジックを一切持たない**バイト proxy に徹する。
- 根拠: COMPAT §5（75/85/86）・`interface.rs`（content 不透明・正準 content）。

**プロセス間メモリ表現（HGLOBAL も HSTRING も跨がない）**:
- x64⟷x86 IPC を跨ぐのは **charset 符号化済みの生バイト列のみ**。
- **HGLOBAL = 32bit helper ローカル**: `GlobalAlloc` で確保し pasta.dll へ渡す。SHIORI3 所有権規約（要求 HGLOBAL は DLL が解放／応答 HGLOBAL はホストが解放）ゆえ 32bit ローカル確保が必須。プロセスローカルで跨げない（COMPAT §85 明記）。
- **HSTRING = x64 ローカル**: バッキングは UTF-16 ヒープ（プロセスローカル）。HSTRING は WinRT プリミティブゆえ標準 OOP マーシャラを持つが、areka は `IShiori` を **in-proc 直 vtable**で呼び OOP 自動マーシャリングを発生させない（COMPAT §92/93）ため、**HSTRING はそもそもプロセスを跨がない**。
- 対称性: 各プロセスが自前の「文字列/メモリ通貨」（x64=HSTRING／x86=HGLOBAL）を持ち、橋を渡るのは生バイト列のみ。

**先進坑への反映（議題 #5 = (A) 確定）**:
- 先進坑も **helper = バイト proxy・x64 親で SHIORI3 組立／`Value:` parse**。go 基準(1) は「x64 親が `Value:` を受領・確認」で充足とし、COM `IShiori` 接続は本坑領分（対象外）。これにより先進坑の形が本坑 x64 アダプタの**ミニチュア**になり、知見が直接転用できる。

## 6. 技術調査結果（要件フェーズで先行 de-risk・2026-06-30）

> 開発者要望「技術調査を先に」により、go の前提を握る環境未知を要件フェーズで先行実証。実機検証ゆえ design へ確証として引き継ぐ（README 検証結果とは別の事前調査記録）。

| 調査項目 | 結果 | 方法 |
|---|---|---|
| **i686-pc-windows-msvc ビルド成立**（#1 High リスク・§4） | ✅ **GO** | 最小 bin を `cargo build --target i686-pc-windows-msvc` → PE Machine **0x014C** 生成 → 実行成功。MSVC x86 リンカ在。rustup target 導入済（aarch64/i686/x86_64） |
| **`pasta.dll` の SHIORI エクスポート＋呼出規約** | ✅ `load`/`unload`/`request` を**装飾なし＝cdecl flat-C** で確認（PE32・machine 0x014C・export 181 件中に存在） | committed fixture の PE export table を解析 |
| **`windows` 0.62.2 ＋ `wintf-winmsg-executor` 0.0.5 の i686 ビルド** | ✅ **GO**（全 stack コンパイル完走） | scratch bin に依存追加し i686 ビルド |

**これが解いた設計不確実性**:
- **#1 High リスク（i686 ビルド）消滅** → go の最大の環境前提が立った。
- **gap §3.3 メッセージループ実装（Cat-B #4）**: `wintf-winmsg-executor`（i686 ビルド検証済）を **helper で流用**で確定可。raw Win32 自前ループは不要。
- `pasta.dll` の cdecl flat-C 確認 → §5.2 item 4（`request` シグネチャ）の前提が固まる。
- いずれも **Option D（WM_COPYDATA）** を後押し: 親 x64・helper 双方が `wintf-winmsg-executor` で窓を持てば、WM_COPYDATA が WndProc へ自然配送＝統合作業ゼロ。

**残る go-gating でない未知（design で詰める）**: `request` の HGLOBAL 所有権の実挙動・`load` の ghostdir 実引数・WM_COPYDATA 往復の再入（応答）実装。

---

> 本書は情報提供であり決定ではない（two-tunnel／kiro-validate-gap 原則）。go 判定は開発者の人間判断（要件 6.5・two-tunnel ハードゲート）。
> 次フェーズ: 要件ディスカッション（上記 §5.3 を論点に）→ `/kiro-design pilot-shiori-host-32`。
