//! シェルのフォルダから面の絵を組み立てる読み込みの入口（`shell_target`）。
//!
//! fs を触るのは [`load_shell_target`]（上限を渡す兄弟 [`load_shell_target_with_limits`]）と
//! [`load_shell_target_quiet`] だけで、触らない核は `build_core`（テストは `build_shell_target`
//! 経由でメモリ上の復号器で通す）。
//!
//! `select_surface_images` は `surface<数字>.png` というファイル名の慣習だけで置かれた絵を、
//! その番号の面の画像として認める。`surfaces.txt` に `element` 行が 1 本も無いシェル
//! （里々・YAYA の標準テンプレート）はこれにすべてを委ねる。名前の判定はバルーン側と同じ
//! `crate::balloon::face_digits_of`（接頭辞 `surface`）。
//!
//! # 記録を出す場所
//!
//! 読み込みの記録を出すのは fs を触る入口 [`load_shell_target`] だけで、読み込み 1 回につき
//! 各 1 度。`select_surface_images`・`build_core`・[`ShellTarget::build_world`] は記録を出さず、
//! 事実を戻り値や報告に載せて返す（装着の記録だけは `build_world` を呼ぶたびに出る）。
//! [`load_shell_target_quiet`]（採寸の読み）は読み込みの記録を出さず、`Err` で終える 3 つの
//! 失敗の `error!` だけを出す。
//!
//! 宛先は既定の target（`areka_emo_present::shell_target`）、文言はスコープの接頭辞 `shell:`
//! で始め、値は構造化フィールドで渡す。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use areka_emo_atlas::{
    AlphaParams, AnimationLimits, AtlasTable, BakeError, ElementDecoder, PackConfig, SetId,
    SurfaceSet, UseSelfAlpha, bake_quiet, bake_with_limits,
};
use areka_emo_compose::{
    BaseImageReport, BoxIssue, BoxLayout, BoxReport, ElementKind, EmoWorld, NestIssue, NestReport,
    ShapeIssue, element_kind, fold_boxes, shape_issues,
};
use areka_parsers::charset::{DefaultEncoding, decode};
use areka_parsers::shell::{
    AppendTarget, BoxDefinition, CursorBrace, DrawMethod, Element, ElementOptions, ElementPath,
    Shell, ShellText, Surface,
};

use crate::balloon::face_digits_of;
use crate::load_records::LoadRecords;

/// シェルの面定義ファイル名（読むのは [`load_shell_target`] の 1 回だけ）。
const SURFACES_TXT: &str = "surfaces.txt";

/// シェルの設定ファイル名（透過の宣言 `seriko.use_self_alpha` を [`load_shell_target`] が読む）。
const DESCRIPT_TXT: &str = "descript.txt";

/// シェルの面画像の接頭辞（大小無視で比較される）。
const SURFACE_PREFIX: &str = "surface";

/// 名前の一覧から決まった「番号 → 採ったファイル名」と、その過程で捨てたもの。
///
/// 3 つの欄はどれも**入力の順に依存しない**。番号は数値の昇順、名前は辞書順である。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct SurfaceImageSelection {
    /// 番号 → 採ったファイル名（**元の綴りのまま**——焼く側が実パスを開くため）。
    pub images: BTreeMap<u32, String>,
    /// 同じ番号に複数あったもの: `(番号, 採った名前, 捨てた名前の一覧)`。番号の昇順。
    pub duplicates: Vec<(u32, String, Vec<String>)>,
    /// 形は面の画像だが、数字が `u32` に収まらなかった名前。辞書順。
    pub overflow: Vec<String>,
}

