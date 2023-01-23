use bevy::{prelude::{IntoSystemDescriptor, In, With, Res, IntoPipeSystem, IntoSystem, EventWriter}, ecs::{query::ReadOnlyWorldQuery, schedule::SystemDescriptor}, core::FrameCount, app::AppExit, log::LogPlugin};
use bevy::{
    ecs::{
        schedule_v3::{self, IntoSystemSetConfig},
    },
    prelude::{
        App, Commands, Component, Entity, Mut, Query, ResMut, Resource, World,
    },
    MinimalPlugins,
};

impl CameraSchedule {
    // i'd like to get rid of this & but i don't know how         \/
    pub fn camera_system<F: ReadOnlyWorldQuery + 'static, P>(func: &'static (impl IntoSystem<Entity, (), P> + Send + Sync + Copy)) -> SystemDescriptor {
        (move |cams: Query<Entity, (With<Camera>, F)>,
         cam_schedule: ResMut<CameraSchedule>,
         update: Res<UpdateSchedule>| {
            add_camera_system(update, cams, cam_schedule, func)
        })
        .label("add_cam_system").after("setup")
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugin(LogPlugin::default());

    let schedule = schedule_v3::Schedule::default();
    app.insert_resource(CameraSchedule(schedule));

    app.add_startup_system(test_setup);

    app.add_system(setup_camera_systems.label("setup"));

    app.add_system(CameraSchedule::camera_system::<(With<Camera2d>, With<Bloom>), _>(&bloom_2d));
    app.add_system(CameraSchedule::camera_system::<With<Bloom>, _>(&bloom));

    app.add_system(run_camera_schedule.after("add_cam_system"));
    app.add_system(die);
    app.insert_resource(Print(true));
    app.insert_resource(UpdateSchedule(true));

    let now = std::time::Instant::now();
    app.run();
    let done = std::time::Instant::now();
    let elapsed = done - now;
    println!("elapsed: {:?}", elapsed);
}

#[derive(Resource)]
struct Print(bool);

#[derive(Resource)]
struct UpdateSchedule(bool);

fn die(
    frame: Res<FrameCount>,
    mut exit: EventWriter<AppExit>,
) {
    if frame.0 == 10 {
        exit.send_default();
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
struct CameraSystemSet {
    entity: Entity,
}

impl schedule_v3::SystemSet for CameraSystemSet {
    fn dyn_clone(&self) -> Box<dyn schedule_v3::SystemSet> {
        Box::new(self.clone())
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, bevy::ecs::schedule_v3::SystemSet)]
struct CoreCameraSystemSet {
    entity: Entity,
}

#[derive(Resource)]
struct CameraSchedule(schedule_v3::Schedule);

#[derive(Component, Default)]
struct Camera {
    run_after: Vec<Entity>,
}

#[derive(Component)]
struct Camera2d;

fn test_setup(mut commands: Commands) {
    let zero = commands.spawn((Camera::default(),)).id();

    let one = commands
        .spawn((
            Camera {
                run_after: vec![zero],
            },
            Bloom,
            Camera2d,
        ))
        .id();

    let two = commands
        .spawn((Camera {
            run_after: vec![zero],
        },))
        .id();

    let three = commands
        .spawn((
            Camera {
                run_after: vec![zero, one],
            },
            Bloom,
        ))
        .id();

    // zero
    // -> one (with bloom and bloom 2d)
    //    -> three (with bloom)
    // -> two
    println!("{:?}", [zero, one, two, three]);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, bevy::ecs::schedule_v3::SystemSet)]
struct Label{
    label: &'static str, 
    entity: Option<Entity>,
}

impl Label {
    pub fn new(label: &'static str) -> Self {
        Self{label, entity: None}
    }

    pub fn for_entity(&self, entity: Entity) -> Self {
        Self {
            label: self.label,
            entity: Some(entity),
        }
    }
}

fn setup_camera_systems(mut sched: ResMut<CameraSchedule>, cams: Query<(Entity, &Camera)>, update: Res<UpdateSchedule>) {
    use schedule_v3::IntoSystemConfigs;
    if update.0 {
        sched.0 = schedule_v3::Schedule::default();

        for (entity, cam) in &cams {
            let set = CameraSystemSet { entity };

            for other in &cam.run_after {
                sched
                    .0
                    .configure_set(set.after(CameraSystemSet { entity: *other }));
            }

            let core_systems = (
                (move || entity).pipe(clear), 
                (move || entity).pipe(opaque)
            ).chain();
            sched
                .0
                .add_systems(core_systems.in_set(set).in_set(Label::new("core").for_entity(entity)));
        }
    }
}

fn run_camera_schedule(world: &mut World) {
    let print = world.resource::<Print>().0;
    world.resource_scope(|world: &mut World, mut sched: Mut<CameraSchedule>| {
        if print {
            println!(">>> scope");
        }
        let sched = &mut sched.0;
        // sched.initialize(world).unwrap();
        sched.run(world);
        if print {
            println!("<<< scope");
        }
    });

    // bevy::utils::tracing::event!(
    //     bevy::utils::tracing::Level::INFO,
    //     message = "finished frame",
    //     tracy.frame_mark = true
    // );

    // world.resource_mut::<UpdateSchedule>().0 = false;
}

fn clear(
    In(view): In<Entity>, 
    print: Res<Print>,
) {
    if print.0 {
        println!("clear {:?}", view);
    }
}

fn opaque(
    In(view): In<Entity>, 
    print: Res<Print>,
) {
    if print.0 {
        println!("opaque {:?}", view);
    }
}


#[derive(Component)]
struct Bloom;

fn add_camera_system<F: ReadOnlyWorldQuery, P>(
    update: Res<UpdateSchedule>,
    cams: Query<Entity, (With<Camera>, F)>,
    mut cam_schedule: ResMut<CameraSchedule>,
    func: &'static (impl IntoSystem<Entity, (), P> + Copy),
) {
    use schedule_v3::IntoSystemConfig;
    if update.0 {
        for entity in cams.iter() {
            cam_schedule.0.add_system((move || entity).pipe(*func).in_set(CameraSystemSet{entity}).after(Label::new("core").for_entity(entity)));
        }
    }
}

fn bloom(
    In(view): In<Entity>, 
    print: Res<Print>,
) {
    if print.0 {
        println!("  bloom {:?}", view);
    }
}

fn bloom_2d(
    In(view): In<Entity>, 
    print: Res<Print>,
) {
    if print.0 {
        println!("   bloom 2d {:?}", view);
    }
}

