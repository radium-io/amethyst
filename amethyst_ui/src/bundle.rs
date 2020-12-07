//! ECS rendering bundle

use crate::{
    BlinkSystem, CacheSelectionOrderSystem, DragWidgetSystem, FontAsset, NoCustomUi, ResizeSystem,
    SelectionKeyboardSystem, SelectionMouseSystem, TextEditingInputSystem, TextEditingMouseSystem,
    ToNativeWidget, UiButtonActionRetriggerSystem, UiButtonSystem, UiLoaderSystem, UiMouseSystem,
    UiSoundRetriggerSystem, UiSoundSystem, UiTransformSystem, WidgetId,
};
use amethyst_core::{
    dispatcher::{DispatcherBuilder, SystemBundle},
    ecs::*,
};
use amethyst_error::Error;
use derive_new::new;
use std::marker::PhantomData;

/// UI bundle
///
/// Will register all necessary components and systems needed for UI, along with any resources.
///
/// Will fail with error 'No resource with the given id' if either the InputBundle or TransformBundle are not added.
#[derive(new, Debug)]
pub struct UiBundle<C = NoCustomUi, W = u32, G = ()> {
    #[new(default)]
    _marker: PhantomData<(C, W, G)>,
}

impl<'a, 'b, C, W, G> SystemBundle<'a, 'b> for UiBundle<C, W, G>
where
    C: ToNativeWidget,
    W: WidgetId,
    G: Send + Sync + PartialEq + 'static,
{
    fn build(
        self,
        world: &mut World,
        builder: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        builder.add(
            UiLoaderSystem::<<C as ToNativeWidget>::PrefabData, W>::default().build(world),
            "ui_loader",
            &[],
        );
        builder.add(
            UiTransformSystem::default().build(world),
            "ui_transform",
            &["transform_system"],
        );
        builder.add(
            UiMouseSystem::new(),
            "ui_mouse_system",
            &["input_system", "ui_transform"],
        );
        builder.add(
            Processor::<FontAsset>::new(),
            "font_processor",
            &["ui_loader"],
        );
        builder.add(
            CacheSelectionOrderSystem::<G>::new(),
            "selection_order_cache",
            &[],
        );
        builder.add(
            SelectionMouseSystem::<G>::default().build(world),
            "ui_mouse_selection",
            &["ui_mouse_system"],
        );
        builder.add(
            SelectionKeyboardSystem::<G>::default().build(world),
            "ui_keyboard_selection",
            // Because when you press tab, you want to override the previously selected elements.
            &["ui_mouse_selection"],
        );
        builder.add(
            TextEditingMouseSystem::default().build(world),
            "ui_text_editing_mouse_system",
            &["ui_mouse_selection", "ui_keyboard_selection"],
        );
        builder.add(
            TextEditingInputSystem::default().build(world),
            "ui_text_editing_input_system",
            // Hard requirement. The system assumes the text to edit is selected.
            &["ui_mouse_selection", "ui_keyboard_selection"],
        );
        builder.add(
            ResizeSystem::default().build(world),
            "ui_resize_system",
            &[],
        );
        builder.add(
            UiButtonSystem::default().build(world),
            "ui_button_system",
            &["ui_mouse_system"],
        );
        builder.add(
            DragWidgetSystem::default().build(world),
            "ui_drag_system",
            &["ui_mouse_system"],
        );

        builder.add(
            UiButtonActionRetriggerSystem::default().build(world),
            "ui_button_action_retrigger_system",
            &["ui_button_system"],
        );
        builder.add(
            UiSoundSystem::default().build(world),
            "ui_sound_system",
            &[],
        );
        builder.add(
            UiSoundRetriggerSystem::default().build(world),
            "ui_sound_retrigger_system",
            &["ui_sound_system"],
        );

        // Required for text editing. You want the cursor image to blink.
        builder.add(BlinkSystem, "blink_system", &[]);

        Ok(())
    }
}
