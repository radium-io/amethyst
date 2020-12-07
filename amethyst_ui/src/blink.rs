//! Module for the Blink component and BlinkSystem.
use amethyst_core::{ecs::*, Hidden, Time};

#[cfg(feature = "profiler")]
use thread_profiler::profile_scope;

/// # Blink Component
/// Periodically adds and removes a `Hidden` Component on the entity this is attached to.
///
/// ## Visibility Period
/// During the first half period, the entity is visible.
/// [0, delay/2[
///
/// During the second half period, the entity is invisible.
/// [delay/2, delay]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Blink {
    /// Period of a full blink cycle.
    pub delay: f32,
    /// Timer value keeping track of the time during the blink cycle.
    pub timer: f32,
    /// Whether to use the scaled or unscaled time.
    pub absolute_time: bool,
}

/// System updating the `Blink` component.
#[derive(Debug)]
pub struct BlinkSystem;

pub fn build_blink_system() -> Box<dyn Runnable> {
    Box::new(
        SystemBuilder::<()>::new("BlinkSystem")
            .read_resource::<Time>()
            .read_component::<Hidden>()
            .with_query(<(Entity, Write<Blink>, TryWrite<Hidden>)>::query())
            .build(move |commands, world, time, query| {
                let abs_sec = time.delta_seconds();
                let abs_unscaled_sec = time.delta_real_seconds();

                for (entity, mut blink, hidden) in query.iter_mut(world) {
                    if blink.absolute_time {
                        blink.timer += abs_unscaled_sec;
                    } else {
                        blink.timer += abs_sec;
                    }

                    // Reset timer because we ended the last cycle.
                    // Keeps the overflow time.
                    if blink.timer > blink.delay {
                        blink.timer -= blink.delay;
                    }

                    // We could cache the division, but that would require a stricter api on Blink.
                    let on = blink.timer < blink.delay / 2.0;

                    match (on, hidden.is_some()) {
                        (true, false) => commands.add_component(*entity, Hidden),
                        (false, true) => commands.remove_component::<Hidden>(*entity),
                        _ => {}
                    };
                }
            }),
    )
}
