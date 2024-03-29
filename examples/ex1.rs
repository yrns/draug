use bevy::asset::LoadState;
use bevy::ecs::system::{Resource, SystemParam};
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel};
use bevy::input::ButtonState;
use bevy::input::{mouse::MouseButton, Input};
use bevy::prelude::{
    warn, App, AssetServer, Camera2dBundle, Commands, DefaultPlugins, EventReader, Font, Handle,
    Local, NonSend, NonSendMut, Res, ResMut, State, SystemSet, Windows,
};
use bevy::time::Time;
use bevy::utils::HashMap;
use bevy::window::*;
use draug::{druid_key_code, scan_to_code};
use druid::keyboard_types::KeyState;
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
        let button = Button::new("Quit").on_click(|_ctx, _data, _env| {
            dbg!("got click");
            //ctx.submit_command(CLOSE_WINDOW);
        });
        let column = Scroll::new(Flex::column().with_child(label).with_child(button)).vertical();
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
        // ordering?
        .add_system(druid_window_system::<SomeData>)
        .add_system(druid_timer_system::<SomeData>)
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
    commands.spawn_bundle(Camera2dBundle::default());

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

// Going from winit -> Bevy -> Druid...
fn druid_mouse_button(b: &MouseButton) -> druid::MouseButton {
    match b {
        MouseButton::Left => druid::MouseButton::Left,
        MouseButton::Right => druid::MouseButton::Right,
        MouseButton::Middle => druid::MouseButton::Middle,
        MouseButton::Other(b) => {
            warn!("unhandled mouse button: {}", b);
            druid::MouseButton::None
        }
    }
}

fn druid_mouse_buttons(input: &Input<MouseButton>) -> druid::MouseButtons {
    // FromIterator?
    let mut set = druid::MouseButtons::new();
    for p in input.get_pressed() {
        set.insert(druid_mouse_button(p));
    }
    set
}

// TODO: Most functionality from WinHandler should be covered
// here. Also we need to pull stuff from WindowHandle.

// TODO: Handle mouse grab: https://bevy-cheatbook.github.io/window/mouse-grab.html

#[derive(SystemParam)]
struct InputParams<'w, 's> {
    cursor_moved: EventReader<'w, 's, CursorMoved>,
    // There is no `WinHandler::mouse_enter`?
    //mut cursor_entered: EventReader<CursorEntered>,
    cursor_left: EventReader<'w, 's, CursorLeft>,
    mouse_input: Res<'w, Input<MouseButton>>,
    mouse_button_input: EventReader<'w, 's, MouseButtonInput>,
    mouse_wheel: EventReader<'w, 's, MouseWheel>,
    key_input: EventReader<'w, 's, KeyboardInput>,
}

