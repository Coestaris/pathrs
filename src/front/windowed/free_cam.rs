use crate::config::Camera;
use glam::{FloatExt, Vec2, Vec3};
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::keyboard::{Key, NamedKey};

#[derive(Clone, Copy, Debug)]
pub struct CameraData {
    pub(crate) position: Vec3,
    pitch: f32,
    yaw: f32,
}

impl CameraData {
    /// The smallest angle that can be considered "zero"
    const EPS: f32 = 0.0001;

    pub fn new(initial: Camera) -> Self {
        let direction = initial.direction.normalize();
        let right = direction.cross(Vec3::Y).normalize();
        let up = direction.cross(right).normalize();
        let pitch = up.y.acos();
        let yaw = right.x.atan2(right.z);
        Self {
            position: initial.position,
            pitch,
            yaw,
        }
    }

    pub fn as_direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            position: self.position.lerp(other.position, t),
            pitch: self.pitch.lerp(other.pitch, t),
            yaw: self.yaw.lerp(other.yaw, t),
        }
    }
}

impl PartialEq<Self> for CameraData {
    fn eq(&self, other: &Self) -> bool {
        (self.position - other.position).length() < Self::EPS
            && (self.pitch - other.pitch).abs() < Self::EPS
            && (self.yaw - other.yaw).abs() < Self::EPS
    }
}

struct InputState {
    mouse_pos: Vec2,
    mouse_button_pressed: bool,

    forward_pressed: bool,
    back_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    up_pressed: bool,
    down_pressed: bool,
}

pub struct FreeCamera {
    click_pos: Vec2,
    data: CameraData,
    instant: CameraData,
    input_state: InputState,
}

impl FreeCamera {
    pub fn new(initial: Camera) -> Self {
        Self {
            click_pos: Vec2::ZERO,
            data: CameraData::new(initial.clone()),
            instant: CameraData::new(initial),
            input_state: InputState {
                mouse_pos: Default::default(),
                mouse_button_pressed: false,
                forward_pressed: false,
                back_pressed: false,
                left_pressed: false,
                right_pressed: false,
                up_pressed: false,
                down_pressed: false,
            },
        }
    }

    pub fn on_window_event(&mut self, event: &WindowEvent) {
        match &event {
            WindowEvent::CursorMoved { position, .. } => {
                self.input_state.mouse_pos = Vec2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                self.click_pos = self.input_state.mouse_pos;
                self.input_state.mouse_button_pressed = true;
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Right,
                ..
            } => {
                self.input_state.mouse_button_pressed = false;
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    logical_key, state, ..
                },
                ..
            } => match (logical_key, state) {
                (Key::Character(s), state) if s == "w" => {
                    self.input_state.forward_pressed = *state == ElementState::Pressed;
                }
                (Key::Character(s), state) if s == "s" => {
                    self.input_state.back_pressed = *state == ElementState::Pressed;
                }
                (Key::Character(s), state) if s == "a" => {
                    self.input_state.left_pressed = *state == ElementState::Pressed;
                }
                (Key::Character(s), state) if s == "d" => {
                    self.input_state.right_pressed = *state == ElementState::Pressed;
                }
                (Key::Named(NamedKey::Space), state) => {
                    self.input_state.up_pressed = *state == ElementState::Pressed;
                }
                (Key::Named(NamedKey::Shift), state) => {
                    self.input_state.down_pressed = *state == ElementState::Pressed;
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn tick_handler(&mut self) -> Option<CameraData> {
        const MOVE_SPEED: f32 = 3.0;
        const ROTATE_SPEED: f32 = 0.001;
        const LERP: f32 = 0.00001;

        let delta = 1.0 / 60.0; // Assume a fixed timestep for simplicity
        let direction = self.instant.as_direction();
        let right = direction.cross(Vec3::Y).normalize();
        let up = direction.cross(right).normalize();

        if self.input_state.forward_pressed {
            self.instant.position += direction * delta * MOVE_SPEED;
        }
        if self.input_state.back_pressed {
            self.instant.position += direction * -delta * MOVE_SPEED;
        }
        if self.input_state.left_pressed {
            self.instant.position += right * -delta * MOVE_SPEED;
        }
        if self.input_state.right_pressed {
            self.instant.position += right * delta * MOVE_SPEED;
        }
        if self.input_state.up_pressed {
            self.instant.position += up * -delta * MOVE_SPEED;
        }
        if self.input_state.down_pressed {
            self.instant.position += up * delta * MOVE_SPEED;
        }
        if self.input_state.mouse_button_pressed {
            let pos_delta = self.input_state.mouse_pos - self.click_pos;
            self.click_pos = self.input_state.mouse_pos;

            // Allow look around in all directions
            self.instant.yaw = self.instant.yaw - pos_delta.x * ROTATE_SPEED;

            // Clamp pitch to prevent gimbal lock
            self.instant.pitch = self.instant.pitch - pos_delta.y * ROTATE_SPEED;
        }

        // Smoothly interpolate position and rotation
        let factor = 1.0 - LERP.powf(delta);
        let data = self.data.lerp(&self.instant, factor.clamp(0.0, 1.0));
        if self.data != data {
            self.data = data;
            Some(data)
        } else {
            None
        }
    }
}