/// 名前の一覧から「番号 → 採ったファイル名」を決める（fs に触らない純粋な関数）。
///
/// 判定と選択は次のとおり——
///
/// 1. **名前の判定**: [`face_digits_of`] に接頭辞 `surface` を与え、`surface{数字列}.png`
///    （接頭辞・拡張子は大小無視）の数字列を得る。`surfaces.txt`・`surface.png`・
///    `surface+0.png`・`surface0.pna` のように満たさない名前は 0 件（同名の `.pna` は読まない）。
/// 2. **番号の読み**: 数字列を 10 進数として読む。先頭の 0 は無視され、`surface0.png`・
///    `surface00.png`・`surface000.png`・`surface0000.png` はすべて面 0、`surface0010.png` は
///    面 10 になる。`u32` に収まらない数字列は画像と認めず
///    [`SurfaceImageSelection::overflow`] へ送る。
/// 3. **重複の裁き**: 同じ番号に複数の名前が解決したら**ファイル名の辞書順で最小**を採り、
///    残りを [`SurfaceImageSelection::duplicates`] に積む（バルーンの `select_faces` と
///    同じ規則）。フォルダの走査順に結果が左右されないためである。
///
/// 入力はファイル名だけ——フォルダそのものやサブフォルダの中身を除くのは一覧を採る側の
/// 責務であり、本関数は渡された名前の列だけを見る。
pub(crate) fn select_surface_images<S: AsRef<str>>(names: &[S]) -> SurfaceImageSelection {
    // BTreeSet にして、採用（最小）と不採用の並びを入力の順に依存させない。
    let mut candidates: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut overflow: BTreeSet<String> = BTreeSet::new();

    for name in names {
        let name = name.as_ref();
        let Some(digits) = face_digits_of(SURFACE_PREFIX, name) else {
            continue;
        };
        match digits.parse::<u32>() {
            Ok(id) => {
                candidates.entry(id).or_default().insert(name.to_string());
            }
            Err(_) => {
                overflow.insert(name.to_string());
            }
        }
    }

    let mut images: BTreeMap<u32, String> = BTreeMap::new();
    let mut duplicates: Vec<(u32, String, Vec<String>)> = Vec::new();
    for (id, names) in candidates {
        let mut names = names.into_iter();
        let Some(adopted) = names.next() else {
            continue;
        };
        let dropped: Vec<String> = names.collect();
        if !dropped.is_empty() {
            duplicates.push((id, adopted.clone(), dropped));
        }
        images.insert(id, adopted);
    }

    SurfaceImageSelection {
        images,
        duplicates,
        overflow: overflow.into_iter().collect(),
    }
}

/// シェルの読み込みの失敗。
///
/// 呼び手は起動なら `BootWiringError::ShellRead`／`ShellEmpty`、採寸なら
/// `PlacementError::Measure` へ写す。
#[derive(Debug, thiserror::Error)]
pub enum ShellLoadError {
    /// シェルのフォルダの一覧が取れなかった。「画像 0 件」として先へ進まない。
    #[error("shell フォルダの一覧に失敗: {path}")]
    List {
        /// 一覧しようとしたシェルのフォルダ。
        path: PathBuf,
        /// OS が返した理由。
        #[source]
        source: std::io::Error,
    },
    /// `surfaces.txt` が在るのに読めなかった（無いときは失敗にせず画像だけで組む）。
    #[error("surfaces.txt の読み取りに失敗: {path}")]
    Read {
        /// 読もうとした `surfaces.txt` の絶対パス。
        path: PathBuf,
        /// OS が返した理由。
        #[source]
        source: std::io::Error,
    },
    /// シェルに面が 1 つも無い——`surfaces.txt` が無いか面を定義せず、面の画像も無い。
    #[error("シェルに面が 1 つも無い: {path}")]
    Empty {
        /// シェルのフォルダ（`surfaces.txt` が無い場合があるため、そのファイルではない）。
        path: PathBuf,
    },
}

/// 読み込み済みのシェル——焼いた絵と、面の表を必要な数だけ組むのに要るものの一式。
///
/// 面の表（[`EmoWorld`]）は複製できないため、読み込みで組んだ 1 つだけを持ち
/// （[`ShellTarget::take_world`] が 1 度だけ渡す）、他は [`ShellTarget::build_world`] が呼ばれるたびに
/// 組み直す。入力（`surfaces.txt` の解析結果と「番号 → 面の画像」の対応）は読み込みの間ずっと
/// 変わらないので、何度組んでも結果は同じである。
pub struct ShellTarget {
    /// `surfaces.txt` の解析結果（面の表を組むたびに読み直さない）。
    shell: Shell,
    /// 番号 → 面の画像のファイル名（採った綴りのまま）。使わなかった番号も含む全体。
    images: BTreeMap<u32, String>,
    /// 焼いた絵の索引表（`element` が名指しする絵＋土台に**使った**面の画像）。
    atlas: AtlasTable,
    /// 焼く段で落ちた絵（記録を出すのは [`load_shell_target`]）。
    bake_errors: Vec<BakeError>,
    /// 土台の絵の決定の結果。`build_core` が組んだ面の表から写して持つ（記録のために組み直さない）。
    base_images: BaseImageReport,
    /// 相手の面が存在しないコマの「(コマを持つ面, 相手の番号)」。
    dangling: BTreeSet<(u32, u32)>,
    /// 箱の定義の表とサーフェス番号ごとの置き場所の表。
    boxes: BoxLayout,
    /// 箱の畳み込みで読み捨てた事実（記録を出すのは [`load_shell_target`]）。
    box_report: BoxReport,
    /// 箱の置き場所を持つサーフェスの数（[`load_shell_target`] の `info!` の出どころ）。
    box_surfaces: usize,
    /// 入れ子の無い番号と循環の報告（記録を出すのは [`load_shell_target`]）。
    nest_report: NestReport,
    /// 焼いた絵のうち同じ名前の `.pna` が添えてあったものの数（`.pna` は使わない）。
    ignored_pna: usize,
    /// cursorブレスの転記（検証は下流）。
    cursor_braces: Vec<CursorBrace>,
    /// 透過の宣言（面の絵を焼いたのと同じ値・箱の矢印の絵も同じ扱いで読む）。
    use_self_alpha: UseSelfAlpha,
    /// 読み込みで組んだ面の表（atlas は未装着・[`ShellTarget::take_world`] が 1 度だけ渡す）。
    world: Option<EmoWorld>,
    /// `region` の画像のマスクの集まり（組む面の表すべてが同じ実体を分け合う）。
    region_masks: std::sync::Arc<areka_emo_compose::RegionMasks>,
}