/// Synchronize windows via events.
fn druid_window_system<T: Data + Resource + Root>(
    mut focused: Local<Option<bevy::window::WindowId>>,
    mut cursor_position: Local<kurbo::Point>,
    piet_params: druid::piet::PietParams,
    mut data: ResMut<T>,
    env: NonSend<Env>,
    mut windows: NonSendMut<DruidWindows<T>>,
    bevy_windows: Res<Windows>,
    mut window_created: EventReader<WindowCreated>,
    mut window_resized: EventReader<WindowResized>,
    mut _window_close_req: EventReader<WindowCloseRequested>,
    // `WinHandler::got_focus` is only used for AppState things and
    // `lost_focus` is not used at all?
    mut window_focused: EventReader<WindowFocused>,
    input: InputParams,
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

    // Track focused window so we know where to send events. Maybe
    // this is already tracked somewhere in Bevy...
    for w in window_focused.iter() {
        match (*focused, w) {
            // Set new focus.
            (_, WindowFocused { id, focused: true }) => {
                *focused = Some(*id);
                break;
            }
            // Unset focus.
            (Some(cur), WindowFocused { id, focused: false }) if cur == *id => *focused = None,
            _ => (),
        }
    }

    // We went past the allowable number of parameters to a system function.
    let InputParams {
        mut cursor_moved,
        mut cursor_left,
        mouse_input,
        mut mouse_button_input,
        mut mouse_wheel,
        mut key_input,
    } = input;

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

    let druid_mouse_event = |pos| -> druid::MouseEvent {
        druid::MouseEvent {
            pos,
            window_pos: kurbo::Point::ZERO,
            buttons: druid_mouse_buttons(&*mouse_input),
            mods: Modifiers::empty(), // TODO:
            count: 0,
            focus: false,
            button: druid::MouseButton::None,
            wheel_delta: Vec2::ZERO,
        }
    };

    for e in cursor_moved.iter() {
        if let Some(window) = windows.get_mut(&e.id) {
            // Mouse events prior to the initial layout aren't very
            // useful (and just emit warnings).
            if window.needs_layout() {
                continue;
            }
            let pos = Point::new(
                e.position.x as f64,
                // CursorMoved is in dp.
                window.size().height - e.position.y as f64,
            );
            *cursor_position = pos;
            window.event(
                piet.text(),
                &mut command_queue,
                Event::MouseMove(druid_mouse_event(pos)),
                &mut *data,
                &*env,
            );
        }
    }

    // Bevy input handling isn't per-window, so we check the local
    // `focused` handle.
    if let Some(window) = focused.and_then(|id| windows.get_mut(&id)) {
        // mouse_down/mouse_up
        for e in mouse_button_input.iter() {
            let MouseButtonInput { button, state } = e;
            let mut druid_event = druid_mouse_event(*cursor_position);
            druid_event.button = druid_mouse_button(button);
            let event = match state {
                ButtonState::Pressed => Event::MouseDown(druid_event),
                ButtonState::Released => Event::MouseUp(druid_event),
            };
            window.event(piet.text(), &mut command_queue, event, &mut *data, &*env);
        }

        // wheel
        for e in mouse_wheel.iter() {
            let mut druid_event = druid_mouse_event(*cursor_position);
            druid_event.wheel_delta = match e.unit {
                MouseScrollUnit::Line => druid::Vec2::new(e.x as f64, e.y as f64),
                // TODO: differently?
                MouseScrollUnit::Pixel => druid::Vec2::new(e.x as f64, e.y as f64),
            };
            window.event(
                piet.text(),
                &mut command_queue,
                Event::Wheel(druid_event),
                &mut *data,
                &*env,
            );
        }

        // WinHandler::zoom's only origin in Druid is on Mac with a
        // pinch event. TODO: Bevy touch input?

        // key_down/key_up
        for e in key_input.iter() {
            if let KeyboardInput {
                scan_code,
                key_code: Some(key_code),
                state,
            } = e
            {
                dbg!(key_code);
                let mut druid_event = druid::KeyEvent::default();
                druid_event.state = match state {
                    ButtonState::Pressed => KeyState::Down,
                    ButtonState::Released => KeyState::Up,
                };
                // druid_event.mods = ???

                // u32 to druid::KbKey?
                druid_event.code = scan_to_code(*scan_code);
                druid_event.key = druid_key_code(key_code, false);

                println!("{:?}", druid_event);
            }
        }
    }
}

fn druid_timer_system<T: Data + Resource + Root>(
    mut data: ResMut<T>,
    env: NonSend<Env>,
    piet: druid::piet::PietParams,
    time: Res<Time>,
    mut windows: NonSendMut<DruidWindows<T>>,
) {
    let mut text = piet.text();

    let mut command_queue = VecDeque::new();

    let mut finished = Vec::new();

    for window in windows.values_mut() {
        // Move to WindowHandle?
        {
            let mut timers = window.handle.0.timers.borrow_mut();

            // retain_mut is nightly, so we loop twice:
            timers.iter_mut().for_each(|(_, t)| {
                t.tick(time.delta());
            });

            timers.retain(|(token, timer)| {
                if timer.finished() {
                    finished.push(*token);
                    false
                } else {
                    true
                }
            });
        }

        for token in finished.iter() {
            window.event(
                &mut text,
                &mut command_queue,
                Event::Timer(*token),
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
