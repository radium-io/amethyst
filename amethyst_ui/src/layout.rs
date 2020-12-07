use glyph_brush::{HorizontalAlign, VerticalAlign};
use serde::{Deserialize, Serialize};

#[cfg(feature = "profiler")]
use thread_profiler::profile_scope;

use amethyst_core::{ecs::*, transform::Parent};
use amethyst_window::ScreenDimensions;

use super::UiTransform;

/// Indicates if the position and margins should be calculated in pixel or
/// relative to their parent size.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub enum ScaleMode {
    /// Use directly the pixel value.
    Pixel,
    /// Use a proportion (%) of the parent's dimensions (or screen, if there is no parent).
    Percent,
}

/// Indicated where the anchor is, relative to the parent (or to the screen, if there is no parent).
/// Follow a normal english Y,X naming.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
pub enum Anchor {
    /// Anchors the entity at the top left of the parent.
    TopLeft,
    /// Anchors the entity at the top middle of the parent.
    TopMiddle,
    /// Anchors the entity at the top right of the parent.
    TopRight,
    /// Anchors the entity at the middle left of the parent.
    MiddleLeft,
    /// Anchors the entity at the center of the parent.
    Middle,
    /// Anchors the entity at the middle right of the parent.
    MiddleRight,
    /// Anchors the entity at the bottom left of the parent.
    BottomLeft,
    /// Anchors the entity at the bottom middle of the parent.
    BottomMiddle,
    /// Anchors the entity at the bottom right of the parent.
    BottomRight,
}

impl Anchor {
    /// Returns the normalized offset using the `Anchor` setting.
    /// The normalized offset is a [-0.5,0.5] value
    /// indicating the relative offset multiplier from the parent's position (centered).
    pub fn norm_offset(self) -> (f32, f32) {
        match self {
            Anchor::TopLeft => (-0.5, 0.5),
            Anchor::TopMiddle => (0.0, 0.5),
            Anchor::TopRight => (0.5, 0.5),
            Anchor::MiddleLeft => (-0.5, 0.0),
            Anchor::Middle => (0.0, 0.0),
            Anchor::MiddleRight => (0.5, 0.0),
            Anchor::BottomLeft => (-0.5, -0.5),
            Anchor::BottomMiddle => (0.0, -0.5),
            Anchor::BottomRight => (0.5, -0.5),
        }
    }

    /// Vertical align. Used by the `UiGlyphsSystem`.
    pub(crate) fn vertical_align(self) -> VerticalAlign {
        match self {
            Anchor::TopLeft => VerticalAlign::Top,
            Anchor::TopMiddle => VerticalAlign::Top,
            Anchor::TopRight => VerticalAlign::Top,
            Anchor::MiddleLeft => VerticalAlign::Center,
            Anchor::Middle => VerticalAlign::Center,
            Anchor::MiddleRight => VerticalAlign::Center,
            Anchor::BottomLeft => VerticalAlign::Bottom,
            Anchor::BottomMiddle => VerticalAlign::Bottom,
            Anchor::BottomRight => VerticalAlign::Bottom,
        }
    }

    /// Horizontal align. Used by the `UiGlyphsSystem`.
    pub(crate) fn horizontal_align(self) -> HorizontalAlign {
        match self {
            Anchor::TopLeft => HorizontalAlign::Left,
            Anchor::TopMiddle => HorizontalAlign::Center,
            Anchor::TopRight => HorizontalAlign::Right,
            Anchor::MiddleLeft => HorizontalAlign::Left,
            Anchor::Middle => HorizontalAlign::Center,
            Anchor::MiddleRight => HorizontalAlign::Right,
            Anchor::BottomLeft => HorizontalAlign::Left,
            Anchor::BottomMiddle => HorizontalAlign::Center,
            Anchor::BottomRight => HorizontalAlign::Right,
        }
    }
}