// `EmoWorld` は `Debug` を持たないので手で書く（面の表は有無だけを出す）。
impl std::fmt::Debug for ShellTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellTarget")
            .field("shell", &self.shell)
            .field("images", &self.images)
            .field("atlas", &self.atlas)
            .field("bake_errors", &self.bake_errors)
            .field("base_images", &self.base_images)
            .field("dangling", &self.dangling)
            .field("boxes", &self.boxes)
            .field("box_report", &self.box_report)
            .field("box_surfaces", &self.box_surfaces)
            .field("nest_report", &self.nest_report)
            .field("ignored_pna", &self.ignored_pna)
            .field("cursor_braces", &self.cursor_braces)
            .field("use_self_alpha", &self.use_self_alpha)
            .field("world", &self.world.as_ref().map(|_| "EmoWorld"))
            .field("region_masks", &self.region_masks.unreadable())
            .finish()
    }
}

impl ShellTarget {
    /// 焼いた絵の索引表（面の表へ装着する正本）。
    pub fn atlas(&self) -> &AtlasTable {
        &self.atlas
    }

    /// 焼く段で落ちた絵の一覧（`emo2` では 0 件）。
    pub fn bake_errors(&self) -> &[BakeError] {
        &self.bake_errors
    }

    /// 箱の定義の表とサーフェス番号ごとの置き場所の表（箱の無いシェルでは空）。
    pub fn boxes(&self) -> &BoxLayout {
        &self.boxes
    }

    /// cursorブレスの転記（cursorブレスの無いシェルでは空）。
    pub fn cursor_braces(&self) -> &[CursorBrace] {
        &self.cursor_braces
    }

    /// 透過の宣言（シェルの `seriko.use_self_alpha`・面の絵を焼いたのと同じ値）。
    pub fn use_self_alpha(&self) -> UseSelfAlpha {
        self.use_self_alpha
    }

    /// 面の表を 1 つ組む。スコープの数だけ呼んでよく、内容は毎回同じ。
    ///
    /// 読み込みの記録は出さない（装着の記録だけが組むたびに出る）。
    pub fn build_world(&self) -> EmoWorld {
        let mut world = EmoWorld::build_with_images_quiet(&self.shell, &self.images);
        world.bind_atlas(&self.atlas, SetId(0));
        world.set_region_masks(self.region_masks.clone());
        world
    }

    /// 1 度目は読み込みで組んだ面の表に atlas を装着して渡す。2 度目からは [`ShellTarget::build_world`] と同じ。
    pub fn take_world(&mut self) -> EmoWorld {
        match self.world.take() {
            Some(mut world) => {
                world.bind_atlas(&self.atlas, SetId(0));
                world
            }
            None => self.build_world(),
        }
    }
}

