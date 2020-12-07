use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use amethyst_core::{ecs::*, math::Vector2, shrev::EventChannel, Hidden, HiddenPropagate};
use amethyst_input::InputHandler;
use amethyst_window::ScreenDimensions;

use crate::{
    get_parent_pixel_size, targeted_below, Interactable, ScaleMode, UiEvent, UiEventType,
    UiTransform,
};

/// Component that denotes whether a given ui widget is draggable.
/// Requires UiTransform to work, and its expected way of usage is
/// through UiTransformData prefab.
#[derive(Debug, Serialize, Deserialize)]
pub struct Draggable;

#[derive(Debug)]
pub struct DragWidgetSystem {
    ui_reader_id: ReaderId<UiEvent>,

    /// hashmap whose keys are every entities being dragged,
    /// and whose element is a tuple whose first element is
    /// the original mouse position when drag first started,
    /// and second element the mouse position one frame ago
    record: HashMap<Entity, (Vector2<f32>, Vector2<f32>)>,
}

impl DragWidgetSystem {
    pub fn new(ui_reader_id: ReaderId<UiEvent>) -> Self {
        Self {
            ui_reader_id,
            record: HashMap::new(),
        }
    }
}

impl<'s> System<'s> for DragWidgetSystem {
    type SystemData = (
        Entities<'s>,
        Read<'s, InputHandler>,
        ReadExpect<'s, ScreenDimensions>,
        ReadExpect<'s, ParentHierarchy>,
        ReadStorage<'s, Hidden>,
        ReadStorage<'s, HiddenPropagate>,
        ReadStorage<'s, Draggable>,
        ReadStorage<'s, Interactable>,
        Write<'s, EventChannel<UiEvent>>,
        WriteStorage<'s, UiTransform>,
    );

    fn run(
        &mut self,
        (
            entities,
            input_handler,
            screen_dimensions,
            hierarchy,
            hiddens,
            hidden_props,
            draggables,
            interactables,
            mut ui_events,
            mut ui_transforms,
        ): Self::SystemData,
    ) {
        let mouse_pos = input_handler.mouse_position().unwrap_or((0., 0.));
        let mouse_pos = Vector2::new(mouse_pos.0, screen_dimensions.height() - mouse_pos.1);

        let mut click_stopped: HashSet<Entity> = HashSet::new();

        for event in ui_events.read(&mut self.ui_reader_id) {
            match event.event_type {
                UiEventType::ClickStart => {
                    if draggables.get(event.target).is_some() {
                        self.record.insert(event.target, (mouse_pos, mouse_pos));
                    }
                }
                UiEventType::ClickStop => {
                    if self.record.contains_key(&event.target) {
                        click_stopped.insert(event.target);
                    }
                }
                _ => (),
            }
        }

        for (entity, _) in self.record.iter() {
            if hiddens.get(*entity).is_some() || hidden_props.get(*entity).is_some() {
                click_stopped.insert(*entity);
            }
        }

        for (entity, (first, prev)) in self.record.iter_mut() {
            ui_events.single_write(UiEvent::new(
                UiEventType::Dragging {
                    offset_from_mouse: mouse_pos - *first,
                    new_position: mouse_pos,
                },
                *entity,
            ));

            let change = mouse_pos - *prev;

            let (parent_width, parent_height) =
                get_parent_pixel_size(*entity, &world, &screen_dimensions);

            let ui_transform = ui_transforms.get_mut(*entity).unwrap();
            let (scale_x, scale_y) = match ui_transform.scale_mode {
                ScaleMode::Pixel => (1.0, 1.0),
                ScaleMode::Percent => (parent_width, parent_height),
            };

            ui_transform.local_x += change[0] / scale_x;
            ui_transform.local_y += change[1] / scale_y;

            *prev = mouse_pos;
        }

        for entity in click_stopped.iter() {
            ui_events.single_write(UiEvent::new(
                UiEventType::Dropped {
                    dropped_on: targeted_below(
                        (mouse_pos[0], mouse_pos[1]),
                        ui_transforms.get(*entity).unwrap().global_z,
                        (
                            &*entities,
                            &ui_transforms,
                            interactables.maybe(),
                            !&hiddens,
                            !&hidden_props,
                        )
                            .join(),
                    ),
                },
                *entity,
            ));

            self.record.remove(entity);
        }
    }
}