/// Indicates if a component should be stretched.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Stretch {
    /// No stretching occurs
    NoStretch,
    /// Stretches on the X axis.
    X {
        /// The margin length for the width
        x_margin: f32,
    },
    /// Stretches on the Y axis.
    Y {
        /// The margin length for the height
        y_margin: f32,
    },
    /// Stretches on both axes.
    XY {
        /// The margin length for the width
        x_margin: f32,
        /// The margin length for the height
        y_margin: f32,
        /// Keep the aspect ratio by adding more margin to one axis when necessary
        keep_aspect_ratio: bool,
    },
}

/// Manages the `Parent` component on entities having `UiTransform`
/// It does almost the same as the `TransformSystem`, but with some differences,
/// like `UiTransform` alignment and stretching.
#[derive(Debug)]
pub struct UiTransformSystem {
    screen_size: (f32, f32),
}

impl<'a> UiTransformSystem {
    /// Creates a new `UiTransformSystem`.
    pub fn new(world: &World) -> Self {
        Self {
            screen_size: (0.0, 0.0),
        }
    }

    pub fn build(&'static mut self) -> Box<dyn Runnable> {
        Box::new(
            SystemBuilder::<()>::new("UiTransformSystem")
                .read_resource::<ScreenDimensions>()
                .with_query(
                    <(Entity, Write<UiTransform>, Read<Parent>)>::query()
                        .filter(maybe_changed::<UiTransform>()),
                )
                .with_query(<(Entity, Read<Parent>)>::query())
                .build(move |commands, world, screen_dim, (query, parents)| {
                    let current_screen_size = (screen_dim.width(), screen_dim.height());
                    let screen_resized = current_screen_size != self.screen_size;
                    self.screen_size = current_screen_size;
                    if screen_resized {
                        process_root_iter(
                            <Write<UiTransform>>::query()
                                .filter(!component::<Parent>())
                                .iter_mut(world),
                            &*screen_dim,
                        );
                    } else {
                        // FIXME: should only process modified UI
                        process_root_iter(
                            <Write<UiTransform>>::query()
                                .filter(!component::<Parent>())
                                .iter_mut(world),
                            &*screen_dim,
                        );
                    }

                    /* FIXME: we don't have modified event in legion
                    // Populate the modifications we just did.
                    self.events.try_iter().for_each(|event| {
                        if let ComponentEvent::Modified(id) = event {
                            self_transform_modified.add(*id);
                        }
                    });*/

                    let transforms: Vec<(Entity, UiTransform)> = {
                        <(Entity, Read<UiTransform>)>::query()
                            .iter(world)
                            .map(|x| (*x.0, x.1.clone()))
                            .collect()
                    };

                    // Compute transforms with parents.
                    for (entity, transform, parent) in query.iter_mut(world) {
                        {
                            if let Some(parent_transform_copy) = transforms
                                .iter()
                                .find(|x| x.0 == parent.0)
                                .map(|x| x.1.clone())
                            {
                                let norm = transform.anchor.norm_offset();
                                transform.pixel_x = parent_transform_copy.pixel_x
                                    + parent_transform_copy.pixel_width * norm.0;
                                transform.pixel_y = parent_transform_copy.pixel_y
                                    + parent_transform_copy.pixel_height * norm.1;
                                transform.global_z =
                                    parent_transform_copy.global_z + transform.local_z;

                                let new_size = match transform.stretch {
                                    Stretch::NoStretch => (transform.width, transform.height),
                                    Stretch::X { x_margin } => (
                                        parent_transform_copy.pixel_width - x_margin * 2.0,
                                        transform.height,
                                    ),
                                    Stretch::Y { y_margin } => (
                                        transform.width,
                                        parent_transform_copy.pixel_height - y_margin * 2.0,
                                    ),
                                    Stretch::XY {
                                        keep_aspect_ratio: false,
                                        x_margin,
                                        y_margin,
                                    } => (
                                        parent_transform_copy.pixel_width - x_margin * 2.0,
                                        parent_transform_copy.pixel_height - y_margin * 2.0,
                                    ),
                                    Stretch::XY {
                                        keep_aspect_ratio: true,
                                        x_margin,
                                        y_margin,
                                    } => {
                                        let scale = f32::min(
                                            (parent_transform_copy.pixel_width - x_margin * 2.0)
                                                / transform.width,
                                            (parent_transform_copy.pixel_height - y_margin * 2.0)
                                                / transform.height,
                                        );

                                        (transform.width * scale, transform.height * scale)
                                    }
                                };
                                transform.width = new_size.0;
                                transform.height = new_size.1;

                                match transform.scale_mode {
                                    ScaleMode::Pixel => {
                                        transform.pixel_x += transform.local_x;
                                        transform.pixel_y += transform.local_y;
                                        transform.pixel_width = transform.width;
                                        transform.pixel_height = transform.height;
                                    }
                                    ScaleMode::Percent => {
                                        transform.pixel_x +=
                                            transform.local_x * parent_transform_copy.pixel_width;
                                        transform.pixel_y +=
                                            transform.local_y * parent_transform_copy.pixel_height;
                                        transform.pixel_width =
                                            transform.width * parent_transform_copy.pixel_width;
                                        transform.pixel_height =
                                            transform.height * parent_transform_copy.pixel_height;
                                    }
                                }
                                let pivot_norm = transform.pivot.norm_offset();
                                transform.pixel_x += transform.pixel_width * -pivot_norm.0;
                                transform.pixel_y += transform.pixel_height * -pivot_norm.1;
                            }
                        }
                    }
                }),
        )
    }
}