/// シェルのフォルダを読んで [`ShellTarget`] を作る（fs を触る入口・記録を黙らせた版は [`load_shell_target_quiet`]）。
///
/// 焼くのを最後に回すのは、どの面の画像を土台に**使う**かが面の表を組んでみないと決まらない
/// ため（[`EmoWorld`] の構築は焼いた結果を要しない）。
///
/// 文字コードは `surfaces.txt` 自身の `charset,<名前>` 宣言に従い、未宣言は既定（Ansi）。
///
/// # Errors
///
/// フォルダの一覧が取れない（[`ShellLoadError::List`]）・`surfaces.txt` が在るのに読めない
/// （[`ShellLoadError::Read`]）・面が 1 つも無い（[`ShellLoadError::Empty`]）。
/// 一覧の中の 1 件だけが取れない場合は、その 1 件を飛ばして続行する（失敗にしない）。
///
/// `surfaces.txt` が無い、または面を 1 つも定義しないときは、面の画像が 1 つ以上在れば画像だけで
/// 面を組み、そのことと面の数を `info!` で 1 行残す。
///
/// # 記録
///
/// 記録はすべて本関数が出す（モジュール冒頭「記録を出す場所」）。3 つの失敗はいずれも `error!` を伴う。
pub fn load_shell_target(
    shell_dir: &Path,
    decoder: &impl ElementDecoder,
) -> Result<ShellTarget, ShellLoadError> {
    load_shell_target_with_limits(shell_dir, decoder, AnimationLimits::from_env())
}

/// [`load_shell_target`] の動く絵の上限を外から渡す版（記録も失敗も同じ・上限だけが違う）。
/// 環境変数に左右されないテストのための口。
///
/// # Errors
///
/// [`load_shell_target`] と同じ 3 つ。
pub fn load_shell_target_with_limits(
    shell_dir: &Path,
    decoder: &impl ElementDecoder,
    limits: AnimationLimits,
) -> Result<ShellTarget, ShellLoadError> {
    load_shell_target_with(shell_dir, decoder, LoadRecords::Emit, limits)
}

/// [`load_shell_target`] と同じ中身で読み、読み込みの記録を出さない（採寸の読み）。戻り値は [`load_shell_target`] と等しい。
///
/// # Errors
///
/// [`load_shell_target`] と同じ 3 つ。読み込みを `Err` で終えるその失敗の `error!` だけは出す。
pub fn load_shell_target_quiet(
    shell_dir: &Path,
    decoder: &impl ElementDecoder,
) -> Result<ShellTarget, ShellLoadError> {
    // 採寸の読み手は動く絵の道に入らないので、上限は使われない（`bake_quiet` が環境から取る）。
    load_shell_target_with(
        shell_dir,
        decoder,
        LoadRecords::Quiet,
        AnimationLimits::from_env(),
    )
}

