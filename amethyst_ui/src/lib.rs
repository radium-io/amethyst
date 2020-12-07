//! Provides components and systems to create an in game user interface.

#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    rust_2018_compatibility
)]
#![warn(clippy::all)]
#![allow(clippy::new_without_default)]

pub use self::blink::*;
pub use self::bundle::*;
//pub use self::button::*;
pub use self::event::*;
pub use self::image::*;
pub use self::label::*;
pub use self::layout::*;
pub use self::pass::*;
pub use self::selection::*;
pub use self::selection_order_cache::*;
pub use self::text::*;
pub use self::transform::*;

mod blink;
mod bundle;
//mod button;
// mod drag;
mod event;
// mod event_retrigger;
// mod font;
// mod format;
// mod glyphs;
mod image;
mod label;
mod layout;
mod pass;
// mod prefab;
// mod resize;
mod selection;
mod selection_order_cache;
// mod sound;
mod text;
// mod text_editing;
mod transform;
// mod widgets;