fn process_root_iter<'a, I>(iter: I, screen_dim: &ScreenDimensions)
where
    I: Iterator<Item = &'a mut UiTransform>,
{
    for transform in iter {
        let norm = transform.anchor.norm_offset();
        transform.pixel_x = screen_dim.width() / 2.0 + screen_dim.width() * norm.0;
        transform.pixel_y = screen_dim.height() / 2.0 + screen_dim.height() * norm.1;
        transform.global_z = transform.local_z;

        let new_size = match transform.stretch {
            Stretch::NoStretch => (transform.width, transform.height),
            Stretch::X { x_margin } => (screen_dim.width() - x_margin * 2.0, transform.height),
            Stretch::Y { y_margin } => (transform.width, screen_dim.height() - y_margin * 2.0),
            Stretch::XY {
                keep_aspect_ratio: false,
                x_margin,
                y_margin,
            } => (
                screen_dim.width() - x_margin * 2.0,
                screen_dim.height() - y_margin * 2.0,
            ),
            Stretch::XY {
                keep_aspect_ratio: true,
                x_margin,
                y_margin,
            } => {
                let scale = f32::min(
                    (screen_dim.width() - x_margin * 2.0) / transform.width,
                    (screen_dim.height() - y_margin * 2.0) / transform.height,
                );

                (transform.width * scale, transform.height * scale)
            }
        };
        transform.width = new_size.0;
        transform.height = new_size.1;
        match transform.scale_mode {
            ScaleMode::Pixel => {
                transform.pixel_x += transform.local_x;
                transform.pixel_y += transform.local_y;
                transform.pixel_width = transform.width;
                transform.pixel_height = transform.height;
            }
            ScaleMode::Percent => {
                transform.pixel_x += transform.local_x * screen_dim.width();
                transform.pixel_y += transform.local_y * screen_dim.height();
                transform.pixel_width = transform.width * screen_dim.width();
                transform.pixel_height = transform.height * screen_dim.height();
            }
        }
        let pivot_norm = transform.pivot.norm_offset();
        transform.pixel_x += transform.pixel_width * -pivot_norm.0;
        transform.pixel_y += transform.pixel_height * -pivot_norm.1;
    }
}