/// 読み込みの中身（1 本）。読み込みの記録は `records` が出すときだけ出し、3 つの失敗の
/// `error!` は `records` に関わらず出す。
fn load_shell_target_with(
    shell_dir: &Path,
    decoder: &impl ElementDecoder,
    records: LoadRecords,
    limits: AnimationLimits,
) -> Result<ShellTarget, ShellLoadError> {
    let names = list_file_names(shell_dir, records)?;
    let selection = select_surface_images(&names);

    // 重複と桁溢れは `build_core` へ渡すと消えるので、渡す前に記録する。
    if records.emits() {
        for (id, adopted, dropped) in &selection.duplicates {
            tracing::warn!(
                surface_id = *id,
                adopted = adopted.as_str(),
                dropped = ?dropped,
                "shell: 同じ番号へ解決する面の画像が複数在る（ファイル名の辞書順で最小を採用・R1.5）"
            );
        }
        for file in &selection.overflow {
            tracing::debug!(
                file = file.as_str(),
                "shell: 数字が面の番号として大きすぎるので面の画像と認めない（R1.6）"
            );
        }
    }

    let surfaces_path = shell_dir.join(SURFACES_TXT);
    let (content, surfaces_txt) = match std::fs::read(&surfaces_path) {
        Ok(bytes) => (decode(&bytes, DefaultEncoding::Ansi), "empty"),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => (String::new(), "missing"),
        Err(source) => {
            tracing::error!(
                path = %surfaces_path.display(),
                error = %source,
                "shell: surfaces.txt の読み取りに失敗"
            );
            return Err(ShellLoadError::Read {
                path: surfaces_path,
                source,
            });
        }
    };
    let ShellText {
        shell,
        boxes,
        undrawn,
        cursor_braces,
        issues,
    } = areka_parsers::shell::parse_all(&content);
    let image_only = shell.surfaces.is_empty();
    if image_only && selection.images.is_empty() {
        tracing::error!(
            shell_dir = %shell_dir.display(),
            surfaces_txt,
            "shell: シェルに面が 1 つも無い（surfaces.txt が面を定義せず、面の画像も無い）"
        );
        return Err(ShellLoadError::Empty {
            path: shell_dir.to_path_buf(),
        });
    }

    // 昔からのシェルの descript.txt は Shift_JIS が普通なので、UTF-8 として読まず ANSI を既定にする。
    let descript_path = shell_dir.join(DESCRIPT_TXT);
    let descript = match std::fs::read(&descript_path) {
        Ok(bytes) => Some(decode(&bytes, DefaultEncoding::Ansi)),
        Err(error) => {
            if records.emits() {
                tracing::warn!(
                    path = %descript_path.display(),
                    error = %error,
                    "shell: descript.txt が読めないので透過の宣言なしとして続ける"
                );
            }
            None
        }
    };
    let use_self_alpha = crate::self_alpha::read_use_self_alpha(
        "shell",
        "seriko.use_self_alpha",
        shell_dir,
        descript.as_deref(),
        records,
    );

    let mut target = build_core(
        shell,
        &boxes,
        selection,
        shell_dir,
        decoder,
        use_self_alpha,
        records,
        limits,
    );
    target.cursor_braces = cursor_braces;

    if records.emits() {
        for issue in &issues {
            if issue.severity() == tracing::Level::ERROR {
                tracing::error!("{}", issue.message());
            } else {
                tracing::warn!("{}", issue.message());
            }
        }
        if target.ignored_pna != 0 {
            tracing::warn!(
                shell_dir = %shell_dir.display(),
                ignored_pna = target.ignored_pna,
                "shell: 絵に添えてある .pna を使わずに表示する"
            );
        }

        for (&surface_id, file) in &target.base_images.shadowed {
            tracing::debug!(
                surface_id,
                file = file.as_str(),
                "shell: element0 が在るため面の画像を土台に使わなかった（R6.2）"
            );
        }
        for &(surface_id, dangling_target) in &target.dangling {
            tracing::warn!(
                surface_id,
                target = dangling_target,
                "shell: コマの相手の面が宣言も画像も無い（描かずに続行・R3.5）"
            );
        }
        for error in &target.bake_errors {
            tracing::error!(error = %error, "shell: shell bake で脱落した element");
        }
        for issue in &target.nest_report.issues {
            log_nest_issue(issue);
        }
        if let Some(world) = &target.world {
            log_shape_issues(&shape_issues(world));
        }
        for issue in &target.box_report.issues {
            log_box_issue(issue);
        }
        for line in &undrawn {
            tracing::warn!(
                heading = line.heading.as_str(),
                element = line.element.as_str(),
                method = line.method.as_str(),
                "shell: areka が描けない描画メソッドの element定義を描かない"
            );
        }
        if !target.boxes.is_empty() {
            tracing::info!(
                braces = boxes
                    .iter()
                    .filter(|d| matches!(d, BoxDefinition::Brace(_)))
                    .count(),
                surfaces = target.box_surfaces,
                "shell: 箱を読んだ（書かれた balloon.*ブレスの数・箱の置き場所を持つサーフェスの数）"
            );
        }
        if image_only {
            tracing::info!(
                shell_dir = %shell_dir.display(),
                surfaces_txt,
                surfaces = target.images.len(),
                "shell: surfaces.txt が面を定義しないので、面の画像だけで面を組んだ（要件 7.3）"
            );
        }
        tracing::info!(
            shell_dir = %shell_dir.display(),
            recognized = target.images.len(),
            used = target.base_images.used.len(),
            shadowed = target.base_images.shadowed.len(),
            "shell: シェルの面の画像の一覧が終わった"
        );
    }

    Ok(target)
}

/// fs を触らない核（復号器を除く）: 面の表を組んで使う画像を決め、焼いて [`ShellTarget`] にする。
///
/// 焼く絵の一覧は「`shell.surfaces` の複製＋**使う**画像 1 枚につき `element0` だけを持つ
/// 面 1 個」（[`base_image_surface`]）。使わなかった画像は焼かない（索引表に載らない）。
#[cfg(test)] // 呼び手はテストだけ（本番の入口は `build_core` を直に呼ぶ）。
pub(crate) fn build_shell_target(
    shell: Shell,
    selection: SurfaceImageSelection,
    shell_dir: &Path,
    decoder: &impl ElementDecoder,
    use_self_alpha: UseSelfAlpha,
) -> ShellTarget {
    build_shell_target_with_boxes(shell, &[], selection, shell_dir, decoder, use_self_alpha)
}

