use crate::ecs::{
    systems::{Executor, ParallelRunnable, Step},
    *,
};
use amethyst_error::Error;

/// A SystemBundle is a structure that adds multiple systems to the [Dispatcher] and loads/unloads all required resources.
pub trait SystemBundle {
    /// This method is lazily evaluated when [Dispatcher] is built with [DispatcherBuilder::build].
    /// It is used to add systems or bundles (recursive) into [Dispatcher]. [World] and [Resources] are
    /// also provided to initialize any entities or resources used by the system.
    fn load(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
        builder: &mut DispatcherBuilder,
    ) -> Result<(), Error>;

    /// This method is called once [Dispatcher] is disposed. It can be used to cleanup entities or resources from ECS.
    fn unload(&mut self, _world: &mut World, _resources: &mut Resources) -> Result<(), Error> {
        Ok(())
    }
}

/// This structure is an intermediate step for building [Dispatcher]. When [DispatcherBuilder::build] is called,
/// all system bundles are evaluated by calling [SystemBundle::load]. This structure is used to split systems
/// (executable by [Schedule]) and system bundles (used for cleanup with unload).
#[derive(Default)]
pub struct DispatcherData {
    /// Holds all steps that can be executed by [Schedule].
    steps: Vec<Step>,
    /// Temporarily holds systems which are later combined into [Executor].
    accumulator: Vec<Box<dyn ParallelRunnable>>,
    /// Bundles that can be later used for cleanup by calling [SystemBundle::unload].
    bundles: Vec<Box<dyn SystemBundle>>,
}

impl DispatcherData {
    fn finalize_executor(&mut self) {
        if !self.accumulator.is_empty() {
            let mut systems = Vec::new();
            std::mem::swap(&mut self.accumulator, &mut systems);
            let executor = Executor::new(systems);
            self.steps.push(Step::Systems(executor));
        }
    }
}

/// A builder which is used to construct [Dispatcher] from multiple systems and system bundles.
#[derive(Default)]
pub struct DispatcherBuilder {
    items: Vec<DispatcherItem>,
}

impl DispatcherBuilder {
    /// Adds a system to the schedule.
    pub fn add_system<T: ParallelRunnable + 'static>(&mut self, system: T) -> &mut Self {
        self.items.push(DispatcherItem::System(Box::new(system)));
        self
    }

    /// Waits for executing systems to complete, and the flushes all outstanding system
    /// command buffers.
    pub fn flush(&mut self) -> &mut Self {
        self.items.push(DispatcherItem::FlushCmdBuffers);
        self
    }

    /// Adds a thread local function to the schedule. This function will be executed on the main thread.
    pub fn add_thread_local_fn<F: FnMut(&mut World, &mut Resources) + 'static>(
        &mut self,
        f: F,
    ) -> &mut Self {
        self.items.push(DispatcherItem::ThreadLocalFn(
            Box::new(f) as Box<dyn FnMut(&mut World, &mut Resources)>
        ));
        self
    }

    /// Adds a thread local system to the schedule. This system will be executed on the main thread.
    pub fn add_thread_local<S: Runnable + 'static>(&mut self, system: S) -> &mut Self {
        let system = Box::new(system) as Box<dyn Runnable>;
        self.items.push(DispatcherItem::ThreadLocalSystem(system));
        self
    }

    /// Adds [SystemBundle] to the dispatcher. System bundles allow inserting multiple systems
    /// and initialize any required entities or resources.
    pub fn add_bundle<T: SystemBundle + 'static>(&mut self, bundle: T) -> &mut Self {
        self.items
            .push(DispatcherItem::SystemBundle(Box::new(bundle)));
        self
    }

    /// Evaluates all system bundles (recursively). Resulting systems and unpacked bundles are put into [DispatcherData].
    pub fn load(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
        data: &mut DispatcherData,
    ) -> Result<(), Error> {
        for item in self.items.drain(..) {
            match item {
                DispatcherItem::System(s) => data.accumulator.push(s),
                DispatcherItem::FlushCmdBuffers => {
                    data.finalize_executor();
                    data.steps.push(Step::FlushCmdBuffers);
                }
                DispatcherItem::ThreadLocalFn(f) => {
                    data.finalize_executor();
                    data.steps.push(Step::ThreadLocalFn(f));
                }
                DispatcherItem::ThreadLocalSystem(s) => {
                    data.finalize_executor();
                    data.steps.push(Step::ThreadLocalSystem(s));
                }
                DispatcherItem::SystemBundle(mut bundle) => {
                    let mut builder = DispatcherBuilder::default();
                    bundle.load(world, resources, &mut builder)?;
                    builder.load(world, resources, data)?;
                    data.bundles.push(bundle);
                }
            }
        }

        Ok(())
    }

    /// Finalizes the builder into a [Dispatcher]. This also evaluates all system bundles by calling [SystemBundle::load].
    pub fn build(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
    ) -> Result<Dispatcher, Error> {
        let mut data = DispatcherData::default();

        self.flush().load(world, resources, &mut data)?;

        Ok(Dispatcher {
            schedule: Schedule::from(data.steps),
            bundles: data.bundles,
        })
    }
}

