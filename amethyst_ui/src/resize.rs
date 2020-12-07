use amethyst_core::{ecs::*, shrev::ReaderId};
use amethyst_window::ScreenDimensions;

#[cfg(feature = "profiler")]
use thread_profiler::profile_scope;

use crate::UiTransform;

/// Whenever the window is resized the function in this component will be called on this
/// entity's UiTransform, along with the new width and height of the window.
///
/// The function in this component is also guaranteed to be called at least once by the
/// `ResizeSystem` when either the component is attached, or the function is changed.
pub struct UiResize {
    /// The core function of this component
    pub function: Box<dyn FnMut(&mut UiTransform, (f32, f32)) + Send + Sync>,
}

impl UiResize {
    /// Creates a new component with the given function.
    pub fn new<F>(function: F) -> Self
    where
        F: FnMut(&mut UiTransform, (f32, f32)) + Send + Sync + 'static,
    {
        UiResize {
            function: Box::new(function),
        }
    }
}

/// This system rearranges UI elements whenever the screen is resized using their `UiResize`
/// component.
#[derive(Debug)]
pub struct ResizeSystem {
    screen_size: (f32, f32),
    local_modified: BitSet,
}

impl ResizeSystem {
    pub fn new() -> ResizeSystem {
        let screen_size = (0.0, 0.0);
        let (tx, rx) = crossbeam_channel::unbounded::<world::Event>();
        world.subscribe(tx, component::<UiResize>());

        ResizeSystem {
            screen_size,
            rx,
            local_modified: BitSet::default(),
        }
    }
}

impl<'a> System<'a> for ResizeSystem {
    type SystemData = (
        WriteStorage<'a, UiTransform>,
        WriteStorage<'a, UiResize>,
        ReadExpect<'a, ScreenDimensions>,
    );

    fn run(&mut self, (mut transform, mut resize, dimensions): Self::SystemData) {
        #[cfg(feature = "profiler")]
        profile_scope!("resize_system");

        self.local_modified.clear();

        let self_local_modified = &mut self.local_modified;

        let self_resize_events_id = &mut self.resize_events_id;
        resize
            .channel()
            .read(self_resize_events_id)
            .for_each(|event| match event {
                ComponentEvent::Inserted(id) | ComponentEvent::Modified(id) => {
                    self_local_modified.add(*id);
                }
                ComponentEvent::Removed(_id) => {}
            });

        let screen_size = (dimensions.width() as f32, dimensions.height() as f32);
        if self.screen_size != screen_size {
            self.screen_size = screen_size;
            for (transform, resize) in (&mut transform, &mut resize).join() {
                (resize.function)(transform, screen_size);
            }
        } else {
            // Immutable borrow
            let self_local_modified = &*self_local_modified;
            for (transform, resize, _) in (&mut transform, &mut resize, self_local_modified).join()
            {
                (resize.function)(transform, screen_size);
            }
        }

        // We need to treat any changes done inside the system as non-modifications, so we read out
        // any events that were generated during the system run
        resize
            .channel()
            .read(self_resize_events_id)
            .for_each(|event| match event {
                ComponentEvent::Inserted(id) | ComponentEvent::Modified(id) => {
                    self_local_modified.add(*id);
                }
                ComponentEvent::Removed(_id) => {}
            });
    }
}