/// `build_shell_target` に箱の転記（[`parse_boxes`](areka_parsers::shell::parse_boxes) の結果）を足した fs を触らない核。
///
/// 箱の表は [`fold_boxes`] が作り、読み捨てた事実と入れ子の報告（[`EmoWorld::nest_report`]）は
/// [`ShellTarget`] に載せるだけで記録は出さない。焼く一覧は画像でない element（数字だけの欄）を外す。
#[cfg(test)] // 呼び手はテストだけ（本番の入口は `build_core` を直に呼ぶ）。
pub(crate) fn build_shell_target_with_boxes(
    shell: Shell,
    boxes: &[BoxDefinition],
    selection: SurfaceImageSelection,
    shell_dir: &Path,
    decoder: &impl ElementDecoder,
    use_self_alpha: UseSelfAlpha,
) -> ShellTarget {
    build_core(
        shell,
        boxes,
        selection,
        shell_dir,
        decoder,
        use_self_alpha,
        LoadRecords::Emit,
        AnimationLimits::from_env(),
    )
}

/// 核の中身（1 本）。`records` は焼く段と面の表の記録を出すかどうかだけを切り替える。
#[allow(clippy::too_many_arguments)] // 中の核 1 本の引数（上限を足して 8 つ）。束ねる型は作らない。
fn build_core(
    shell: Shell,
    boxes: &[BoxDefinition],
    selection: SurfaceImageSelection,
    shell_dir: &Path,
    decoder: &impl ElementDecoder,
    use_self_alpha: UseSelfAlpha,
    records: LoadRecords,
    limits: AnimationLimits,
) -> ShellTarget {
    let images = selection.images;

    // 土台に使う画像は面の表を組んで初めて決まる（`element0` が在る面では使わない）。
    // この表は捨てずに持ち、`take_world` が 1 度渡す。
    let mut probe = if records.emits() {
        EmoWorld::build_with_images(&shell, &images)
    } else {
        EmoWorld::build_with_images_quiet(&shell, &images)
    };
    let base_images = probe.base_images().clone();
    let dangling = probe.dangling_pattern_targets();
    let (box_layout, box_report) = fold_boxes(boxes, &images, &probe);
    let box_surfaces = probe
        .surface_ids()
        .filter(|&id| !box_layout.placements(id).is_empty())
        .count();
    let nest_report = probe.nest_report();
    let region_masks = regions::load_region_masks(&probe, shell_dir, decoder, records);
    probe.set_region_masks(region_masks.clone());

    // 数字だけの element定義はサーフェスを置くもので画像ではない。焼く一覧に入れると
    // その名前の絵を読みに行って脱落の `error!` になるので外す。
    let mut surfaces = shell.surfaces.clone();
    for surface in &mut surfaces {
        surface
            .elements
            .retain(|e| element_kind(&e.path) == ElementKind::Image);
    }
    surfaces.extend(
        base_images
            .used
            .iter()
            .map(|(&id, file)| base_image_surface(id, file)),
    );

    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: shell_dir,
        alpha_params: AlphaParams { use_self_alpha },
    };
    let sets = std::slice::from_ref(&set);
    let baked = if records.emits() {
        bake_with_limits(sets, decoder, PackConfig::default(), limits)
    } else {
        bake_quiet(sets, decoder, PackConfig::default())
    };

    ShellTarget {
        shell,
        images,
        atlas: baked.table,
        bake_errors: baked.errors,
        base_images,
        dangling,
        boxes: box_layout,
        box_report,
        box_surfaces,
        nest_report,
        ignored_pna: baked.ignored_pna,
        cursor_braces: Vec::new(),
        use_self_alpha,
        world: Some(probe),
        region_masks,
    }
}

/// 入れ子の報告の 1 件を `warn!` 1 行にする（親・element・指した番号）。
fn log_nest_issue(issue: &NestIssue) {
    match issue {
        NestIssue::MissingTarget {
            surface,
            element,
            target,
        } => tracing::warn!(
            surface = *surface,
            element = *element,
            target = target.as_str(),
            "shell: element定義が指したサーフェスが無いので置かない"
        ),
        NestIssue::Cycle {
            surface,
            element,
            target,
        } => tracing::warn!(
            surface = *surface,
            element = *element,
            target = *target,
            "shell: element定義の参照が循環するので、先祖へ戻る参照を置かない"
        ),
    }
}

