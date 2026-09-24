use super::{
    AtlasTable, BindSet, CommandSender, Composer, CycleState, EmoBoot, EmoPresenter, EmoWorld,
    Entity, PatternState, SHELL_INITIAL_X, SHELL_INITIAL_Y, WicDecoderArm, World,
    build_balloon_target, compute_balloon_pos, create_balloon_window, create_shell_window, emo2,
    emo2_balloon, load_shell_target,
};

// ---------------------------------------------------------------------------
// Async Setup（UI スレッドで適用されるコマンド）
// ---------------------------------------------------------------------------

/// 起動セットアップコマンドを UI スレッドへ送る。
///
/// 送信するクロージャ本体は UI スレッド（MTA・COM 初期化済み）で実行されるため、その中で
/// `WicDecoderArm`（COM 必要）を生成し実 PNG をデコードしてアセットを組める。クロージャは
/// `Send` 境界（`BoxedCommand`）を満たすが、`!Send` な `EmoPresenter` はクロージャ本体内の
/// ローカルとして生成し `insert_non_send` で World へ載せる（キャプチャしない）。
pub(super) async fn run_setup(tx: CommandSender) {
    let _ = tx.send(Box::new(|world: &mut World| {
        build_and_spawn(world);
    }));
}

/// アセット構築・窓生成・presenter 生成・`EmoBoot` 挿入を一括で行う（UI スレッド）。
fn build_and_spawn(world: &mut World) {
    // 実 WIC デコーダ（COM 初期化済み UI スレッドで生成）。実 PNG を復号する（MemoryDecoder は test 専用）。
    let decoder = match WicDecoderArm::new() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = ?e, "emo-present: WicDecoderArm 生成に失敗（COM 未初期化？）— 中止");
            return;
        }
    };

    // シェル・バルーンのアセットを**本番と同一経路**で構築（シェルは読み込みの権威を 1 回）。
    let shell = load_shell_assets(&decoder);
    let balloon = build_balloon_assets(&decoder);

    // どちらも構築できなければ表示する窓が無い（log-first・誤成功なし）。
    if shell.is_none() && balloon.is_none() {
        tracing::error!("emo-present: シェル・バルーンのアセット構築が双方失敗 — 窓を生成しない");
        return;
    }

    // presenter は生成のみ（attach/apply は GPU 資源到達後に boot_present_system が駆動する）。
    let presenter = EmoPresenter::new();

    let mut boot = EmoBoot {
        presenter,
        shell_window: Entity::PLACEHOLDER,
        balloon_window: Entity::PLACEHOLDER,
        shell_assets: None,
        balloon_assets: None,
        attached: false,
        shell_cycling: false,
        cycle_state: CycleState::EyesOpen,
        next_switch_at: 0.0,
    };

    // シェル窓（surface 原寸で採寸・物理 px）。
    if let Some((emo_world, atlas, w, h)) = shell {
        boot.shell_window = create_shell_window(world, SHELL_INITIAL_X, SHELL_INITIAL_Y, w, h);
        boot.shell_assets = Some((emo_world, atlas));
        // バルーンはアンカーオフセット（R5.4）で配置する。descript に offsetx/offsety があれば
        // それを既定基準からの調整として適用し、無指定なら既定整列（バルーン右端＝シェル左端・
        // 上端揃え）を算出する（emo2 fixture は無指定ゆえ後者が実際に走る）。
        if let Some((b_world, b_atlas, bw, bh)) = balloon {
            let (balloon_x, balloon_y) = compute_balloon_pos(SHELL_INITIAL_X, SHELL_INITIAL_Y, bw);
            boot.balloon_window = create_balloon_window(world, balloon_x, balloon_y, bw, bh);
            boot.balloon_assets = Some((b_world, b_atlas));
        }
    } else if let Some((b_world, b_atlas, bw, bh)) = balloon {
        // シェル無しでもバルーンだけは表示する（degrade・log は load_shell_assets 側で出済み）。
        boot.balloon_window =
            create_balloon_window(world, SHELL_INITIAL_X, SHELL_INITIAL_Y, bw, bh);
        boot.balloon_assets = Some((b_world, b_atlas));
    }

    world.insert_non_send(boot);
    tracing::info!("emo-present: 窓生成とアセット構築を完了（GPU 資源到達で表示を装着）");
}

/// シェル surface（emo2）を**シェル読み込みの権威**（`load_shell_target`）で構築し、
/// surface0 の合成外形（物理 px）を添えて返す。失敗時は log-first で `None`。
///
/// 一覧・`surfaces.txt` の読取と解析・面の画像の決定・焼くまでは権威が 1 回で行う
/// （`areka_emo_present::shell_target`）。文字コードの扱い（`charset` 宣言に従う）・焼く段で
/// 落ちた絵の `warn!` も権威側が持つので、この example は呼ぶだけである——本番（`build_boot_assets`）・
/// 採寸（`build_shell_assets`）と同じ入口を通るため、この example が見る絵は本番と食い違わない。
fn load_shell_assets(decoder: &WicDecoderArm) -> Option<(EmoWorld, AtlasTable, u32, u32)> {
    let base = emo2("shell/master");
    let target = match load_shell_target(&base, decoder) {
        Ok(t) => t,
        Err(e) => {
            // load_shell_target は内部で error! 済み（一覧失敗／読取失敗／面 0 個）。文脈のみ添える。
            tracing::error!(
                dir = %base.display(),
                error = %e,
                "emo-present: shell の読み込みに失敗"
            );
            return None;
        }
    };
    let atlas = target.atlas().clone();
    let emo_world = target.build_world();

    // surface0 を一度合成して窓の物理 px 外形を得る（DPI 表示契約: 窓クライアント寸 ≔ surface 原寸）。
    let (w, h) = match Composer::new().compose(
        &emo_world,
        &atlas,
        0,
        &BindSet::default(),
        &PatternState::default(),
    ) {
        Ok(cs) => (cs.width(), cs.height()),
        Err(e) => {
            tracing::error!(error = %e, "emo-present: shell surface0 の採寸合成に失敗");
            return None;
        }
    };
    if w == 0 || h == 0 {
        tracing::error!(w, h, "emo-present: shell surface0 の合成外形が 0 寸");
        return None;
    }
    Some((emo_world, atlas, w, h))
}

/// バルーン枠（`balloons0.png`）を `build_balloon_target`（シェルと同一経路）で構築し、
/// surface 0 の合成外形（物理 px）を添えて返す。失敗時は log-first で `None`。
fn build_balloon_assets(decoder: &WicDecoderArm) -> Option<(EmoWorld, AtlasTable, u32, u32)> {
    let dir = emo2_balloon();
    let (emo_world, atlas) = match build_balloon_target(&dir, decoder, 0) {
        Ok(pair) => pair,
        Err(e) => {
            // build_balloon_target は内部で error! 済み（枠なし／bake 脱落）。ここは文脈を添えるのみ。
            tracing::error!(dir = %dir.display(), error = %e, "emo-present: バルーン target 構築に失敗");
            return None;
        }
    };

    let (w, h) = match Composer::new().compose(
        &emo_world,
        &atlas,
        0,
        &BindSet::default(),
        &PatternState::default(),
    ) {
        Ok(cs) => (cs.width(), cs.height()),
        Err(e) => {
            tracing::error!(error = %e, "emo-present: balloon surface0 の採寸合成に失敗");
            return None;
        }
    };
    if w == 0 || h == 0 {
        tracing::error!(w, h, "emo-present: balloon surface0 の合成外形が 0 寸");
        return None;
    }
    Some((emo_world, atlas, w, h))
}
