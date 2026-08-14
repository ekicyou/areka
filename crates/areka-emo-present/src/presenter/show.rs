//! `ShowSurface` の適用（`EmoPresenter::apply_show`）——k 導出・引き当て／合成・供給面遅延生成・
//! アップロード＋マスク同期＋可視化・表示成立点の状態更新を 1 呼び出しで行う単一漏斗。

use std::sync::Arc;

use wintf::ecs::widget::bitmap_source::AlphaMask;

use super::budget::AllocSite;
use super::timing::{EmitContext, FrameTiming, Stage, compose_key_hash};
use super::{
    AlphaMaskResource, BindSet, ComposeError, ComposedSurface, DPI, EmoPresenter, GraphicsCore,
    PatternState, PresentError, PresentOutcome, ReplySender, SwapChainPresenter, TargetId,
    VisibilityOwnership, VisualMount, World, WucGraphicsResource, derive_scale, resample,
};

impl EmoPresenter {
    /// `ShowSurface` の適用（キャッシュ引き当て or 合成 → 供給面アップロード → マスク同期 → 可視化）。
    ///
    /// 手順（design §System Flows・Flow 1）: (1) 未装着なら error! ＋ `Err(TargetNotAttached)`。
    /// (1.5) 窓の `DPI` component と target 政策から**この適用に使う k**を導出する（[`derive_scale`]・
    /// component 不在は `None` のまま渡して要件 1.4 の縮退へ落とす）。以降 k は合成入力と同格のキー
    /// 要素であり、ミス時は合成（native）→ [`resample`]（k 適用）を経て挿入される。(2) 合成入力
    /// （surface id＋bind 集合）が直前と完全一致するヒットなら再合成しない（R4.2）——bind 集合が
    /// 1 要素でも異なれば必ずミス＝再合成する（着せ替え・まばたきの正しさの担保）。(3) ミスなら合成し、
    /// `SurfaceNotFound` は error! ＋表示不変＋
    /// `Err`（R3.4）、`EmptyComposition` は warn! ＋ Hide 縮退＋`Ok`（設計ディスカッション #1）、`Ok` なら
    /// マスクを 1 回だけ生成して `cache.insert` へ表示バッファと対で渡す。(4) 使えるエントリで、`chain`/`mount` 未生成なら原寸確定
    /// 後に遅延生成し、`chain.upload` ＋ `AlphaMaskResource::set` ＋ 可視化を同一呼び出し内で行う（R2.4）。
    ///
    /// # 可視化手順だけが所有権でゲートされる（`areka-P0-balloon-visibility` Requirement 1.1/6.2/6.9）
    ///
    /// [`VisibilityOwnership::External`] の target では、末尾の可視化（`set_visible(true)`／`visible=true`）
    /// と mount 遅延生成の初期可視性だけが変わり、**それ以外の全手順は所有権に依らず共通**である
    /// （k 導出・合成・キャッシュ・アップロード・マスク・`set_bounds`・`applied`／`native_size`／
    /// `last_show`／`pending_resize`・下の `info!`）。表示成立点が 1 つであることを崩さないための境界であり、
    /// 「不可視のまま配置先と面が確立する」（Requirement 1.3）はこの共通部分がそのまま与える。
    /// 可視化は [`EmoPresenter::show_target`] が同じ漏斗を再通過したうえで付与する。
    pub(super) fn apply_show(
        &mut self,
        world: &mut World,
        target_id: TargetId,
        surface_id: u32,
        binds: BindSet,
        pattern: PatternState,
        reply: Option<ReplySender<PresentOutcome>>,
    ) {
        // (−1) 段階計時の起点（Requirement 1.1・design D5）。**無条件**に開始する——ログ設定を
        // 一切参照しないため、計時ログの有効・無効で表示経路が分岐しない（Requirement 1.5）ことが
        // 檻ではなく構造で成立する。`emit` は `self` を消費するため、以降の early return は
        // すべて計時器を黙って drop する＝**成立点の対でしか行が出ない**（design §FrameTiming）。
        let mut timing = FrameTiming::start();

        let Some(target) = self.targets.get_mut(&target_id) else {
            tracing::error!(
                ?target_id,
                surface_id,
                "apply(ShowSurface): 未装着ターゲット"
            );
            Self::reply(reply, Err(PresentError::TargetNotAttached(target_id)));
            return;
        };

        // (0) k 導出（show 適用ごと・design Flow 1）。窓 DPI は wintf の `DPI` component から読む
        // （consume のみ・新規依存なし）。**component 不在は `None` のまま [`derive_scale`] へ渡す**——
        // ここで 96 を捏造すると要件 1.4 の縮退（error! ＋ k=1.0）が「正常系のふり」で通ってしまう。
        let window = target.window;
        let window_dpi = world.get::<DPI>(window).map(|d| (d.dpi_x, d.dpi_y));
        let scale = derive_scale(target.policy, window_dpi);

        // (1) 引き当て: 合成入力（id＋binds＋pattern）＋表示スケール k の完全一致のみヒット＝再合成
        // しない（R4.2/R5.2・要件 2.4）。ミスのみ合成する。pattern は指令が運ぶ現在コマ集合をそのまま
        // 透過する（presenter は新しい判断を持たず輸送のみ）。空 PatternState なら拡張前と観測等価
        // （R5.4）。k が変われば必ずミスするため、旧 k の絵とマスクを表示に載せることはない（設計 D6）。
        let cache_hit = target
            .cache
            .get(surface_id, &binds, &pattern, scale)
            .is_some();
        timing.mark(Stage::CacheLookup);
        if !cache_hit {
            // 合成結果を先に束縛して境界を作る（計時点 `Stage::Compose` を合成の直後に置くため）。
            // 分岐・呼出順序・エラー経路は下の `match` がそのまま担い、挙動は不変である。
            let composed = target
                .composer
                // pattern を合成入力の第一級要素として合成器へ透過する（R5.1）。
                .compose(
                    &target.emo_world,
                    &target.atlas,
                    surface_id,
                    &binds,
                    &pattern,
                );
            timing.mark(Stage::Compose);
            match composed {
                Ok(composed) => {
                    // `Composer::compose` は毎回 `ComposedSurface::new(0,0)` を起こし、内部の
                    // `resize_and_clear` が外形まで伸長する（emo-compose lib.rs:158）。確保は上流の
                    // 内部で起きるため、観測点は**合成が成功した直後のこの呼出側**に置く（Err は
                    // 伸長前に落ちるため数えない）。是正（task 5.3）で `compose_into`＋常設席へ
                    // 移ると、同じ計数が「席の再利用が成立しなかった回」を指すようになる。
                    target.budget.note_alloc(AllocSite::ComposeDst);
                    // 合成は常に native 原寸（emo-compose の合成経路は k を知らない・設計 D3 の A2）。
                    let native_extent = (composed.width(), composed.height());
                    // k 適用（要件 2.1/2.3）: 合成済みの 1 枚（element 入れ子・SERIKO パターン・mayuna
                    // 着せ替えが畳み込まれた結果）へ**単一の k** を掛けるため、要素間の相対配置・重なりは
                    // 等倍時と同一の見た目関係を保つ。恒等 k は resample を呼ばず native を素通しする
                    // （要件 7.2: 既存 golden がバイト単位で不変であることの構造保証・割り当ても増えない）。
                    let display = if scale.is_identity() {
                        composed
                    } else {
                        let mut scaled = ComposedSurface::new(0, 0);
                        // 表示バッファ（リサンプル先）の新規確保。0×0 で起こして `resample` 内の
                        // `resize_and_clear` が出力外形まで伸長する（is-a A2）。
                        target.budget.note_alloc(AllocSite::ResampleDst);
                        // リサンプル作業領域（x 軸写像表 `Vec<AxisSample>`・emo-compose scale.rs:423）
                        // は `resample` の内部で毎回新規に組まれる私有バッファであり、呼出側から
                        // 到達できる観測点はこの呼出そのものである（是正 task 5.2 で `ResampleScratch`
                        // の常設席へ移ると、同じ計数が再利用の不成立を指す）。
                        target.budget.note_alloc(AllocSite::Xmap);
                        resample(&composed, scale, &mut scaled);
                        timing.mark(Stage::Resample);
                        scaled
                    };
                    // マスクをこの適用で 1 回だけ生成する（R2.1/R2.4）。生成点は `cache.insert` の
                    // 内側から**呼出側のここへ移った**（設計 D4）——最終的な住所は `FrameBudget` の
                    // 輪番シーム（task 5.2/5.3）であり、ここはその途中形である。挿入は表示バッファと
                    // マスクを同時に受け取るため、対が崩れないことは引き続き構造で担保される。
                    let mask = Arc::new(AlphaMask::from_pbgra32(
                        display.bytes(),
                        display.width(),
                        display.height(),
                        display.stride(),
                    ));
                    // 当たり判定マスクの新規確保（`vec!` 1 本）。計数はその確保の**直後**に置く
                    // （是正 task 5.2/5.3 で輪番スロットの in-place 再生成へ移ると、同じ計数が
                    // 「輪番が unique にならず新規確保へ落ちた回」を指すようになる）。
                    target.budget.note_alloc(AllocSite::Mask);
                    // pattern は binds と同格のキー要素として挿入キーへ透過する（R5.2）。マスクは
                    // k 適用済み bytes 由来ゆえ物理 px 契約が無修正で整合する（設計 D6）。
                    target.cache.insert(
                        surface_id,
                        binds.clone(),
                        pattern.clone(),
                        scale,
                        display,
                        mask,
                    );
                    // `Stage::MaskGen` の区間はマスク生成＋挿入全体（スロット置換＋旧エントリ解放）を
                    // 含む——旧エントリの解放は is-a A6 そのものであり、内訳として一体で読むのが正しい。
                    timing.mark(Stage::MaskGen);
                    // スロットの中身と対で原寸を控える（`insert` と同じ場所＝対が崩れない唯一の書き方）。
                    // 以降この回が失敗して early return しても、後からヒットで表示が成立した時点で
                    // 正しい原寸が照会契約へ渡る。
                    target.cached_native = Some(native_extent);
                }
                Err(ComposeError::EmptyComposition(id)) => {
                    // 全透明退化（外形 0×0）: 許容される正常退化として Hide 縮退＋reply Ok（skip ではない）。
                    tracing::warn!(
                        ?target_id,
                        surface_id = id,
                        was_visible = target.visible,
                        "apply(ShowSurface): 全透明退化（EmptyComposition）→ Hide 縮退（reply Ok）"
                    );
                    if let Some(mount) = target.mount.as_ref() {
                        mount.set_visible(world, false);
                    }
                    target.visible = false;
                    // EmptyComposition 縮退は Hide と同じ表示結果ゆえ現サーフェス無し（R3.2・Key decisions (b)）。
                    target.current_surface_id = None;
                    Self::reply(reply, Ok(()));
                    return;
                }
                Err(e) => {
                    // 解決不能 id（SurfaceNotFound 等）: error! ＋ 表示不変 ＋ reply Err（R3.4）。
                    tracing::error!(
                        ?target_id,
                        surface_id,
                        error = %e,
                        "apply(ShowSurface): 合成失敗 → 表示は適用前のまま（reply Err）"
                    );
                    Self::reply(reply, Err(PresentError::Compose(e)));
                    return;
                }
            }
        }

        // (2) 供給面・装着の遅延生成（初回表示・原寸確定後）。
        if target.chain.is_none() {
            let (w, h) = {
                let entry = target
                    .cache
                    .get(surface_id, &binds, &pattern, scale)
                    .expect("直前に引き当て済み");
                (entry.composed.width(), entry.composed.height())
            };

            // Compositor は所有クローンで取り出し、以後の &mut World 装着と借用衝突しないようにする。
            let Some(compositor) = world
                .get_resource::<WucGraphicsResource>()
                .and_then(|r| r.compositor().cloned())
            else {
                tracing::error!(
                    ?target_id,
                    "apply(ShowSurface): WucGraphicsResource/Compositor 不在（供給面を生成できない）"
                );
                Self::reply(
                    reply,
                    Err(PresentError::Device {
                        hresult: 0,
                        context: "WucGraphicsResource::compositor",
                    }),
                );
                return;
            };

            // GraphicsCore は生成呼び出しの間だけ借用する（surface は所有で返るため借用は閉じる）。
            let new_chain = {
                let Some(gfx) = world.get_resource::<GraphicsCore>() else {
                    tracing::error!(
                        ?target_id,
                        "apply(ShowSurface): GraphicsCore 不在（供給面を生成できない）"
                    );
                    Self::reply(
                        reply,
                        Err(PresentError::Device {
                            hresult: 0,
                            context: "GraphicsCore resource",
                        }),
                    );
                    return;
                };
                SwapChainPresenter::new(gfx, &compositor, w, h)
            };
            let (chain, surface) = match new_chain {
                Ok(pair) => pair,
                // SwapChainPresenter::new は内部で error! 済み（chain.rs device_err）。ここは reply のみ。
                Err(e) => {
                    Self::reply(reply, Err(e));
                    return;
                }
            };

            // 初期可視性は**所有権から導出**する（`areka-P0-balloon-visibility` Requirement 1.2）。
            // `CommandDriven` は従来どおり可視で構築（この漏斗の末尾で `set_visible(true)` する経路と
            // 同値）、`External` は不可視で構築する——「可視で spawn してから消す」経路を持たないことで、
            // 可視状態を component レベルでも一度も経由しないことを構造で保証する。導出をここで行う
            // （装着側の既定に委ねない）のは、可視性の所有者を知っているのがこの層だけだからである。
            let initially_visible = matches!(target.ownership, VisibilityOwnership::CommandDriven);
            let mount = match VisualMount::attach(
                world,
                window,
                &surface,
                &compositor,
                (w, h),
                initially_visible,
            ) {
                Ok(m) => m,
                // VisualMount::attach も内部で error! 済み（mount.rs device_err）。
                Err(e) => {
                    Self::reply(reply, Err(e));
                    return;
                }
            };

            target.chain = Some(chain);
            target.mount = Some(mount);
        }

        // (3) 供給面アップロード ＋ マスク同期 ＋ 可視化（同一呼び出し内＝原子入替・R2.4）。
        //
        // **可視化手順だけ**が所有権でゲートされる（`areka-P0-balloon-visibility` Requirement 6.2/6.9）。
        // `External` の target では表示状態の確立（合成・供給面・マスク・bounds・k・面 id・再表示入力・
        // 窓寸 reconcile 要求）は従来どおり全て完了し、可視性の付与だけを行わない——面切替もループ由来の
        // 反復指令も、経路を問わず可視状態を変えない。
        let visualize = matches!(target.ownership, VisibilityOwnership::CommandDriven);
        let entry = target
            .cache
            .get(surface_id, &binds, &pattern, scale)
            .expect("直前に引き当て済み");
        let chain = target.chain.as_mut().expect("直上で生成済み");
        if let Err(e) = chain.upload(&entry.composed) {
            // upload は内部で error! 済み（chain.rs）。表示は前状態を保つ（成功まで旧状態不変）。
            Self::reply(reply, Err(e));
            return;
        }
        timing.mark(Stage::Upload);
        // 表示物理寸は**供給面の実寸**を単一真実源とする（upload が外形変化を検知して合わせ込んだ後の
        // 値＝k 適用済み composed の外形）。エントリ外形から別途組み立てないことで、供給面・visual
        // 境界・マスクが同一の物理寸に揃うことを構造で担保する（R3.2・k 追従は A2 の自動追従）。
        let size = chain.size();

        let mount = target.mount.as_ref().expect("直上で生成済み");
        if let Some(mut mask_res) = world.get_mut::<AlphaMaskResource>(mount.surface_entity()) {
            // 表示バッファと同一 bytes 由来のマスクを hit-test へ供給する（R2.2/R2.5）。
            // エントリ側が `Arc` 表現になったための機械的追随で、**供給は従来どおり実体の複製**
            // （`set`）である。参照カウント増だけで済ませる `set_shared` への置換は task 5.3。
            mask_res.set(entry.mask.as_ref().clone());
        } else {
            tracing::warn!(
                ?target_id,
                entity = ?mount.surface_entity(),
                "apply(ShowSurface): surface entity に AlphaMaskResource が無い（当たり判定は矩形/前状態）"
            );
        }
        if visualize {
            mount.set_visible(world, true);
        }
        // bounds（＝αマスク座標基準）は可視性に依らず常に合わせる。不可視中の更新は安全であり
        // （`Arrangement`・`SpriteVisual::SetSize` は合成コミットと独立、`HitTest::none()` 中はマスクが
        // 判定に使われない）、ここで揃えておくことが「不可視期間中に生じた変化を取りこぼさない」
        // （Requirement 6.6）の土台になる。
        mount.set_bounds(world, size);
        if visualize {
            target.visible = true;
        }
        // 表示確立＝この id が現サーフェス（全透明でも成立・α 非依存の単一真実源・R3.1/3.3・Key decisions）。
        target.current_surface_id = Some(surface_id);
        // ここが**表示成立点**＝ k・native 原寸・再表示入力の唯一の更新点（design Flow 1 キー決定）。
        // 手前の失敗経路はすべて early return 済みゆえ、失敗時は前 k・前表示が保たれる（要件 4.4）。

        // (3.5) 状態照合＝窓寸 reconcile 要求の生成（design Flow 1 キー決定・議題 #2 裁定）。
        //
        // **前値を上書きする前に**前回適用の物理寸を組み立てる。組み立ては契約式
        // `物理寸 == applied.scaled_extent(native_size)`（design §State Management）に従う——別フィールドで
        // 物理寸を二重に持つと更新点が 2 つになり、片方だけ書かれる欠陥（本 spec で既出）を招く。両者は
        // 表示成立点で必ず揃って更新されるため、この導出は常に「前回この経路が表示へ載せた物理寸」に一致する
        // （`resample` の事後条件が `出力外形 == scaled_extent(入力外形)` ゆえ `chain.size()` と厳密に等しい）。
        //
        // 前値なし（初回表示）は `None` ≠ `Some(size)` ゆえ**必ず差分扱い**になる。これは意図した設計である
        // ——窓は起動時 k₀ 見積もり寸で生成されており実窓 DPI 由来の k と一致する保証がないため、初回を
        // 黙らせると Flow 3 手順 5 の補正が永久に走らない。
        let prev_physical = target
            .applied
            .zip(target.native_size)
            .map(|(k, (nw, nh))| k.scaled_extent(nw, nh));
        let size_changed = prev_physical != Some(size);
        if size_changed {
            // 差分あり＝呼び手（frame drain フェーズ）へ新物理寸を報告する。同寸のときは**何も触らない**
            // ——`None` を書き戻すと未消費の要求を殺してしまう（取りこぼしを作らない・べき等）。
            target.pending_resize = Some(size);
        }

        target.applied = Some(scale);
        // いま表示に使ったエントリ由来の原寸をそのまま写す（合成した回か否かで分岐しない——分岐させると
        // 「insert 済みのまま失敗 → 後からヒットで成立」の経路で照会値が画面と乖離する）。
        target.native_size = target.cached_native;
        // 合成キーの安定ハッシュ（perf サマリ行の `key_hash`・Requirement 7.2 の裁定材料）は
        // `last_show` への move の**直前**に取る。全段の `mark` が済んだ後なので、この走査
        // （借用のみ・確保なし）は段別所要へ混入せず `t_total_us` にだけ含まれる。
        let key_hash = compose_key_hash(surface_id, &binds, &pattern, scale);
        target.last_show = Some((surface_id, binds, pattern));

        // 表示成立点の観測ログ（設計 D10・要件 6.1/6.3 の判定素材）。実機サインオフは有界 auto-exit で
        // 起動し `RUST_LOG` を grep してここを読むため、**`info!` レベル**であることが契約である
        // （`debug!` へ落とすと既定の観測条件で消える）。k 導出値（`k`・`k_ratio`）と適用寸（`native_*`・
        // `scaled_*`）が揃うことで、2 水準（125%/200%）の実行が「異なる物理寸で描かれた」ことを
        // ログだけで決定論的に判定できる。
        //
        // `native_*` の供給源 `native_size` は直上で `cached_native` から写しており、スロットと対の
        // 不変条件によりこの経路では必ず `Some` である（引き当てが成立した＝スロットに中身がある）。
        // 万一崩れた場合の `0×0` は**実在し得ない外形**（0 外形は上流 `EmptyComposition` が先行遮断する）
        // ゆえ、値を捏造せず「対が壊れた」ことを示す診断番兵として機能する。
        let (native_w, native_h) = target.native_size.unwrap_or((0, 0));
        tracing::info!(
            ?target_id,
            surface_id,
            cache_hit,
            // k の有理表現（既約 num/den）。`ScaleRatio` の num/den は非公開ゆえ `Debug` で出す。
            k_ratio = ?scale,
            k = scale.as_f32(),
            author_dpi = target.policy.author_dpi,
            // `None` は要件 1.4 の縮退（DPI component 不在 → k=1.0）そのものゆえ潰さずに出す。
            window_dpi = ?window_dpi,
            native_w,
            native_h,
            scaled_w = size.0,
            scaled_h = size.1,
            // 今回の表示成立が窓寸 reconcile 要求を積んだか（議題 #2 裁定の状態照合の観測点）。
            size_changed,
            "apply(ShowSurface): 表示・マスクを更新"
        );

        // perf サマリ行（Requirement 1.1/1.3・design.md §Data Models）。上の info! と**同一適用の対**
        // として隣接して出る別行である。ここが唯一の emit 点＝早期復帰の経路では 1 行も出ない。
        //
        // `take_delta` は増分を取り出して器を 0 に戻す。早期復帰で取り出されなかった増分は次の適用へ
        // 持ち越される（budget.rs の契約）——確保は実際に起きているため、成立した次の行に現れるのが
        // 正しく、累積（`cumulative`）は常に厳密である。
        let allocs = target.budget.take_delta();
        timing.emit(
            &EmitContext {
                target_id,
                surface_id,
                cache_hit,
                key_hash,
            },
            allocs,
        );
        Self::reply(reply, Ok(()));
    }
}