/// 形の誤り（pattern定義の辺の循環・描かないメソッドの pattern・知らない sort）を 1 件 1 行の
/// `warn!` にする。描かないメソッドの pattern と同じ pattern の循環は、描かない 1 行に寄せる。
fn log_shape_issues(issues: &[ShapeIssue]) {
    let undrawn: BTreeSet<(u32, u32, u32)> = issues
        .iter()
        .filter_map(|i| match i {
            ShapeIssue::UndrawnPattern {
                surface,
                animation,
                pattern,
                ..
            } => Some((*surface, *animation, *pattern)),
            _ => None,
        })
        .collect();
    for issue in issues {
        match issue {
            ShapeIssue::UnknownAnimationSort { value } => tracing::warn!(
                value = value.as_str(),
                "shell: animation-sort の値が ascend／descend でないので descend として組む"
            ),
            ShapeIssue::Cycle {
                surface,
                animation,
                pattern,
                target,
            } => {
                if !undrawn.contains(&(*surface, *animation, *pattern)) {
                    tracing::warn!(
                        surface_id = *surface,
                        animation_id = *animation,
                        pattern = *pattern,
                        target = *target,
                        "shell: pattern の参照が循環するので、先祖へ戻る参照を置かない"
                    );
                }
            }
            ShapeIssue::UndrawnPattern {
                surface,
                animation,
                pattern,
                method,
            } => tracing::warn!(
                surface_id = *surface,
                animation_id = *animation,
                pattern = *pattern,
                method = method.as_str(),
                "shell: areka が描けない描画メソッドの pattern を描かない"
            ),
        }
    }
}

/// 箱の報告の 1 件を `warn!` 1 行にする（欄は報告の対象の欄そのまま）。
fn log_box_issue(issue: &BoxIssue) {
    match issue {
        BoxIssue::BraceEmptyName { heading } => tracing::warn!(
            heading = heading.as_str(),
            "shell: 名前が空の balloon.*ブレスを採らない"
        ),
        BoxIssue::BraceMissingSize { name } => tracing::warn!(
            name = name.as_str(),
            "shell: size の無い balloon.*ブレスを採らない"
        ),
        BoxIssue::BraceBadSize { name, value } => tracing::warn!(
            name = name.as_str(),
            value = value.as_str(),
            "shell: size が正の整数 2 つとして読めない balloon.*ブレスを採らない"
        ),
        BoxIssue::BraceNumericName { name } => tracing::warn!(
            name = name.as_str(),
            "shell: 名前が整数として読める balloon.*ブレスを採らない"
        ),
        BoxIssue::BraceReplaced { name } => tracing::warn!(
            name = name.as_str(),
            "shell: 同じ名前の balloon.*ブレスを後のもので置き換えた"
        ),
        BoxIssue::BraceKeyIgnored { name, key } => tracing::warn!(
            name = name.as_str(),
            key = key.as_str(),
            "shell: 箱に当てはまらないキーを読み捨てた"
        ),
        BoxIssue::BraceBadFollow { name, value } => tracing::warn!(
            name = name.as_str(),
            value = value.as_str(),
            "shell: font.follow の値が不正（scope として扱う）"
        ),
        BoxIssue::ElementBadNumber { surface, element } => tracing::warn!(
            surface = *surface,
            element = element.as_str(),
            "shell: element番号が読めない箱の element定義を読み捨てた"
        ),
        BoxIssue::ElementUnknownBrace {
            surface,
            element,
            name,
        } => tracing::warn!(
            surface = *surface,
            element = *element,
            name = name.as_str(),
            "shell: 名前の balloon.*ブレスが無い箱の element定義を読み捨てた"
        ),
        BoxIssue::ElementBadPosition {
            surface,
            element,
            name,
            x,
            y,
        } => tracing::warn!(
            surface = *surface,
            element = *element,
            name = name.as_str(),
            x = x.as_str(),
            y = y.as_str(),
            "shell: X・Y が整数として読めない箱の element定義を読み捨てた"
        ),
        BoxIssue::ElementDuplicateName {
            surface,
            element,
            name,
            kept,
        } => tracing::warn!(
            surface = *surface,
            element = *element,
            name = name.as_str(),
            kept = *kept,
            "shell: 同じ名前の箱の element定義のうち element番号が最小でないものを読み捨てた"
        ),
        BoxIssue::ElementBelowImage {
            surface,
            element,
            name,
            image_element,
        } => tracing::warn!(
            surface = *surface,
            element = *element,
            name = name.as_str(),
            image_element = *image_element,
            "shell: 箱の element番号が画像の element番号より小さい（箱は画像より手前に描く）"
        ),
        BoxIssue::AppendTargetMissing { surface, name } => tracing::warn!(
            surface = *surface,
            name = name.as_str(),
            "shell: surface.append*ブレスの追記先のサーフェスが無いので箱の element定義を読み捨てた"
        ),
        BoxIssue::InChildSurface {
            parent,
            child,
            name,
        } => tracing::warn!(
            parent = *parent,
            child = *child,
            name = name.as_str(),
            "shell: 子として置かれたサーフェスの箱は親の中に置かない"
        ),
    }
}

