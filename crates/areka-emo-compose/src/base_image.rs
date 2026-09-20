//! 面の土台の絵の決定: ファイル名だけで置かれた面の画像を、面の層 0 として足す。
//!
//! シェルのフォルダ直下に `surface0.png` のように置かれた絵は、`surfaces.txt` に 1 行も
//! 書かれていなくても面の土台になる（ukadoc の慣習・要件 2.1 の表ア〜エ）。「番号 → ファイル名」
//! の対応を決めるのは上流（`areka-emo-present` の `shell_target`）で、本モジュールは決まった
//! 対応を受け取り、**畳み込み（[`crate::fold::fold_shell`]）が終わった後の面の表**に対して
//! 番号ごとに次の判定を下す。
//!
//! | 面が面の表に在る | 層 0 の element が在る | すること | 要件 2.1 の表 |
//! |---|---|---|---|
//! | いいえ | — | 画像 1 枚を層 0 に持つ面を新設する | ア（要件 2.2・3.9） |
//! | はい | いいえ | 画像を層 0・位置 (0,0)・`Overlay` で足し、層の昇順に並べ直す | ア／イ |
//! | はい | はい | 何もしない（画像は使わない）。[`BaseImageReport::shadowed`] に数える | ウ |
//!
//! 画像の無い番号には触らない（エ・変更 0）。畳み込みの**後**に下すので、複数番号の見出し
//! （`surface0,1`）は番号ごとに自分の画像を受け取り、`surface.append` が後から足した層 0 も
//! 「層 0 が在る」に数えられる。層 0 が空いているときだけ足すため、`element0` より下に敷く
//! 新しい層の型は要らない。
//!
//! 画素は持たない（[`crate::EmoWorld`] の不変条件のまま）。本仕様が足す記録（要件 6.1・6.2）は
//! 本モジュールでは出さず、結果を [`BaseImageReport`] で返して上流の権威が 1 か所で出す
//! （design「Monitoring」）。例外は本来生じない不整合（[`SurfaceIndex`] が指すのに
//! [`SurfaceMaster`] が欠ける）の `warn!` 1 本で、[`crate::fold`] の同じ枝と同じ扱いである。

use std::collections::BTreeMap;

use areka_parsers::shell::ElementPath;
use bevy_ecs::prelude::Resource;
use bevy_ecs::world::World;

use crate::method::ComposeMethod;
use crate::normalized::{NormalizedElement, SurfaceMaster, Transform};
use crate::world::{SurfaceId, SurfaceIndex};

/// 面の番号 → 面の画像のファイル名（元の綴りのまま）。
///
/// 読み込みの間は変わらない。面の表の中では畳み込みの前に 1 度だけ置かれ、以後は読むだけである。
/// 読み手は [`crate::fold::fold_append`] の 1 か所で、`surface.append` の対象が波括弧を持たなくても
/// 画像を持つなら「既にある面」と数えるために引く（要件 3.7）。[`apply_base_images`] は引数の
/// 対応をそのまま使うので、ここからは引かない。ファイル名は焼いた絵の索引表のキーと同じ綴りで
/// なければならない（`AtlasTable::resolve` は文字列の完全一致で引く）。
#[derive(Debug, Default, Clone, Resource)]
pub struct SurfaceImages(pub BTreeMap<u32, String>);

/// 土台の絵の決定の結果（どの画像を使い、どれを使わなかったか）。
///
/// 不変条件: `used` と `shadowed` のキーは重ならず、和は渡した画像の全体に等しい。
#[derive(Debug, Default, Clone, PartialEq, Eq, Resource)]
pub struct BaseImageReport {
    /// 層 0 として足した画像（番号 → ファイル名）。
    pub used: BTreeMap<u32, String>,
    /// 使わなかった画像（番号 → ファイル名）。ほぼすべては「層 0 の element が在ったため」だが、
    /// 本来生じない不整合（`SurfaceIndex` が指すのに `SurfaceMaster` が欠ける）で飛ばした画像も
    /// ここへ入る（不変条件「和は渡した画像の全体」を保つため）。その枝は `warn!` を残す。
    pub shadowed: BTreeMap<u32, String>,
}

/// 畳み込み済みの面の表に、番号ごとの土台の絵を適用する（モジュール冒頭の表ア〜エ）。
///
/// 番号の昇順（`BTreeMap` の並び）に走査するので、結果は入力の順に依らず決まる。足す層は
/// 層 0・位置 (0,0)・[`ComposeMethod::Overlay`] で、既存の element の位置は変えない（要件 2.5）。
/// 既存の面へ足したときは層の昇順（安定ソート）へ並べ直し、`SurfaceMaster.elements` の不変条件を保つ。
pub(crate) fn apply_base_images(
    world: &mut World,
    images: &BTreeMap<u32, String>,
) -> BaseImageReport {
    let mut report = BaseImageReport::default();

    for (&id, file) in images {
        let existing = world.resource::<SurfaceIndex>().0.get(&id).copied();
        let Some(entity) = existing else {
            // ア: 宣言の無い番号は、画像 1 枚を層 0 に持つ「存在する面」として新設する（要件 2.2・3.9）。
            let master = SurfaceMaster {
                id,
                elements: vec![base_element(file)],
                collisions: Vec::new(),
                animations: Vec::new(),
            };
            let entity = world.spawn((SurfaceId(id), master)).id();
            world.resource_mut::<SurfaceIndex>().0.insert(id, entity);
            report.used.insert(id, file.clone());
            continue;
        };

        let Some(mut master) = world.get_mut::<SurfaceMaster>(entity) else {
            // SurfaceIndex に載るが component 欠落（本来生じない不整合）。パニックせず観測可能化し、
            // 画像は使わなかったものとして数える（不変条件「和は渡した画像の全体」を保つ）。
            tracing::warn!(
                target: "areka_emo_compose",
                id,
                "SurfaceIndex は指すが SurfaceMaster component が欠落: 面の画像を使わずスキップ"
            );
            report.shadowed.insert(id, file.clone());
            continue;
        };

        if master.elements.iter().any(|e| e.layer == 0) {
            // ウ: `element0` が在る面では画像を使わない（C4・C5・要件 10.3）。
            report.shadowed.insert(id, file.clone());
        } else {
            // ア／イ: 層 0 は空いているので、画像を最も奥へ敷いて層の昇順に並べ直す。
            master.elements.push(base_element(file));
            master.elements.sort_by_key(|e| e.layer);
            report.used.insert(id, file.clone());
        }
    }

    report
}

/// 面の画像 1 枚を表す層 0 の element（位置 (0,0)・`Overlay`・要件 2.5）。
fn base_element(file: &str) -> NormalizedElement {
    NormalizedElement {
        layer: 0,
        path: ElementPath::new(file.to_string()),
        transform: Transform::translate(0, 0),
        method: ComposeMethod::Overlay,
    }
}

#[cfg(test)]
#[path = "base_image_tests.rs"]
mod tests;
