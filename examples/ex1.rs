use bevy::asset::LoadState;
use bevy::ecs::system::Resource;
use bevy::prelude::{
    App, AssetServer, Commands, DefaultPlugins, EventReader, Font, Handle, NonSend, NonSendMut,
    Res, ResMut, State, SystemSet, UiCameraBundle, Windows,
};
use bevy::utils::HashMap;
use bevy::window::*;
use druid::widget::*;
use druid::*;
use std::collections::VecDeque;

#[derive(Clone, Data, Debug)]
struct SomeData(String);

trait Root {
    fn root() -> Box<dyn Widget<Self>>;
}

// Two windows share the same data but have different root widgets, so
// this fails? Tie root widget to camera?
impl Root for SomeData {
    fn root() -> Box<dyn Widget<Self>> {
        let label = Label::new("Hello, druid."); //.align_vertical(UnitPoint::TOP_LEFT);
        let button = Button::new("Quit");
        let column = Flex::column().with_child(label).with_child(button);
        Box::new(column) as Box<dyn Widget<Self>>
    }
}

type DruidWindows<T> = HashMap<bevy::window::WindowId, druid::Window<T>>;

// We have to pre-load fonts since Druid will panic if a text layout
// fails to build.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DruidState {
    Loading,
    Running,
}

#[derive(Default)]
struct DruidFonts(Vec<Handle<Font>>);

fn main() {
    let mut env = Env::with_default_i10n();
    env.set(
        druid::theme::UI_FONT,
        // loaded from assets, system fonts don't work
        FontDescriptor::new(FontFamily::new_unchecked("Vollkorn-Regular.ttf")).with_size(15.0),
    );

    App::new()
        // .insert_resource(bevy::window::WindowDescriptor {
        //     scale_factor_override: Some(1.0),
        //     ..Default::default()
        // })
        .add_plugins_with(DefaultPlugins, |group| {
            group.disable::<bevy::ui::UiPlugin>()
        })
        .add_plugin(piet::PietPlugin::default())
        .add_state(DruidState::Loading)
        .insert_non_send_resource(env)
        .insert_resource(SomeData("druid".to_string()))
        .insert_non_send_resource(DruidWindows::<SomeData>::default())
        .insert_resource(DruidFonts::default())
        .add_system(druid_window_system::<SomeData>)
        .add_system_set(SystemSet::on_enter(DruidState::Loading).with_system(setup))
        .add_system_set(SystemSet::on_update(DruidState::Loading).with_system(check_fonts))
        .add_system_set(
            SystemSet::on_update(DruidState::Running).with_system(druid_system::<SomeData>),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut druid_fonts: ResMut<DruidFonts>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn_bundle(UiCameraBundle::default());

    // Load all fonts or tie in w/ env?
    //druid_fonts.0 = asset_server.load_folder(".").unwrap();
    druid_fonts
        .0
        .push(asset_server.load("Vollkorn-Regular.ttf"))
}

// Remove DruidFonts resource?
fn check_fonts(
    mut state: ResMut<State<DruidState>>,
    druid_fonts: Res<DruidFonts>,
    asset_server: Res<AssetServer>,
) {
    match asset_server.get_group_load_state(druid_fonts.0.iter().map(|handle| handle.id)) {
        LoadState::Failed => panic!("failed to load fonts"),
        LoadState::Loaded => state.set(DruidState::Running).unwrap(),
        _ => (), // loading
    }
}

/// Synchronize windows via events.
fn druid_window_system<T: Data + Resource + Root>(
    mut data: ResMut<T>,
    env: NonSend<Env>,
    mut windows: NonSendMut<DruidWindows<T>>,
    bevy_windows: Res<Windows>,
    mut window_created: EventReader<WindowCreated>,
    mut window_resized: EventReader<WindowResized>,
    mut _window_close_req: EventReader<WindowCloseRequested>,
    // `WinHandler::got_focus` is only used for AppState things and
    // `lost_focus` is not used at all?
    //mut window_focused: EventReader<WindowFocused>,
    mut cursor_moved: EventReader<CursorMoved>,
    // There is no `WinHandler::mouse_enter`?
    //mut cursor_entered: EventReader<CursorEntered>,
    mut cursor_left: EventReader<CursorLeft>,

    piet_params: druid::piet::PietParams,
) {
    // construct text only?
    let mut piet = druid::piet::Piet::new(piet_params, 0.0);

    let mut command_queue = VecDeque::new();

    for e in window_created.iter() {
        if let Some(_b) = bevy_windows.get(e.id) {
            //let size = druid::Size::new(b.physical_width() as f64, b.physical_height() as f64);
            let mut window = druid::Window::new(druid::WindowId::next(), T::root());
            //, &b.title());

            window.event(
                piet.text(),
                &mut command_queue,
                Event::WindowConnected,
                &mut *data,
                &*env,
            );

            windows.insert(e.id, window);
        }
    }

    for e in window_resized.iter() {
        if let Some(window) = windows.get_mut(&e.id) {
            // logical size
            let size = druid::Size::new(e.width as f64, e.height as f64);
            window.event(
                piet.text(),
                &mut command_queue,
                Event::WindowSize(size),
                &mut *data,
                &*env,
            );
        }
    }

    for e in cursor_left.iter() {
        if let Some(window) = windows.get_mut(&e.id) {
            window.event(
                piet.text(),
                &mut command_queue,
                Event::Internal(InternalEvent::MouseLeave),
                &mut *data,
                &*env,
            );
        }
    }

    for e in cursor_moved.iter() {
        if let Some(window) = windows.get_mut(&e.id) {
            let pos = Point::new(
                e.position.x as f64,
                // CursorMoved is in dp.
                window.size().height - e.position.y as f64,
            );
            window.event(
                piet.text(),
                &mut command_queue,
                Event::MouseMove(MouseEvent {
                    pos,
                    // Window remaps this?
                    window_pos: pos,
                    buttons: MouseButtons::new(), // TODO:
                    mods: Modifiers::empty(),     // TODO:
                    count: 0,
                    focus: false,
                    button: MouseButton::None,
                    wheel_delta: Vec2::ZERO,
                }),
                &mut *data,
                &*env,
            );
        }
    }
}

fn druid_system<T: Data + Resource + Root>(
    mut data: ResMut<T>,
    env: NonSend<Env>,
    mut windows: NonSendMut<DruidWindows<T>>,
    bevy_windows: Res<Windows>,
    piet_params: druid::piet::PietParams,
) {
    let mut piet = druid::piet::Piet::new(
        piet_params,
        bevy_windows.get_primary().unwrap().height() as f32,
    );

    let mut command_queue = VecDeque::new();

    // do_paint does this
    //piet.clear(None, druid::piet::Color::TRANSPARENT);

    // "paint"
    // this should crash when handle methods are called
    for (_id, window) in windows.iter_mut()
    //.filter_map(|(id, dw)| bevy_windows.get(*id).map(|w| (w, dw)))
    {
        window.prepare_paint(piet.text(), &mut command_queue, &mut *data, &*env);

        // AppState::do_update
        window.update(piet.text(), &mut command_queue, &*data, &*env);
        //window.invalidate_and_finalize();

        // We are not doing partial invalidation, so if anything has
        // changed, repaint the whole window.
        //if !window.handle.0.render().is_empty() {
        if !window.invalid().is_empty() || window.needs_layout() {
            window.invalid_mut().clear();
            let invalid: Region = window.size().to_rect().into();
            window.do_paint(&mut piet, &invalid, &mut command_queue, &*data, &*env);
            piet.finish().unwrap();
        }
    }
}