/// シェルのフォルダ**直下**の**ファイル**の名前を 1 回の走査で集める。
///
/// フォルダそのものとサブフォルダの中身は 0 件である（`file_type().is_file()`）。非 UTF-8 の
/// 名前は面の画像の名前の規約（ASCII の接頭辞＋数字＋拡張子）を満たし得ないのでここで落とす。
/// 1 件ごとの取得失敗（種別が読めない場合を含む）は致命ではなく、その 1 件を飛ばして続ける。
///
/// バルーンの `enumerate_file_names` を流用しないのは、失敗の型がバルーン専用
/// （`BalloonLoadError`）で、フォルダを除かないためである。
fn list_file_names(shell_dir: &Path, records: LoadRecords) -> Result<Vec<String>, ShellLoadError> {
    let read_dir = std::fs::read_dir(shell_dir).map_err(|source| {
        tracing::error!(
            shell_dir = %shell_dir.display(),
            error = %source,
            "shell: シェルのフォルダの一覧に失敗（画像 0 件として先へ進まない・R1.7）"
        );
        ShellLoadError::List {
            path: shell_dir.to_path_buf(),
            source,
        }
    })?;

    let mut names: Vec<String> = Vec::new();
    for entry in read_dir {
        // 1 件の取得失敗・種別の読み取り失敗は致命ではない（その 1 件を飛ばして続行）。
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                if records.emits() {
                    tracing::warn!(
                        shell_dir = %shell_dir.display(),
                        error = %error,
                        "shell: フォルダの 1 件の取得に失敗（その 1 件を飛ばして続行）"
                    );
                }
                continue;
            }
        };
        match entry.file_type() {
            Ok(file_type) if file_type.is_file() => {}
            // フォルダ等は失敗ではないので記録しない。
            Ok(_) => continue,
            Err(error) => {
                if records.emits() {
                    tracing::warn!(
                        file = %entry.file_name().to_string_lossy(),
                        error = %error,
                        "shell: フォルダの 1 件の種別の読み取りに失敗（その 1 件を飛ばして続行）"
                    );
                }
                continue;
            }
        }
        if let Some(name) = entry.file_name().to_str() {
            names.push(name.to_string());
        }
    }
    Ok(names)
}

/// 面の画像 1 枚だけを持つ合成 `Surface`（焼く絵の一覧へ足す入力）。
///
/// パスの綴りは採ったファイル名そのままで、面の表の層 0 の [`ElementPath`] と完全一致する。
/// `AtlasTable::resolve` は文字列の完全一致で引くので、ここがずれると絵は記録も無く描かれない。
fn base_image_surface(id: u32, file: &str) -> Surface {
    Surface {
        targets: vec![AppendTarget::Single(id)],
        elements: vec![Element {
            layer: 0,
            path: ElementPath::new(file.to_string()),
            x: 0,
            y: 0,
            method: DrawMethod::new("overlay".to_string()),
            options: ElementOptions::new(String::new()),
        }],
        collisions: Vec::new(),
        animations: Vec::new(),
    }
}

#[path = "shell_target_regions.rs"]
mod regions;

#[cfg(test)]
#[path = "shell_target_test_support.rs"]
pub(crate) mod test_support;

#[cfg(test)]
#[path = "shell_target_names_tests.rs"]
mod names_tests;

#[cfg(test)]
#[path = "shell_target_load_tests.rs"]
mod load_tests;

#[cfg(test)]
#[path = "shell_target_base_image_tests.rs"]
mod base_image_tests;

#[cfg(test)]
#[path = "shell_target_template_tests.rs"]
mod template_tests;

#[cfg(test)]
#[path = "shell_target_emo2_tests.rs"]
mod emo2_tests;

#[cfg(test)]
#[path = "shell_target_boxes_tests.rs"]
mod boxes_tests;

#[cfg(test)]
#[path = "shell_target_nesting_tests.rs"]
mod nesting_tests;

#[cfg(test)]
#[path = "shell_target_animated_fixture_tests.rs"]
mod animated_fixture_tests;

#[cfg(test)]
#[path = "shell_target_image_only_tests.rs"]
mod image_only_tests;

#[cfg(test)]
#[path = "shell_target_element_base_tests.rs"]
mod element_base_tests;

#[cfg(test)]
#[path = "shell_target_shape_issues_tests.rs"]
mod shape_issues_tests;

#[cfg(test)]
#[path = "shell_target_quiet_tests.rs"]
mod quiet_tests;