/// Dispatcher items. This is different from [Step] in that it contains [SystemBundle].
pub enum DispatcherItem {
    /// A simple system.
    System(Box<dyn ParallelRunnable>),
    /// Flush system command buffers.
    FlushCmdBuffers,
    /// A thread local function.
    ThreadLocalFn(Box<dyn FnMut(&mut World, &mut Resources)>),
    /// A thread local system.
    ThreadLocalSystem(Box<dyn Runnable>),
    /// A system bundle
    SystemBundle(Box<dyn SystemBundle>),
}

/// Dispatcher is created by [DispatcherBuilder] and contains [Schedule] used to execute all systems.
pub struct Dispatcher {
    // Used to execute unload on system bundles once dispatcher is disposed.
    bundles: Vec<Box<dyn SystemBundle>>,
    schedule: Schedule,
}

impl Dispatcher {
    /// Executes systems according to the [Schedule].
    pub fn execute(&mut self, world: &mut World, resources: &mut Resources) {
        // TODO: use ArcThreadPool from resources to dispatch legion
        self.schedule.execute(world, resources);
    }

    /// Unloads any resources by calling [SystemBundle::unload] for stored system bundles and returns [DispatcherBuilder]
    /// containing the same bundles.
    pub fn unload(mut self, world: &mut World, resources: &mut Resources) -> Result<(), Error> {
        for bundle in &mut self.bundles {
            bundle.unload(world, resources)?;
        }

        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    struct MyResource(bool);

    #[test]
    fn dispatcher_loads_and_unloads() {
        struct MyBundle;

        impl SystemBundle for MyBundle {
            fn load(
                &mut self,
                _world: &mut World,
                resources: &mut Resources,
                _builder: &mut DispatcherBuilder,
            ) -> Result<(), Error> {
                resources.insert(MyResource(false));
                Ok(())
            }

            fn unload(
                &mut self,
                _world: &mut World,
                resources: &mut Resources,
            ) -> Result<(), Error> {
                resources.remove::<MyResource>();
                Ok(())
            }
        }

        let mut world = World::default();
        let mut resources = Resources::default();

        // Create dispatcher
        let dispatcher = DispatcherBuilder::default()
            .add_bundle(MyBundle)
            .build(&mut world, &mut resources)
            .unwrap();

        // Ensure that resources were loaded
        assert!(resources.get::<MyResource>().is_some());

        // Unload
        dispatcher.unload(&mut world, &mut resources).unwrap();

        // Ensure that resources were unloaded
        assert!(resources.get::<MyResource>().is_none());
    }

    #[test]
    fn dispatcher_legion_system() {
        let mut world = World::default();
        let mut resources = Resources::default();

        resources.insert(MyResource(false));

        let system = SystemBuilder::new("test")
            .write_resource::<MyResource>()
            .build(|_, _, res, _| {
                res.0 = true;
            });

        let mut dispatcher = DispatcherBuilder::default()
            .add_system(system)
            .build(&mut world, &mut resources)
            .unwrap();

        dispatcher.execute(&mut world, &mut resources);

        assert_eq!(resources.get::<MyResource>().unwrap().0, true);
    }
}
